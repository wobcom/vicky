pub mod lock;
pub mod task;

pub use lock::Lock;
use rocket_sync_db_pools::database;
pub use task::Task;

#[database("postgres_db")]
pub struct Database(diesel::PgConnection);
