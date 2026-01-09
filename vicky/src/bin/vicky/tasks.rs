use chrono::{TimeDelta, Utc};
use log::{error, warn};
use rocket::http::Status;
use rocket::response::stream::{Event, EventStream};
use rocket::{State, get, post, serde::json::Json};
use serde::{Deserialize, Serialize};
use std::time;
use tokio::sync::broadcast::{self, error::TryRecvError};
use uuid::Uuid;
use vickylib::database::entities::task::HEARTBEAT_TIMEOUT_SEC;
use vickylib::database::entities::task::{FlakeRef, TaskResult, TaskStatus};
use vickylib::database::entities::{Database, Lock, Task};
use vickylib::query::FilterParams;
use vickylib::{
    errors::VickyError, logs::LogDrain, s3::client::S3Client, vicky::scheduler::Scheduler,
};

macro_rules! task_or {
    ($db:expr, $id:expr => $( $or:tt )+) => {
        $db
            .get_task($id)
            .await?
            $( $or )+
    };
}

macro_rules! task_or_not_found {
    ($db:expr, $id:expr) => {
        task_or!($db, $id => .ok_or(AppError::HttpError(Status::NotFound)))
    };
}

use crate::auth::AnyAuthGuard;
use crate::{
    auth::{MachineGuard, UserGuard},
    errors::AppError,
    events::GlobalEvent,
};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RoTaskNew {
    #[serde(default)] // This will be false, if not given.
    needs_confirmation: bool,
    display_name: String,
    flake_ref: FlakeRef,
    locks: Vec<Lock>,
    features: Vec<String>,
    group: Option<String>,
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Count {
    count: i64,
}

#[get("/count?<status>&<filter_params..>")]
pub async fn tasks_count(
    db: Database,
    _auth: AnyAuthGuard,
    status: Option<String>,
    filter_params: Option<FilterParams>,
) -> Result<Json<Count>, AppError> {
    let task_status: Option<TaskStatus> = status
        .as_deref()
        .map(TaskStatus::try_from)
        .transpose()
        .map_err(|_| AppError::HttpError(Status::BadRequest))?;
    let tasks_count = db.count_all_tasks(task_status, filter_params).await?;
    let c: Count = Count { count: tasks_count };
    Ok(Json(c))
}

#[get("/?<status>&<filter_params..>")]
pub async fn tasks_get(
    db: Database,
    _auth: AnyAuthGuard,
    status: Option<String>,
    filter_params: Option<FilterParams>,
) -> Result<Json<Vec<Task>>, AppError> {
    let task_status: Option<TaskStatus> = status
        .as_deref()
        .map(TaskStatus::try_from)
        .transpose()
        .map_err(|_| AppError::HttpError(Status::BadRequest))?;
    let tasks: Vec<Task> = db
        .get_all_tasks_filtered(task_status, filter_params)
        .await?;
    Ok(Json(tasks))
}

#[get("/<id>")]
pub async fn tasks_get_specific(
    id: Uuid,
    db: Database,
    _auth: AnyAuthGuard,
) -> Result<Json<Option<Task>>, AppError> {
    let tasks: Option<Task> = db.get_task(id).await?;
    Ok(Json(tasks))
}

#[get("/<id>/logs?<start>")]
pub async fn tasks_get_logs<'a>(
    id: Uuid,
    db: Database,
    s3: &'a State<S3Client>,
    _user: UserGuard,
    log_drain: &'a State<LogDrain>,
    start: Option<i32>,
) -> EventStream![Event + 'a] {
    let setup = match db.get_task(id).await {
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
            TaskStatus::NeedsUserValidation | TaskStatus::New => {},
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
                        error!("failed to load logs for {id}: {err}");
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
    _machine: MachineGuard,
) -> Result<Json<LogLines>, AppError> {
    let exists = db.has_task(id).await?;

    if !exists {
        return Err(AppError::HttpError(Status::NotFound));
    }

    let logs = s3.get_logs(&id.to_string()).await?;
    let log_lines = LogLines { lines: logs };

    Ok(Json(log_lines))
}

#[post("/<id>/logs", format = "json", data = "<logs>")]
pub async fn tasks_put_logs(
    id: Uuid,
    db: Database,
    logs: Json<LogLines>,
    _machine: MachineGuard,
    log_drain: &State<LogDrain>,
) -> Result<Json<()>, AppError> {
    let task: Task = task_or_not_found!(db, id)?;

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
    _machine: MachineGuard,
) -> Result<Json<Option<Task>>, AppError> {
    let tasks = db.get_all_tasks().await?;
    let poisoned_locks = db.get_poisoned_locks().await?;
    let scheduler = Scheduler::new(&tasks, &poisoned_locks, &features.features)
        .map_err(|x| VickyError::Scheduler { source: x })?;
    let next_task = scheduler.get_next_task();

    match next_task {
        Some(next_task) => {
            let mut task: Task = task_or_not_found!(db, next_task.id)?;
            task.status = TaskStatus::Running;
            task.claimed_at = Some(Utc::now());
            task.last_heartbeat = task.claimed_at;

            db.update_task(task.clone()).await?;
            global_events.send(GlobalEvent::TaskUpdate { uuid: task.id })?;
            Ok(Json(Some(task)))
        }
        None => Ok(Json(None)),
    }
}

#[post("/<id>/heartbeat")]
pub async fn tasks_heartbeat(id: Uuid, db: Database, _auth: AnyAuthGuard) -> Result<(), AppError> {
    let task: Task = task_or_not_found!(db, id)?;

    if task.status != TaskStatus::Running {
        return Err(AppError::HttpError(Status::Conflict));
    }

    if db.register_task_heartbeat(id).await? == 0 {
        return Err(AppError::HttpError(Status::Conflict));
    }

    // TODO: Consider sending a global event here

    Ok(())
}

#[post("/<id>/finish", format = "json", data = "<finish>")]
pub async fn tasks_finish(
    id: Uuid,
    finish: Json<RoTaskFinish>,
    db: Database,
    global_events: &State<broadcast::Sender<GlobalEvent>>,
    _machine: MachineGuard,
    log_drain: &State<LogDrain>,
) -> Result<Json<Task>, AppError> {
    let mut task: Task = task_or_not_found!(db, id)?;

    task.finish(finish.result);

    db.update_task(task.clone()).await?;

    let log_error = log_drain.finish_logs(id).await;

    global_events.send(GlobalEvent::TaskUpdate { uuid: task.id })?;

    // only handle log error here so that the UI gets the event at the right time
    log_error?;

    Ok(Json(task))
}

#[post("/", data = "<task>")]
pub async fn tasks_add(
    task: Json<RoTaskNew>,
    db: Database,
    global_events: &State<broadcast::Sender<GlobalEvent>>,
    _machine: MachineGuard,
) -> Result<Json<RoTask>, AppError> {
    let status = if task.needs_confirmation {
        TaskStatus::NeedsUserValidation
    } else {
        TaskStatus::New
    };

    let task = task.into_inner();

    let task = Task::builder()
        .status(status)
        .display_name(task.display_name)
        .flake(task.flake_ref.flake)
        .flake_args(task.flake_ref.args)
        .locks(task.locks)
        .requires_features(task.features)
        .maybe_group(task.group)
        .build();

    let Ok(task) = task else {
        return Err(AppError::HttpError(Status::Conflict));
    };

    let ro_task = RoTask {
        id: task.id,
        status,
    };

    db.put_task(task).await?;
    global_events.send(GlobalEvent::TaskAdd)?;

    Ok(Json(ro_task))
}

#[post("/<id>/confirm")]
pub async fn tasks_confirm(
    id: Uuid,
    db: Database,
    global_events: &State<broadcast::Sender<GlobalEvent>>,
    _auth: AnyAuthGuard,
) -> Result<Json<Task>, AppError> {
    let mut task = task_or_not_found!(db, id)?;

    if task.status == TaskStatus::New {
        return Err(AppError::TaskAlreadyConfirmed);
    } else if task.status != TaskStatus::NeedsUserValidation {
        return Err(AppError::HttpError(Status::Conflict));
    }

    task.status = TaskStatus::New;
    db.update_task(task.clone()).await?;
    global_events.send(GlobalEvent::TaskUpdate { uuid: task.id })?;

    Ok(Json(task))
}

// only returns the task back if the task is in a running state and not timed out or finished
#[allow(unused)]
async fn maybe_timeout_task(task: Task, db: &mut Database) -> Result<Option<Task>, AppError> {
    if task.status != TaskStatus::Running {
        return Ok(None);
    }

    let Some(last_heartbeat) = task.last_heartbeat else {
        return Ok(Some(task)); // a running task should always have a last heartbeat from when it was claimed
    };

    if last_heartbeat + TimeDelta::seconds(HEARTBEAT_TIMEOUT_SEC) <= Utc::now() {
        let affected = db.timeout_task(task.id).await?;
        if cfg!(debug_assertions) && affected != 1 {
            warn!("Expected to timeout 1 task, but timed out {affected}");
        }

        return Ok(None);
    }

    Ok(Some(task))
}

#[cfg(test)]
mod tests {
    use vickylib::database::entities::Task;

    #[test]
    fn add_new_conflicting_task() {
        let task = Task::builder()
            .display_name("Test 1")
            .read_lock("mauz")
            .write_lock("mauz")
            .build();
        assert!(task.is_err());
    }

    #[test]
    fn add_new_not_conflicting_task() {
        let task = Task::builder()
            .display_name("Test 1")
            .read_lock("mauz")
            .read_lock("mauz")
            .write_lock("delete_everything")
            .build();
        assert!(task.is_ok())
    }
}
