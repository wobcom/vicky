pub mod lock;
pub mod task;
pub mod user;

use crate::database::entities::lock::PoisonedLock;
use crate::database::entities::lock::db_impl::LockDatabase;
use crate::database::entities::task::TaskStatus;
use crate::database::entities::task::db_impl::TaskDatabase;
use crate::database::entities::user::User;
use crate::database::entities::user::db_impl::UserDatabase;
use crate::errors::VickyError;
use crate::query::FilterParams;
use chrono::Utc;
use delegate::delegate;
pub use lock::{Lock, LockKind};
use rocket_sync_db_pools::{ConnectionPool, database};
pub use task::Task;
use uuid::Uuid;

#[database("postgres_db")]
pub struct Database(diesel::PgConnection);

impl Database {
    pub async fn get_one_from_pool(pool: &ConnectionPool<Self, diesel::PgConnection>) -> Option<Self> {
        pool.get().await.map(Self)
    }

    delegate! {
        #[await(false)]
        #[expr(self.run(move |conn| $).await)]
        #[through(TaskDatabase)]
        to conn {
            pub async fn count_all_tasks<F: Into<FilterParams> + Send + 'static>(
                &self,
                task_status: Option<TaskStatus>,
                filters: F,
            ) -> Result<i64, VickyError>;
            pub async fn get_all_tasks_filtered<F: Into<FilterParams> + Send + 'static>(
                &self,
                task_status: Option<TaskStatus>,
                filters: F,
            ) -> Result<Vec<Task>, VickyError>;
            pub async fn get_all_tasks(&self) -> Result<Vec<Task>, VickyError>;
            pub async fn get_task(&self, task_id: Uuid) -> Result<Option<Task>, VickyError>;
            pub async fn put_task(&self, task: Task) -> Result<usize, VickyError>;
            pub async fn update_task(&self, #[as_ref] task: Task) -> Result<usize, VickyError>;
            pub async fn confirm_task(&self, uuid: Uuid) -> Result<usize, VickyError>;
            pub async fn has_task(&self, task_id: Uuid) -> Result<bool, VickyError>;
            pub async fn has_running_task(&self, task_id: Uuid) -> Result<bool, VickyError>;
            pub async fn perform_timeout_sweep(&self) -> Result<(usize, usize), VickyError>;
            pub async fn timeout_task(&self, task_id: Uuid) -> Result<usize, VickyError>;
        }

        #[await(false)]
        #[expr(self.run(move |conn| $).await)]
        #[through(LockDatabase)]
        to conn {
            pub async fn get_poisoned_locks(&self) -> Result<Vec<Lock>, VickyError>;
            pub async fn get_poisoned_locks_with_tasks(&self) -> Result<Vec<PoisonedLock>, VickyError>;
            pub async fn get_active_locks(&self) -> Result<Vec<Lock>, VickyError>;
            pub async fn unlock_lock(&self, #[as_ref] lock_uuid: Uuid) -> Result<usize, VickyError>;
        }

        #[await(false)]
        #[expr(self.run(move |conn| $).await)]
        #[through(UserDatabase)]
        to conn {
            pub async fn get_user(&self, id: Uuid) -> Result<Option<User>, VickyError>;
            pub async fn upsert_user(&self, user: User) -> Result<(), VickyError>;
        }
    }

    pub async fn register_task_heartbeat(&self, task_id: Uuid) -> Result<usize, VickyError> {
        let heartbeat = Utc::now().naive_utc();
        self.run(move |d| d.register_task_heartbeat(task_id, heartbeat))
            .await
    }
}
