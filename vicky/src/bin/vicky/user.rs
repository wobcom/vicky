use crate::{auth::User, errors::AppError};
use rocket::{get, serde::json::Json};
use vickylib::database::entities::user::Me;

#[get("/")]
pub fn get_user(user: User) -> Result<Json<Me>, AppError> {
    let me = Me {
        full_name: user.full_name,
        role: user.role,
    };

    Ok(Json(me))
}
