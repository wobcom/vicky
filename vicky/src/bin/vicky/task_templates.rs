use crate::auth::AnyAuthGuard;
use crate::errors::AppError;
use crate::events::GlobalEvent;
use diesel::result::DatabaseErrorKind;
use diesel::result::Error::DatabaseError;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, get, post};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;
use uuid::Uuid;
use vickylib::database::entities::task::FlakeRef;
use vickylib::database::entities::task_template::{
    TaskTemplateError, TaskTemplateLock, TaskTemplateVariable,
};
use vickylib::database::entities::{Database, Task, TaskTemplate};
use vickylib::errors::VickyError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoTaskTemplateNew {
    name: String,
    display_name_template: String,
    flake_ref: FlakeRef,
    #[serde(default)]
    locks: Vec<TaskTemplateLock>,
    #[serde(default)]
    features: Vec<String>,
    group: Option<String>,
    #[serde(default)]
    variables: Vec<TaskTemplateVariable>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoTaskTemplateInstantiate {
    #[serde(default)]
    needs_confirmation: bool,
    #[serde(default)]
    variables: HashMap<String, String>,
}

fn template_error_fail(err: TaskTemplateError) -> AppError {
    log::warn!("invalid task template: {err}");
    AppError::HttpError(Status::BadRequest)
}

#[get("/")]
pub async fn task_templates_get(
    db: Database,
    _auth: AnyAuthGuard,
) -> Result<Json<Vec<TaskTemplate>>, AppError> {
    let templates = db.get_all_task_templates().await?;
    Ok(Json(templates))
}

#[get("/<id>")]
pub async fn task_templates_get_specific(
    id: Uuid,
    db: Database,
    _auth: AnyAuthGuard,
) -> Result<Json<Option<TaskTemplate>>, AppError> {
    let template = db.get_task_template(id).await?;
    Ok(Json(template))
}

#[post("/", format = "json", data = "<task_template>")]
pub async fn task_templates_add(
    task_template: Json<RoTaskTemplateNew>,
    db: Database,
    _auth: AnyAuthGuard,
) -> Result<Json<TaskTemplate>, AppError> {
    let task_template = task_template.into_inner();

    let task_template = TaskTemplate {
        id: Uuid::new_v4(),
        name: task_template.name,
        display_name_template: task_template.display_name_template,
        flake_ref: task_template.flake_ref,
        locks: task_template.locks,
        features: task_template.features,
        group: task_template.group,
        variables: task_template.variables,
        created_at: chrono::Utc::now(),
    };

    task_template.validate().map_err(template_error_fail)?;

    match db.put_task_template(task_template.clone()).await {
        Ok(_) => Ok(Json(task_template)),
        Err(VickyError::Diesel {
            source: DatabaseError(DatabaseErrorKind::UniqueViolation, _),
        }) => Err(AppError::HttpError(Status::Conflict)),
        Err(err) => Err(err.into()),
    }
}

#[post("/<id>/instantiate", format = "json", data = "<request>")]
pub async fn task_templates_instantiate(
    id: Uuid,
    request: Json<RoTaskTemplateInstantiate>,
    db: Database,
    global_events: &State<broadcast::Sender<GlobalEvent>>,
    _auth: AnyAuthGuard,
) -> Result<Json<Task>, AppError> {
    let template = db
        .get_task_template(id)
        .await?
        .ok_or(AppError::HttpError(Status::NotFound))?;

    let request = request.into_inner();

    let task = template
        .instantiate(request.variables, request.needs_confirmation)
        .map_err(template_error_fail)?;

    db.put_task(task.clone()).await?;
    global_events.send(GlobalEvent::TaskAdd)?;

    Ok(Json(task))
}
