use rocket::{get, serde::json::Json};
use serde::{Deserialize, Serialize};

use crate::{auth::User, errors::AppError};

#[allow(unused)]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Me {
    full_name: String,
    role: String,
}

#[get("/")]
pub fn get_user(user: User) -> Result<Json<Me>, AppError> {
    let me = Me {
        full_name: user.full_name,
        role: String::from("admin"),
    };

    Ok(Json(me))
}
