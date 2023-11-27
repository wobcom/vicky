use etcd_client::{Client};
use rocket::{get, post, State, serde::json::Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vickylib::{documents::{Task, TaskStatus, TaskResult, FlakeRef, Lock, DocumentClient}, vicky::{scheduler::Scheduler}, logs::LogDrain, s3::client::S3Client, errors::VickyError};
use rocket::response::stream::{EventStream, Event};
use std::time;
use tokio::sync::broadcast::{error::{TryRecvError}, self};
use rocket::{http::Status};


use crate::{auth::{User, Machine}, errors::AppError, events::GlobalEvent};



#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RoTaskNew {
    display_name: String,
    flake_ref: FlakeRef,
    locks: Vec<Lock>,
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
    lines: Vec<String>
}

#[get("/")]
pub async fn tasks_get_user(etcd: &State<Client>, _user: User) -> Result<Json<Vec<Task>>, VickyError> {
    let tasks: Vec<Task> = etcd.get_all_tasks().await?;
    Ok(Json(tasks))
}

#[get("/", rank=2)]
pub async fn tasks_get_machine(etcd: &State<Client>, _machine: Machine) -> Result<Json<Vec<Task>>, VickyError> {
    let tasks: Vec<Task> = etcd.get_all_tasks().await?;
    Ok(Json(tasks))
}

#[get("/<id>")]
pub async fn tasks_specific_get_user(id: String, etcd: &State<Client>, _user: User) -> Result<Json<Option<Task>>, VickyError> {
    let task_uuid = Uuid::parse_str(&id).unwrap();
    let tasks: Option<Task> = etcd.get_task(task_uuid).await?;
    Ok(Json(tasks))
}

#[get("/<id>", rank=2)]
pub async fn tasks_specific_get_machine(id: String, etcd: &State<Client>, _machine: Machine) -> Result<Json<Option<Task>>, VickyError> {
    let task_uuid = Uuid::parse_str(&id).unwrap();
    let tasks: Option<Task> = etcd.get_task(task_uuid).await?;
    Ok(Json(tasks))
}

#[get("/<id>/logs")]
pub async fn tasks_get_logs<'a>(id: String, etcd: &State<Client>, s3: &'a State<S3Client>, _user: User, log_drain: &'a State<&'_ LogDrain>) -> EventStream![Event + 'a] {
    // TODO: Fix Error Handling
    let task_uuid = Uuid::parse_str(&id).unwrap();
    let task = etcd.get_task(task_uuid).await.unwrap().ok_or(AppError::HttpError(Status::NotFound)).unwrap(); 


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
                            // Immediate Retry, doing our best efford ehre.
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
pub async fn tasks_put_logs(id: String, etcd: &State<Client>, logs: Json<LogLines>, _machine: Machine, log_drain: &State<&LogDrain>) ->  Result<Json<()>, AppError> {
    let task_uuid = Uuid::parse_str(&id)?;
    let task = etcd.get_task(task_uuid).await?.ok_or(AppError::HttpError(Status::NotFound))?; 

    match task.status {
        TaskStatus::RUNNING => {
            log_drain.push_logs(id, logs.lines.clone())?;
            Ok(Json(()))
        },
        _ => {
            Err(AppError::HttpError(Status::Locked))?
        }
    }
}

#[post("/claim")]
pub async fn tasks_claim(etcd: &State<Client>, global_events: &State<broadcast::Sender<GlobalEvent>>, _machine: Machine) ->  Result<Json<Option<Task>>, AppError> {
    let tasks = etcd.get_all_tasks().await?;
    let scheduler = Scheduler::new(tasks).map_err(|x| VickyError::Scheduler { source: x })?;
    let next_task = scheduler.get_next_task();

    match next_task {
        Some(next_task) => {
            let mut task = etcd.get_task(next_task.id).await?.ok_or(AppError::HttpError(Status::NotFound))?; 
            task.status = TaskStatus::RUNNING;
            etcd.put_task(&task).await?;
            global_events.send(GlobalEvent::TaskUpdate { uuid: task.id })?;
            Ok(Json(Some(task)))
        },
        None => Ok(Json(None)),
    }
}

#[post("/<id>/finish", format = "json", data = "<finish>")]
pub async fn tasks_finish(id: String, finish: Json<RoTaskFinish>, etcd: &State<Client>, global_events: &State<broadcast::Sender<GlobalEvent>>, _machine: Machine, log_drain: &State<&LogDrain>) ->  Result<Json<Task>, AppError> {
    let task_uuid = Uuid::parse_str(&id)?;
    let mut task = etcd.get_task(task_uuid).await?.ok_or(AppError::HttpError(Status::NotFound))?; 

    log_drain.finish_logs(&id).await?;

    task.status = TaskStatus::FINISHED(finish.result.clone());
    etcd.put_task(&task).await?;
    global_events.send(GlobalEvent::TaskUpdate { uuid: task.id })?;

    Ok(Json(task))
}

#[post("/", data = "<task>")]
pub async fn tasks_add(task: Json<RoTaskNew>, etcd: &State<Client>, global_events: &State<broadcast::Sender<GlobalEvent>>, _machine: Machine) -> Result<Json<RoTask>, AppError> {
    let task_uuid = Uuid::new_v4();

    let task_manifest = Task { 
        id: task_uuid,
        status: TaskStatus::NEW,
        locks: task.locks.clone(),
        display_name: task.display_name.clone(),
        flake_ref: FlakeRef { flake: task.flake_ref.flake.clone(), args: task.flake_ref.args.clone() },
    };

    etcd.put_task(&task_manifest).await?;
    global_events.send(GlobalEvent::TaskAdd)?;

    let ro_task = RoTask {
        id: task_uuid,
        status: TaskStatus::NEW
    };

    Ok(Json(ro_task))
}
