use diesel::PgConnection;
use rocket::{get, patch};
use rocket::serde::json::Json;
use uuid::Uuid;
use vickylib::database::entities::{Database, Lock};
use vickylib::database::entities::lock::db_impl::LockDatabase;
use vickylib::database::entities::lock::PoisonedLock;
use crate::auth::{Machine, User};
use crate::errors::AppError;

async fn locks_get_poisoned(db: &Database) -> Result<Json<Vec<Lock>>, AppError> {
    let poisoned_locks: Vec<Lock> = db.run(PgConnection::get_poisoned_locks).await?;
    Ok(Json(poisoned_locks))
}

async fn locks_get_detailed_poisoned(db: &Database) -> Result<Json<Vec<PoisonedLock>>, AppError> {
    let poisoned_locks: Vec<PoisonedLock> = db.run(PgConnection::get_poisoned_locks_with_tasks).await?;
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


#[get("/poisoned_detailed")]
pub async fn locks_get_detailed_poisoned_user(
    db: Database,
    _user: User,
) -> Result<Json<Vec<PoisonedLock>>, AppError> {
    locks_get_detailed_poisoned(&db).await
}

#[get("/poisoned_detailed", rank = 2)]
pub async fn locks_get_detailed_poisoned_machine(
    db: Database,
    _machine: Machine,
) -> Result<Json<Vec<PoisonedLock>>, AppError> {
    locks_get_detailed_poisoned(&db).await
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

#[patch("/unlock/<lock_id>")]
pub async fn locks_unlock(
    db: Database,
    _user: Machine, // TODO: Should actually be user-only, but we don't have that yet
    lock_id: String,
) -> Result<(), AppError> {
    let lock_uuid = Uuid::try_parse(&lock_id)?;
    
    db.run(move |conn| conn.unlock_lock(&lock_uuid)).await?;
    Ok(())
}