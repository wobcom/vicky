use rocket::{get, serde::json::Json};
use serde::{Serialize, Deserialize};
use vickylib::vicky::errors::VickyError;

use crate::{auth::User};



#[derive(Debug, PartialEq, Serialize, Deserialize)]

pub struct Me {
    full_name: String,
    role: String,
}

#[get("/")]
pub fn get_user(user: User) -> Result<Json<Me>, VickyError>  {

    let me = Me {
        full_name: user.full_name,
        role: String::from("admin"),
    };

    Ok(Json(me))

}