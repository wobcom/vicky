use diesel::PgConnection;
use rocket::get;
use rocket::serde::json::Json;
use vickylib::database::entities::{Database, Lock};
use vickylib::database::entities::lock::db_impl::LockDatabase;
use crate::auth::{Machine, User};
use crate::errors::AppError;

async fn locks_get_poisoned(db: &Database) -> Result<Json<Vec<Lock>>, AppError> {
    let poisoned_locks: Vec<Lock> = db.run(PgConnection::get_poisoned_locks).await?;
    Ok(Json(poisoned_locks))
}

#[get("/poisoned")]
pub async fn locks_get_poisoned_user(
    db: Database,
    _user: User,
) -> Result<Json<Vec<Lock>>, AppError> {
    locks_get_poisoned(&db).await
}

#[get("/poisoned", rank = 2)]
pub async fn locks_get_poisoned_machine(
    db: Database,
    _machine: Machine,
) -> Result<Json<Vec<Lock>>, AppError> {
    locks_get_poisoned(&db).await
}

async fn locks_get_active(db: &Database) -> Result<Json<Vec<Lock>>, AppError> {
    let locks: Vec<Lock> = db.run(PgConnection::get_active_locks).await?;
    Ok(Json(locks))
}

#[get("/active")]
pub async fn locks_get_active_user(
    db: Database,
    _user: User,
) -> Result<Json<Vec<Lock>>, AppError> {
    locks_get_active(&db).await
}

#[get("/active", rank = 2)]
pub async fn locks_get_active_machine(
    db: Database,
    _machine: Machine,
) -> Result<Json<Vec<Lock>>, AppError> {
    locks_get_active(&db).await
}
