use rocket::{get, serde::json::Json};
use serde::{Deserialize, Serialize};
use vickylib::database::entities::task_group::TaskGroup;
use vickylib::database::entities::Database;
use vickylib::query::FilterParams;

use crate::auth::AnyAuthGuard;
use crate::errors::AppError;


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Count {
    count: i64,
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

