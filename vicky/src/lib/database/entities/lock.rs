use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::entities::lock::db_impl::DbLock;
use crate::database::entities::Task;
use crate::database::entities::task::db_impl::DbTask;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Lock {
    #[serde(rename = "WRITE")]
    Write {
        name: String,
        poisoned: Option<Uuid>,
    },
    #[serde(rename = "READ")]
    Read {
        name: String,
        poisoned: Option<Uuid>,
    },
}

impl Lock {
    pub fn is_conflicting(&self, other: &Lock) -> bool {
        if self.name() != other.name() {
            return false;
        }

        if self.is_poisoned() || other.is_poisoned() {
            return true;
        }

        matches!(
            (self, other),
            (Lock::Write { .. }, Lock::Write { .. })
                | (Lock::Read { .. }, Lock::Write { .. })
                | (Lock::Write { .. }, Lock::Read { .. })
        )
    }

    pub fn poison(&mut self, by_task: &Uuid) {
        match self {
            Lock::Write {
                ref mut poisoned, ..
            } => {
                *poisoned = Some(*by_task);
            }
            Lock::Read {
                ref mut poisoned, ..
            } => {
                *poisoned = Some(*by_task);
            }
        };
    }

    pub fn name(&self) -> &str {
        match self {
            Lock::Write { name, .. } => name,
            Lock::Read { name, .. } => name,
        }
    }

    pub fn is_poisoned(&self) -> bool {
        match self {
            Lock::Write { poisoned, .. } => poisoned,
            Lock::Read { poisoned, .. } => poisoned,
        }
        .is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PoisonedLock {
    Write { id: Uuid, name: String, poisoned: Task },
    Read { id: Uuid, name: String, poisoned: Task },
}

impl From<(DbLock, DbTask)> for PoisonedLock {
    fn from(value: (DbLock, DbTask)) -> Self {
        let (lock, task) = value;
        match lock.type_.as_str() {
            // Other locks data is omitted here because of recursivity and since this is explicitly for a PoisonedLock
            "WRITE" => PoisonedLock::Write {
                id: lock.id.unwrap_or_default(),
                name: lock.name,
                poisoned: Task::from((task, vec![])),
            },
            "READ" => PoisonedLock::Read {
                id: lock.id.unwrap_or_default(),
                name: lock.name,
                poisoned: Task::from((task, vec![])),
            },
            _ => panic!("Unexpected lock type received."),
        }
    }
}

pub mod db_impl {
    use diesel::prelude::*;
    use diesel::update;
    use serde::Serialize;
    use uuid::Uuid;

    use crate::database::entities::Lock;
    use crate::database::entities::lock::PoisonedLock;
    use crate::database::entities::task::db_impl::DbTask;
    use crate::database::entities::task::TaskStatus;
    use crate::database::schema::{locks, tasks};
    use crate::errors::VickyError;

    #[derive(Insertable, Selectable, Identifiable, Queryable, Debug, Serialize)]
    #[diesel(table_name = locks)]
    pub struct DbLock {
        pub id: Option<Uuid>,
        pub task_id: Uuid,
        pub name: String,
        pub type_: String,
        pub poisoned_by_task: Option<Uuid>,
    }

    #[derive(Debug, Serialize)]
    pub struct PoisonedDbLock {
        pub id: Uuid,
        pub task_id: Uuid,
        pub name: String,
        pub type_: String,
        pub task: DbTask,
    }

    impl DbLock {
        pub fn from_lock(lock: &Lock, task_id: Uuid) -> Self {
            // Converting a Lock to a DbLock only happens when inserting or updating the database,
            // in which case the id column is irrelevant as it's auto generated in the database.
            // A DbLock should not be inserted into a database anyway, as it's just a transient type
            // for inserting a NewDbLock. Thus, id is set to -1 here. Maybe this can be improved wholly?
            // At least it works.
            match lock {
                Lock::Write { name, poisoned } => DbLock {
                    id: None,
                    task_id,
                    name: name.clone(),
                    type_: "WRITE".to_string(),
                    poisoned_by_task: *poisoned,
                },
                Lock::Read { name, poisoned } => DbLock {
                    id: None,
                    task_id,
                    name: name.clone(),
                    type_: "READ".to_string(),
                    poisoned_by_task: *poisoned,
                },
            }
        }
    }

    impl From<DbLock> for Lock {
        fn from(lock: DbLock) -> Lock {
            match lock.type_.as_str() {
                "WRITE" => Lock::Write {
                    name: lock.name,
                    poisoned: lock.poisoned_by_task,
                },
                "READ" => Lock::Read {
                    name: lock.name,
                    poisoned: lock.poisoned_by_task,
                },
                _ => panic!(
                    "Can't parse lock from database lock. Database corrupted? \
                Expected READ or WRITE but found {} as type at key {}.",
                    lock.type_,
                    lock.id.unwrap_or_default()
                ),
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
                let poisoned_db_locks: Vec<DbLock> =
                    locks::table.filter(locks::poisoned_by_task.is_not_null()).load(self)?;
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
