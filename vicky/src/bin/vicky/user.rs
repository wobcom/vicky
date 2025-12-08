use crate::auth::UserGuard;
use crate::errors::AppError;
use rocket::{get, serde::json::Json};
use vickylib::database::entities::user::Me;

#[get("/")]
pub fn get_user(user: UserGuard) -> Result<Json<Me>, AppError> {
    let me = Me {
        full_name: user.0.name,
        role: user.0.role,
    };

    Ok(Json(me))
}
