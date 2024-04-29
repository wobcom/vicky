use diesel::PgConnection;
use rocket::http::Status;
use rocket::response::stream::{Event, EventStream};
use rocket::{get, post, serde::json::Json, State};
use serde::{Deserialize, Serialize};
use std::time;
use tokio::sync::broadcast::{self, error::TryRecvError};
use uuid::Uuid;
use vickylib::database::entities::db_impl::TaskDatabase;
use vickylib::database::entities::{FlakeRef, Lock, Task, TaskResult, TaskStatus};
use vickylib::{
    errors::VickyError, logs::LogDrain, s3::client::S3Client, vicky::scheduler::Scheduler,
};

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
pub async fn tasks_get_user(
    db: &mut State<PgConnection>,
    _user: User,
) -> Result<Json<Vec<Task>>, VickyError> {
    let tasks: Vec<Task> = db.get_all_tasks().await?;
    Ok(Json(tasks))
}

#[get("/", rank = 2)]
pub async fn tasks_get_machine(
    db: &mut State<PgConnection>,
    _machine: Machine,
) -> Result<Json<Vec<Task>>, VickyError> {
    let tasks: Vec<Task> = db.get_all_tasks().await?;
    Ok(Json(tasks))
}

#[get("/<id>")]
pub async fn tasks_specific_get_user(
    id: String,
    db: &mut State<PgConnection>,
    _user: User,
) -> Result<Json<Option<Task>>, VickyError> {
    let task_uuid = Uuid::parse_str(&id).unwrap();
    let tasks: Option<Task> = db.get_task(task_uuid).await?;
    Ok(Json(tasks))
}

#[get("/<id>", rank = 2)]
pub async fn tasks_specific_get_machine(
    id: String,
    db: &mut State<PgConnection>,
    _machine: Machine,
) -> Result<Json<Option<Task>>, VickyError> {
    let task_uuid = Uuid::parse_str(&id).unwrap();
    let tasks: Option<Task> = db.get_task(task_uuid).await?;
    Ok(Json(tasks))
}

#[get("/<id>/logs")]
pub async fn tasks_get_logs<'a>(
    id: String,
    db: &mut State<PgConnection>,
    s3: &'a State<S3Client>,
    _user: User,
    log_drain: &'a State<&'_ LogDrain>,
) -> EventStream![Event + 'a] {
    // TODO: Fix Error Handling
    let task_uuid = Uuid::parse_str(&id).unwrap();
    let task = db
        .get_task(task_uuid)
        .await
        .unwrap()
        .ok_or(AppError::HttpError(Status::NotFound))
        .unwrap();

    EventStream! {

        match task.status {
            TaskStatus::NEW => {},
            TaskStatus::RUNNING => {
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
            TaskStatus::FINISHED(_) => {
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

#[post("/<id>/logs", format = "json", data = "<logs>")]
pub async fn tasks_put_logs(
    id: String,
    db: &mut State<PgConnection>,
    logs: Json<LogLines>,
    _machine: Machine,
    log_drain: &State<&LogDrain>,
) -> Result<Json<()>, AppError> {
    let task_uuid = Uuid::parse_str(&id)?;
    let task = db
        .get_task(task_uuid)
        .await?
        .ok_or(AppError::HttpError(Status::NotFound))?;

    match task.status {
        TaskStatus::RUNNING => {
            log_drain.push_logs(id, logs.lines.clone())?;
            Ok(Json(()))
        }
        _ => Err(AppError::HttpError(Status::Locked))?,
    }
}

#[post("/claim", format = "json", data = "<features>")]
pub async fn tasks_claim(
    db: &mut State<PgConnection>,
    features: Json<RoTaskClaim>,
    global_events: &State<broadcast::Sender<GlobalEvent>>,
    _machine: Machine,
) -> Result<Json<Option<Task>>, AppError> {
    let tasks = db.get_all_tasks().await?;
    let scheduler = Scheduler::new(tasks, &features.features)
        .map_err(|x| VickyError::Scheduler { source: x })?;
    let next_task = scheduler.get_next_task();

    match next_task {
        Some(next_task) => {
            let mut task = db
                .get_task(next_task.id)
                .await?
                .ok_or(AppError::HttpError(Status::NotFound))?;
            task.status = TaskStatus::RUNNING;
            db.put_task(&task).await?;
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
    db: &mut State<PgConnection>,
    global_events: &State<broadcast::Sender<GlobalEvent>>,
    _machine: Machine,
    log_drain: &State<&LogDrain>,
) -> Result<Json<Task>, AppError> {
    let task_uuid = Uuid::parse_str(&id)?;
    let mut task = db
        .get_task(task_uuid)
        .await?
        .ok_or(AppError::HttpError(Status::NotFound))?;

    log_drain.finish_logs(&id).await?;

    task.status = TaskStatus::FINISHED(finish.result.clone());
    db.put_task(&task).await?;
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
    db: &mut State<PgConnection>,
    global_events: &State<broadcast::Sender<GlobalEvent>>,
    _machine: Machine,
) -> Result<Json<RoTask>, AppError> {
    let task_uuid = Uuid::new_v4();

    let task_manifest = Task::builder()
        .with_id(task_uuid)
        .with_display_name(&task.display_name)
        .with_flake(&task.flake_ref.flake)
        .with_flake_args(task.flake_ref.args.clone())
        .with_locks(task.locks.clone())
        .requires_features(task.features.clone())
        .build();

    if check_lock_conflict(&task_manifest) {
        return Err(AppError::HttpError(Status::Conflict));
    }

    db.put_task(&task_manifest).await?;
    global_events.send(GlobalEvent::TaskAdd)?;

    let ro_task = RoTask {
        id: task_uuid,
        status: TaskStatus::NEW,
    };

    Ok(Json(ro_task))
}

#[cfg(test)]
mod tests {
    use crate::tasks::check_lock_conflict;
    use uuid::Uuid;
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
