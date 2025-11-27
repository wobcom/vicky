pub mod lock;
pub mod task;
pub mod user;

pub use lock::{Lock, LockKind};
use rocket_sync_db_pools::database;
pub use task::Task;

#[database("postgres_db")]
pub struct Database(diesel::PgConnection);
