use std::vec;

use etcd_client::{GetOptions, Client};
use rocket::{State, serde::json::Json, response::status::NotFound};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vickylib::{etcd::client::ClientExt, documents::{Task, TaskStatus, TaskResult, FlakeRef, Lock}};

use crate::{errors::{Error, HTTPError}, scheduler::Scheduler};

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


#[get("/")]
pub async fn tasks_get(etcd: &State<Client>) -> Result<Json<Vec<Task>>, Error> {
    let get_options: GetOptions = GetOptions::new().with_prefix().with_sort(etcd_client::SortTarget::Create, etcd_client::SortOrder::Descend);
    let mut kv_client = etcd.kv_client().clone();
    let tasks: Vec<Task> = kv_client.get_yaml_list("vicky.wobcom.de/task/manifest".to_string(), Some(get_options)).await?;
    Ok(Json(tasks))
}

#[post("/claim")]
pub async fn tasks_claim(etcd: &State<Client>) ->  Result<Json<Option<Task>>, Error> {
    let mut kv_client = etcd.kv_client().clone();
    let get_options: GetOptions = GetOptions::new().with_prefix().with_sort(etcd_client::SortTarget::Create, etcd_client::SortOrder::Descend);
    let tasks: Vec<Task> = kv_client.get_yaml_list("vicky.wobcom.de/task/manifest".to_string(), Some(get_options)).await?;

    let scheduler = Scheduler::new(tasks)?;
    let next_task = scheduler.get_next_task();

    match next_task {
        Some(next_task) => {
            let key = format!("vicky.wobcom.de/task/manifest/{}", next_task.id.to_string());
            let mut task: Task = kv_client.get_yaml(key.clone(), None).await?.ok_or(HTTPError::NotFound)?;
            task.status = TaskStatus::RUNNING;
            kv_client.put_yaml(key.clone(), &task, None).await?;
        
            Ok(Json(Some(task)))
        },
        None => Ok(Json(None)),
    }

   
}


#[post("/finish/<id>", format = "json", data = "<finish>")]
pub async fn tasks_finish(id: String, finish: Json<RoTaskFinish>, etcd: &State<Client>) ->  Result<Json<Task>, Error> {
    let task_uuid = Uuid::parse_str(&id)?;
    let mut kv_client = etcd.kv_client().clone();
    let key = format!("vicky.wobcom.de/task/manifest/{}", task_uuid.to_string());

    let mut task: Task = kv_client.get_yaml(key.clone(), None).await?.ok_or(HTTPError::NotFound)?;
    task.status = TaskStatus::FINISHED(finish.result.clone());
    kv_client.put_yaml(key.clone(), &task, None).await?;

    Ok(Json(task))
}

#[post("/", data = "<task>")]
pub async fn tasks_add(task: Json<RoTaskNew>, etcd: &State<Client>) -> Result<Json<RoTask>, Error> {
    let mut kv_client = etcd.kv_client().clone();

    let task_uuid = Uuid::new_v4();

    let task_manifest = Task { 
        id: task_uuid,
        status: TaskStatus::NEW,
        locks: task.locks.clone(),
        display_name: task.display_name.clone(),
        flake_ref: FlakeRef { flake: task.flake_ref.flake.clone(), args: task.flake_ref.args.clone() },
    };

    kv_client.put_yaml(format!("vicky.wobcom.de/task/manifest/{}", task_uuid), &task_manifest, None).await?;


    let ro_task = RoTask {
        id: task_uuid,
        status: TaskStatus::NEW
    };

    Ok(Json(ro_task))

}
