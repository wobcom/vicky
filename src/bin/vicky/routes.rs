use etcd_client::{Client};
use rocket::{State, serde::json::Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vickylib::{documents::{Task, TaskStatus, TaskResult, FlakeRef, Lock, DocumentClient}, vicky::{scheduler::Scheduler, errors::{HTTPError, VickyError}}};


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
pub async fn tasks_get(etcd: &State<Client>) -> Result<Json<Vec<Task>>, VickyError> {
    let tasks: Vec<Task> = etcd.get_all_tasks().await?;
    Ok(Json(tasks))
}

#[post("/claim")]
pub async fn tasks_claim(etcd: &State<Client>) ->  Result<Json<Option<Task>>, VickyError> {
    let tasks = etcd.get_all_tasks().await?;
    let scheduler = Scheduler::new(tasks)?;
    let next_task = scheduler.get_next_task();

    match next_task {
        Some(next_task) => {
            let mut task = etcd.get_task(next_task.id).await?.ok_or(HTTPError::NotFound)?;
            task.status = TaskStatus::RUNNING;
            etcd.put_task(&task).await?;
            Ok(Json(Some(task)))
        },
        None => Ok(Json(None)),
    }

   
}


#[post("/finish/<id>", format = "json", data = "<finish>")]
pub async fn tasks_finish(id: String, finish: Json<RoTaskFinish>, etcd: &State<Client>) ->  Result<Json<Task>, VickyError> {
    let task_uuid = Uuid::parse_str(&id)?;
    let mut task = etcd.get_task(task_uuid).await?.ok_or(HTTPError::NotFound)?; 
    task.status = TaskStatus::FINISHED(finish.result.clone());
    etcd.put_task(&task).await?;
    Ok(Json(task))
}

#[post("/", data = "<task>")]
pub async fn tasks_add(task: Json<RoTaskNew>, etcd: &State<Client>) -> Result<Json<RoTask>, VickyError> {
    let task_uuid = Uuid::new_v4();

    let task_manifest = Task { 
        id: task_uuid,
        status: TaskStatus::NEW,
        locks: task.locks.clone(),
        display_name: task.display_name.clone(),
        flake_ref: FlakeRef { flake: task.flake_ref.flake.clone(), args: task.flake_ref.args.clone() },
    };

    etcd.put_task(&task_manifest).await?;

    let ro_task = RoTask {
        id: task_uuid,
        status: TaskStatus::NEW
    };

    Ok(Json(ro_task))

}
