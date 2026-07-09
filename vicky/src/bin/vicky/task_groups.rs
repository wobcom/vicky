use rocket::{State, post};
use rocket::{get, serde::json::Json};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;
use vickylib::database::entities::task_group::TaskGroup;
use vickylib::database::entities::Database;
use vickylib::query::FilterParams;
use vickylib::vicky::events::GlobalEvent;

use crate::auth::{AnyAuthGuard, MachineGuard};
use crate::errors::AppError;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct RoTaskGroupNew {
    name: String,
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Count {
    count: i64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Empty {
}

#[get("/count")]
pub async fn task_groups_count(
    db: Database,
    _auth: AnyAuthGuard,
) -> Result<Json<Count>, AppError> {
    let task_group_count = db.count_all_task_groups().await?;
    let c: Count = Count { count: task_group_count };
    Ok(Json(c))
}

#[get("/?<filter_params..>")]
pub async fn task_groups_get(
    db: Database,
    _auth: AnyAuthGuard,
    filter_params: Option<FilterParams>,
) -> Result<Json<Vec<TaskGroup>>, AppError> {
    let task_group: Vec<TaskGroup> = db
        .get_task_groups_filtered(filter_params)
        .await?;
    Ok(Json(task_group))
}

#[get("/<id>")]
pub async fn task_groups_get_specific(
    id: Uuid,
    db: Database,
    _auth: AnyAuthGuard,
) -> Result<Json<Option<TaskGroup>>, AppError> {
    let tasks: Option<TaskGroup> = db.get_task_group(id).await?;
    Ok(Json(tasks))
}


#[post("/", data = "<task_group>")]
pub async fn task_groups_add(
    task_group: Json<RoTaskGroupNew>,
    db: Database,
    _machine: MachineGuard,
    global_events: &State<broadcast::Sender<GlobalEvent>>,
) -> Result<Json<Empty>, AppError> {
    

    let task_group_ro = task_group.into_inner();

    let db_task_group = TaskGroup::new(
        task_group_ro.name,
    );

    db.put_task_groups(db_task_group).await?;

    global_events.send(GlobalEvent::TaskGroupAdd)?;

    Ok(Json(Empty{}))
}
