use chrono::Utc;
use log::{error, warn};
use rocket::http::Status;
use rocket::response::stream::{Event, EventStream};
use rocket::{get, post, serde::json::Json, State};
use serde::{Deserialize, Serialize};
use std::time;
use tokio::sync::broadcast::{self, error::TryRecvError};
use uuid::Uuid;
use vickylib::database::entities::lock::db_impl::LockDatabase;
use vickylib::database::entities::task::db_impl::TaskDatabase;
use vickylib::database::entities::task::{FlakeRef, TaskResult, TaskStatus};
use vickylib::database::entities::{Database, Lock, Task};
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

#[get("/?<status>")]
pub async fn tasks_get_user(
    db: Database,
    _user: User,
    status: Option<String>,
) -> Result<Json<Vec<Task>>, AppError> {
    let task_status: Option<TaskStatus> = status
        .as_deref()
        .map(TaskStatus::try_from)
        .transpose()
        .map_err(|_| AppError::HttpError(Status::BadRequest))?;
    let tasks: Vec<Task> = db
        .run(|conn| conn.get_all_tasks_filtered(task_status))
        .await?;
    Ok(Json(tasks))
}

#[get("/?<status>", rank = 2)]
pub async fn tasks_get_machine(
    db: Database,
    _machine: Machine,
    status: Option<String>,
) -> Result<Json<Vec<Task>>, AppError> {
    let task_status: Option<TaskStatus> = status
        .as_deref()
        .map(TaskStatus::try_from)
        .transpose()
        .map_err(|_| AppError::HttpError(Status::BadRequest))?;
    let tasks: Vec<Task> = db
        .run(|conn| conn.get_all_tasks_filtered(task_status))
        .await?;
    Ok(Json(tasks))
}

async fn tasks_specific_get(id: Uuid, db: &Database) -> Result<Json<Option<Task>>, AppError> {
    let tasks: Option<Task> = db.run(move |conn| conn.get_task(id)).await?;
    Ok(Json(tasks))
}

#[get("/<id>")]
pub async fn tasks_specific_get_user(
    id: Uuid,
    db: Database,
    _user: User,
) -> Result<Json<Option<Task>>, AppError> {
    tasks_specific_get(id, &db).await
}

#[get("/<id>", rank = 2)]
pub async fn tasks_specific_get_machine(
    id: Uuid,
    db: Database,
    _machine: Machine,
) -> Result<Json<Option<Task>>, AppError> {
    tasks_specific_get(id, &db).await
}

#[get("/<id>/logs?<start>")]
pub async fn tasks_get_logs<'a>(
    id: Uuid,
    db: Database,
    s3: &'a State<S3Client>,
    _user: User,
    log_drain: &'a State<&'_ LogDrain>,
    start: Option<i32>,
) -> EventStream![Event + 'a] {
    let setup = match db.run(move |conn| conn.get_task(id)).await {
        Ok(Some(task)) => Some((id, task)),
        Ok(None) => {
            warn!("task {id} not found");
            None
        }
        Err(err) => {
            error!("task lookup failed {id}: {err}");
            None
        }
    };

    // Note: The user might specify a start parameter and we want to skip every line until this start param is reached.
    let mut skip_lines = start.unwrap_or(0);

    EventStream! {
        if let Some((task_uuid, task)) = setup {
        match task.status {
            TaskStatus::New => {},
            TaskStatus::Running => {
                let mut recv = log_drain.send_handle.subscribe();
                let existing_log_messages = log_drain
                    .get_logs(task_uuid)
                    .await
                    .unwrap_or_default();

                for element in existing_log_messages {
                    if skip_lines <= 0 {
                        yield Event::data(element)
                    }
                    skip_lines -= 1;
                }

                loop {
                    let read_val = recv.try_recv();

                    match read_val {
                        Ok((task_id, log_text)) => {
                            if task_id == id {
                                if skip_lines <= 0 {
                                    yield Event::data(log_text)
                                }
                                skip_lines -= 1;
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
                match s3.get_logs(&id.to_string()).await {
                    Ok(logs) => {
                        for element in logs {
                            if skip_lines <= 0 {
                                yield Event::data(element)
                            }
                            skip_lines -= 1;
                        }
                        loop {
                            tokio::time::sleep(time::Duration::from_millis(100)).await;
                        }
                    }
                    Err(err) => {
                        log::error!("failed to load logs for {id}: {err}");
                    }
                }
            },
        }
        }
    }
}

#[get("/<id>/logs/download")]
pub async fn tasks_download_logs(
    id: Uuid,
    db: Database,
    s3: &'_ State<S3Client>,
    _machine: Machine,
) -> Result<Json<LogLines>, AppError> {
    // Note: We still need to verify the existence of the task before accessing S3 with an abitrary string..
    let _task = db
        .run(move |conn| conn.get_task(id))
        .await
        .map_err(AppError::from)?
        .ok_or(AppError::HttpError(Status::NotFound))?;

    let logs = s3.get_logs(&id.to_string()).await.map_err(AppError::from)?;
    let log_lines = LogLines { lines: logs };

    Ok(Json(log_lines))
}

#[post("/<id>/logs", format = "json", data = "<logs>")]
pub async fn tasks_put_logs(
    id: Uuid,
    db: Database,
    logs: Json<LogLines>,
    _machine: Machine,
    log_drain: &State<&LogDrain>,
) -> Result<Json<()>, AppError> {
    let task = db
        .run(move |conn| conn.get_task(id))
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
            task.claimed_at = Some(Utc::now().naive_utc());

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
    id: Uuid,
    finish: Json<RoTaskFinish>,
    db: Database,
    global_events: &State<broadcast::Sender<GlobalEvent>>,
    _machine: Machine,
    log_drain: &State<&LogDrain>,
) -> Result<Json<Task>, AppError> {
    let mut task = db
        .run(move |conn| conn.get_task(id))
        .await?
        .ok_or(AppError::HttpError(Status::NotFound))?;

    log_drain.finish_logs(id).await?;

    task.status = TaskStatus::Finished(finish.result.clone());
    task.finished_at = Some(Utc::now().naive_utc());

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
