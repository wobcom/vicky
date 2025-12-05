use crate::auth::AnyAuthGuard;
use crate::errors::AppError;
use rocket::serde::json::Json;
use rocket::{get, patch};
use uuid::Uuid;
use vickylib::database::entities::lock::PoisonedLock;
use vickylib::database::entities::{Database, Lock};

#[get("/poisoned")]
pub async fn locks_get_poisoned(
    db: Database,
    _auth: AnyAuthGuard,
) -> Result<Json<Vec<Lock>>, AppError> {
    let poisoned_locks: Vec<Lock> = db.get_poisoned_locks().await?;
    Ok(Json(poisoned_locks))
}

#[get("/poisoned_detailed")]
pub async fn locks_get_detailed_poisoned(
    db: Database,
    _auth: AnyAuthGuard,
) -> Result<Json<Vec<PoisonedLock>>, AppError> {
    let poisoned_locks: Vec<PoisonedLock> = db.get_poisoned_locks_with_tasks().await?;
    Ok(Json(poisoned_locks))
}

#[get("/active")]
pub async fn locks_get_active(
    db: Database,
    _auth: AnyAuthGuard,
) -> Result<Json<Vec<Lock>>, AppError> {
    let locks: Vec<Lock> = db.get_active_locks().await?;
    Ok(Json(locks))
}

#[patch("/unlock/<lock_id>")]
pub async fn locks_unlock(
    db: Database,
    _auth: AnyAuthGuard, // TODO: Should actually be user-only, but we don't have that yet
    lock_id: Uuid,
) -> Result<(), AppError> {
    db.unlock_lock(lock_id).await?;
    Ok(())
}
