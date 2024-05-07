use rocket::{get, serde::json::Json};
use rocket::{State};
use serde::{Deserialize, Serialize};

use crate::{errors::AppError, WebConfig};

#[derive(Debug, PartialEq, Serialize, Deserialize)]

pub struct Me {
    full_name: String,
    role: String,
}

#[get("/")]
pub fn get_web_config(cfg: &State<WebConfig>) -> Result<Json<WebConfig>, AppError> {
    Ok(Json(cfg.inner().clone()))
}
