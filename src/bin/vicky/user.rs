use rocket::{http::CookieJar, serde::json::Json, State};
use serde::{Serialize, Deserialize};
use vickylib::vicky::errors::VickyError;

use crate::{Config, auth::User};



#[derive(Debug, PartialEq, Serialize, Deserialize)]

struct Me {
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