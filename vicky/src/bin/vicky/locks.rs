use crate::auth::{Machine, User};
use crate::errors::AppError;
use rocket::serde::json::Json;
use rocket::{get, patch};
use uuid::Uuid;
use vickylib::database::entities::lock::PoisonedLock;
use vickylib::database::entities::{Database, Lock};

async fn locks_get_poisoned(db: &Database) -> Result<Json<Vec<Lock>>, AppError> {
    let poisoned_locks: Vec<Lock> = db.get_poisoned_locks().await?;
    Ok(Json(poisoned_locks))
}

async fn locks_get_detailed_poisoned(db: &Database) -> Result<Json<Vec<PoisonedLock>>, AppError> {
    let poisoned_locks: Vec<PoisonedLock> = db.get_poisoned_locks_with_tasks().await?;
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
    let locks: Vec<Lock> = db.get_active_locks().await?;
    Ok(Json(locks))
}

#[get("/active")]
pub async fn locks_get_active_user(db: Database, _user: User) -> Result<Json<Vec<Lock>>, AppError> {
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
    lock_id: Uuid,
) -> Result<(), AppError> {
    db.unlock_lock(lock_id).await?;
    Ok(())
}
