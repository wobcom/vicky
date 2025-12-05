use rocket::State;
use rocket::{get, serde::json::Json};

use crate::config::WebConfig;
use crate::errors::AppError;

#[get("/")]
pub fn get_web_config(cfg: &State<WebConfig>) -> Result<Json<WebConfig>, AppError> {
    Ok(Json(cfg.inner().clone()))
}
