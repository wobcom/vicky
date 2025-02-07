use rocket::http::Status;
use rocket::response::stream::{Event, EventStream};
use rocket::{get, post, serde::json::Json, State};
use serde::{Deserialize, Serialize};
use std::time;
use tokio::sync::broadcast::{self, error::TryRecvError};
use uuid::Uuid;
use vickylib::database::entities::task::db_impl::TaskDatabase;
use vickylib::database::entities::task::{FlakeRef, TaskResult, TaskStatus};
use vickylib::database::entities::{Database, Lock, Task};
use vickylib::{
    errors::VickyError, logs::LogDrain, s3::client::S3Client, vicky::scheduler::Scheduler,
};
use vickylib::database::entities::lock::db_impl::LockDatabase;

use crate::{
    auth::{Machine, User},
    errors::AppError,
    events::GlobalEvent,
};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RoTaskNew {
    display_name: String,
    flake_ref: FlakeRef,
    locks: Vec<Lock>,
    features: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RoTaskClaim {
    features: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RoTask {
    id: Uuid,
    status: TaskStatus,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RoTaskFinish {
    result: TaskResult,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct LogLines {
    lines: Vec<String>,
}

#[get("/")]
pub async fn tasks_get_user(db: Database, _user: User) -> Result<Json<Vec<Task>>, VickyError> {
    let tasks: Vec<Task> = db.run(|conn| conn.get_all_tasks()).await?;
    Ok(Json(tasks))
}

#[get("/", rank = 2)]
pub async fn tasks_get_machine(
    db: Database,
    _machine: Machine,
) -> Result<Json<Vec<Task>>, VickyError> {
    let tasks: Vec<Task> = db.run(|conn| conn.get_all_tasks()).await?;
    Ok(Json(tasks))
}

async fn tasks_specific_get(id: &str, db: &Database) -> Result<Json<Option<Task>>, VickyError> {
    let task_uuid = Uuid::parse_str(id).unwrap();
    let tasks: Option<Task> = db.run(move |conn| conn.get_task(task_uuid)).await?;
    Ok(Json(tasks))
}

#[get("/<id>")]
pub async fn tasks_specific_get_user(
    id: String,
    db: Database,
    _user: User,
) -> Result<Json<Option<Task>>, VickyError> {
    tasks_specific_get(&id, &db).await
}

#[get("/<id>", rank = 2)]
pub async fn tasks_specific_get_machine(
    id: String,
    db: Database,
    _machine: Machine,
) -> Result<Json<Option<Task>>, VickyError> {
    tasks_specific_get(&id, &db).await
}

#[get("/<id>/logs")]
pub async fn tasks_get_logs<'a>(
    id: String,
    db: Database,
    s3: &'a State<S3Client>,
    _user: User,
    log_drain: &'a State<&'_ LogDrain>,
) -> EventStream![Event + 'a] {
    // TODO: Fix Error Handling
    let task_uuid = Uuid::parse_str(&id).unwrap();
    let task = db
        .run(move |conn| conn.get_task(task_uuid))
        .await
        .unwrap()
        .ok_or(AppError::HttpError(Status::NotFound))
        .unwrap();

    EventStream! {

        match task.status {
            TaskStatus::New => {},
            TaskStatus::Running => {
                let mut recv = log_drain.send_handle.subscribe();
                let existing_log_messages = log_drain.get_logs(task_uuid.to_string()).await.unwrap();

                for element in existing_log_messages {
                    yield Event::data(element)
                }

                loop {
                    let read_val = recv.try_recv();

                    match read_val {
                        Ok((task_id, log_text)) => {
                            if task_id == id {
                                yield Event::data(log_text)
                            }
                        },
                        Err(TryRecvError::Closed) => {
                            break;
                        },
                        Err(TryRecvError::Lagged(_)) => {
                            // Immediate Retry, doing our best effort here.
                        },
                        Err(TryRecvError::Empty) => {
                            tokio::time::sleep(time::Duration::from_millis(100)).await;
                        },
                    }
                }
            },
            TaskStatus::Finished(_) => {
                let logs = s3.get_logs(&id).await.unwrap();
                for element in logs {
                    yield Event::data(element)
                }
                loop {
                    tokio::time::sleep(time::Duration::from_millis(100)).await;
                }
            },
        }

    }
}


#[get("/<id>/logs/download")]
pub async fn tasks_download_logs(
    id: String,
    db: Database,
    s3: &'_ State<S3Client>,
    _machine: Machine,
) -> Result<Json<LogLines>, VickyError> {
    // TODO: Fix Error Handling
    let task_uuid = Uuid::parse_str(&id).unwrap();
    // Note: We still need to verify the existance of the task before accessing S3 with an abitrary string..
    let _task = db
        .run(move |conn| conn.get_task(task_uuid))
        .await
        .unwrap()
        .ok_or(AppError::HttpError(Status::NotFound))
        .unwrap();

    let logs = s3.get_logs(&id).await.unwrap();
    let log_lines = LogLines {
        lines: logs,
    };
        
    Ok(Json(log_lines))
}

#[post("/<id>/logs", format = "json", data = "<logs>")]
pub async fn tasks_put_logs(
    id: String,
    db: Database,
    logs: Json<LogLines>,
    _machine: Machine,
    log_drain: &State<&LogDrain>,
) -> Result<Json<()>, AppError> {
    let task_uuid = Uuid::parse_str(&id)?;
    let task = db
        .run(move |conn| conn.get_task(task_uuid))
        .await?
        .ok_or(AppError::HttpError(Status::NotFound))?;

    match task.status {
        TaskStatus::Running => {
            log_drain.push_logs(id, logs.lines.clone())?;
            Ok(Json(()))
        }
        _ => Err(AppError::HttpError(Status::Locked))?,
    }
}

#[post("/claim", format = "json", data = "<features>")]
pub async fn tasks_claim(
    db: Database,
    features: Json<RoTaskClaim>,
    global_events: &State<broadcast::Sender<GlobalEvent>>,
    _machine: Machine,
) -> Result<Json<Option<Task>>, AppError> {
    let tasks = db.run(|conn| conn.get_all_tasks()).await?;
    let poisoned_locks = db.run(|conn| conn.get_poisoned_locks()).await?;
    let scheduler = Scheduler::new(&tasks, &poisoned_locks, &features.features)
        .map_err(|x| VickyError::Scheduler { source: x })?;
    let next_task = scheduler.get_next_task();

    match next_task {
        Some(next_task) => {
            let mut task = db
                .run(move |conn| conn.get_task(next_task.id))
                .await?
                .ok_or(AppError::HttpError(Status::NotFound))?;
            task.status = TaskStatus::Running;
            let task2 = task.clone();
            db.run(move |conn| conn.update_task(&task2)).await?;
            global_events.send(GlobalEvent::TaskUpdate { uuid: task.id })?;
            Ok(Json(Some(task)))
        }
        None => Ok(Json(None)),
    }
}

#[post("/<id>/finish", format = "json", data = "<finish>")]
pub async fn tasks_finish(
    id: String,
    finish: Json<RoTaskFinish>,
    db: Database,
    global_events: &State<broadcast::Sender<GlobalEvent>>,
    _machine: Machine,
    log_drain: &State<&LogDrain>,
) -> Result<Json<Task>, AppError> {
    let task_uuid = Uuid::parse_str(&id)?;
    let mut task = db
        .run(move |conn| conn.get_task(task_uuid))
        .await?
        .ok_or(AppError::HttpError(Status::NotFound))?;

    log_drain.finish_logs(&id).await?;

    task.status = TaskStatus::Finished(finish.result.clone());
    
    if finish.result == TaskResult::Error {
        task.locks.iter_mut().for_each(|lock| lock.poison(&task.id));
    }
    
    // TODO: this clone is weird and can be saved I think
    let task2 = task.clone();
    db.run(move |conn| conn.update_task(&task2)).await?;
    global_events.send(GlobalEvent::TaskUpdate { uuid: task.id })?;

    Ok(Json(task))
}

// TODO: Move into Task Builder
fn check_lock_conflict(task: &Task) -> bool {
    task.locks.iter().enumerate().any(|(i, lock)| {
        task.locks
            .iter()
            .enumerate()
            .any(|(j, lock2)| i < j && lock.is_conflicting(lock2))
    })
}

#[post("/", data = "<task>")]
pub async fn tasks_add(
    task: Json<RoTaskNew>,
    db: Database,
    global_events: &State<broadcast::Sender<GlobalEvent>>,
    _machine: Machine,
) -> Result<Json<RoTask>, AppError> {
    let task_uuid = Uuid::new_v4();

    let task = Task::builder()
        .with_id(task_uuid)
        .with_display_name(&task.display_name)
        .with_flake(&task.flake_ref.flake)
        .with_flake_args(task.flake_ref.args.clone())
        .with_locks(task.locks.clone())
        .requires_features(task.features.clone())
        .build();

    if check_lock_conflict(&task) {
        return Err(AppError::HttpError(Status::Conflict));
    }

    db.run(move |conn| conn.put_task(task)).await?;
    global_events.send(GlobalEvent::TaskAdd)?;

    let ro_task = RoTask {
        id: task_uuid,
        status: TaskStatus::New,
    };

    Ok(Json(ro_task))
}

#[cfg(test)]
mod tests {
    use crate::tasks::check_lock_conflict;
    use vickylib::database::entities::Task;

    #[test]
    fn add_new_conflicting_task() {
        let task = Task::builder()
            .with_display_name("Test 1")
            .with_read_lock("mauz")
            .with_write_lock("mauz")
            .build();
        assert!(check_lock_conflict(&task))
    }

    #[test]
    fn add_new_not_conflicting_task() {
        let task = Task::builder()
            .with_display_name("Test 1")
            .with_read_lock("mauz")
            .with_read_lock("mauz")
            .with_write_lock("delete_everything")
            .build();
        assert!(!check_lock_conflict(&task))
    }
}
