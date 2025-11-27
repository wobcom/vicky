use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::entities::lock::db_impl::DbLock;
use crate::database::entities::task::db_impl::DbTask;
use crate::database::entities::Task;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "UPPERCASE")]
pub enum LockKind {
    Read,
    Write,
}

impl LockKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            LockKind::Read => "READ",
            LockKind::Write => "WRITE",
        }
    }

    pub fn is_write(&self) -> bool {
        matches!(self, LockKind::Write)
    }
}

impl TryFrom<&str> for LockKind {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "READ" => Ok(Self::Read),
            "WRITE" => Ok(Self::Write),
            _ => Err("Unexpected lock type received."),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lock {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: LockKind,
    #[serde(rename = "poisoned")]
    pub poisoned_by: Option<Uuid>,
}

impl Lock {
    pub fn read<S: Into<String>>(name: S) -> Self {
        Self::new(name, LockKind::Read)
    }

    pub fn write<S: Into<String>>(name: S) -> Self {
        Self::new(name, LockKind::Write)
    }

    pub fn is_conflicting(&self, other: &Lock) -> bool {
        if self.name() != other.name() {
            return false;
        }

        if self.is_poisoned() || other.is_poisoned() {
            return true;
        }

        self.kind.is_write() || other.kind.is_write()
    }

    pub fn poison(&mut self, by_task: &Uuid) {
        self.poisoned_by = Some(*by_task);
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn is_poisoned(&self) -> bool {
        self.poisoned_by.is_some()
    }

    fn new<S: Into<String>>(name: S, kind: LockKind) -> Self {
        Self {
            name: name.into(),
            kind,
            poisoned_by: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoisonedLock {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: LockKind,
    pub poisoned: Task,
}

impl From<(DbLock, DbTask)> for PoisonedLock {
    fn from(value: (DbLock, DbTask)) -> Self {
        let (lock, task) = value;
        let kind =
            LockKind::try_from(lock.lock_type.as_str()).expect("Unexpected lock type received.");

        PoisonedLock {
            id: lock.id,
            name: lock.name,
            kind,
            poisoned: Task::from((task, vec![])),
        }
    }
}

pub mod db_impl {
    use diesel::prelude::*;
    use diesel::update;
    use serde::Serialize;
    use uuid::Uuid;

    use crate::database::entities::lock::{Lock, LockKind, PoisonedLock};
    use crate::database::entities::task::db_impl::DbTask;
    use crate::database::entities::task::TaskStatus;
    use crate::database::schema::{locks, tasks};
    use crate::errors::VickyError;

    #[derive(Selectable, Identifiable, Queryable, Debug, Serialize)]
    #[diesel(table_name = locks)]
    pub struct DbLock {
        pub id: Uuid,
        pub task_id: Uuid,
        pub name: String,
        pub lock_type: String,
        pub poisoned_by_task: Option<Uuid>,
    }

    #[derive(Insertable, Debug)]
    #[diesel(table_name = locks)]
    pub struct NewDbLock {
        pub task_id: Uuid,
        pub name: String,
        pub lock_type: String,
        pub poisoned_by_task: Option<Uuid>,
    }

    impl NewDbLock {
        pub fn from_lock(lock: &Lock, task_id: Uuid) -> Self {
            NewDbLock {
                task_id,
                name: lock.name.clone(),
                lock_type: lock.kind.as_str().to_string(),
                poisoned_by_task: lock.poisoned_by,
            }
        }
    }

    impl From<DbLock> for Lock {
        fn from(lock: DbLock) -> Lock {
            let kind = LockKind::try_from(lock.lock_type.as_str()).unwrap_or_else(|_| {
                panic!(
                    "Can't parse lock from database lock. Database corrupted? \
                Expected READ or WRITE but found {} as type at key {}.",
                    lock.lock_type, lock.id
                )
            });

            Lock {
                name: lock.name,
                kind,
                poisoned_by: lock.poisoned_by_task,
            }
        }
    }

    pub trait LockDatabase {
        fn get_poisoned_locks(&mut self) -> Result<Vec<Lock>, VickyError>;
        fn get_poisoned_locks_with_tasks(&mut self) -> Result<Vec<PoisonedLock>, VickyError>;
        fn get_active_locks(&mut self) -> Result<Vec<Lock>, VickyError>;
        fn unlock_lock(&mut self, lock_uuid: &Uuid) -> Result<(), VickyError>;
    }

    impl LockDatabase for PgConnection {
        fn get_poisoned_locks(&mut self) -> Result<Vec<Lock>, VickyError> {
            let poisoned_locks = {
                let poisoned_db_locks: Vec<DbLock> = locks::table
                    .filter(locks::poisoned_by_task.is_not_null())
                    .load(self)?;
                poisoned_db_locks.into_iter().map(Lock::from).collect()
            };

            Ok(poisoned_locks)
        }

        fn get_poisoned_locks_with_tasks(&mut self) -> Result<Vec<PoisonedLock>, VickyError> {
            let poisoned_locks = {
                let poisoned_db_locks = locks::table
                    .inner_join(tasks::table.on(locks::poisoned_by_task.eq(tasks::id.nullable())))
                    .select((locks::all_columns, tasks::all_columns))
                    .load::<(DbLock, DbTask)>(self)?;
                poisoned_db_locks
                    .into_iter()
                    .map(PoisonedLock::from)
                    .collect()
            };

            Ok(poisoned_locks)
        }

        fn get_active_locks(&mut self) -> Result<Vec<Lock>, VickyError> {
            let locks = {
                let db_locks: Vec<DbLock> = locks::table
                    .select(locks::all_columns)
                    .left_join(tasks::table.on(locks::task_id.eq(tasks::id)))
                    .filter(
                        locks::poisoned_by_task
                            .is_not_null()
                            .or(tasks::status.eq(TaskStatus::Running.to_string())),
                    )
                    .load(self)?;
                db_locks.into_iter().map(Lock::from).collect()
            };

            Ok(locks)
        }

        fn unlock_lock(&mut self, lock_uuid: &Uuid) -> Result<(), VickyError> {
            update(locks::table.filter(locks::id.eq(lock_uuid)))
                .set(locks::poisoned_by_task.eq(None::<Uuid>))
                .execute(self)?;
            Ok(())
        }
    }
}
