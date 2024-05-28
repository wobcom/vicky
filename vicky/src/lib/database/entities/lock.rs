use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Lock {
    WRITE {
        name: String,
        poisoned: Option<Uuid>,
    },
    READ {
        name: String,
        poisoned: Option<Uuid>,
    },
}

impl Lock {
    pub fn is_conflicting(&self, other: &Lock) -> bool {
        match (self, other) {
            (Lock::WRITE { name: name1, .. }, Lock::WRITE { name: name2, .. })
            | (Lock::READ { name: name1, .. }, Lock::WRITE { name: name2, .. })
            | (Lock::WRITE { name: name1, .. }, Lock::READ { name: name2, .. }) => name1 == name2,
            _ => false,
        }
    }

    pub fn poison(&mut self, by_task: &Uuid) {
        match self {
            Lock::WRITE { ref mut poisoned, .. } => {
                *poisoned = Some(*by_task);
            }
            Lock::READ { ref mut poisoned, ..} => {
                *poisoned = Some(*by_task);
            }
        };
    }
}

pub mod db_impl {
    use diesel::prelude::*;
    use serde::Serialize;
    use uuid::Uuid;

    use crate::database::entities::Lock;
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

    impl DbLock {
        pub fn from_lock(lock: &Lock, task_id: Uuid) -> Self {
            // Converting a Lock to a DbLock only happens when inserting or updating the database,
            // in which case the id column is irrelevant as it's auto generated in the database.
            // A DbLock should not be inserted into a database anyway, as it's just a transient type
            // for inserting a NewDbLock. Thus, id is set to -1 here. Maybe this can be improved wholly?
            // At least it works.
            match lock {
                Lock::WRITE { name, poisoned } => DbLock {
                    id: None,
                    task_id,
                    name: name.clone(),
                    type_: "WRITE".to_string(),
                    poisoned_by_task: *poisoned,
                },
                Lock::READ { name, poisoned } => DbLock {
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
                "WRITE" => Lock::WRITE {
                    name: lock.name,
                    poisoned: lock.poisoned_by_task,
                },
                "READ" => Lock::READ {
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
        fn get_active_locks(&mut self) -> Result<Vec<Lock>, VickyError>;
    }

    impl LockDatabase for PgConnection {
        fn get_poisoned_locks(&mut self) -> Result<Vec<Lock>, VickyError> {
            use self::locks::dsl::*;

            let poisoned_locks = {
                let poisoned_db_locks: Vec<DbLock> =
                    locks.filter(poisoned_by_task.is_not_null()).load(self)?;
                poisoned_db_locks.into_iter().map(Lock::from).collect()
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
                            .or(tasks::status.eq(TaskStatus::RUNNING.to_string())),
                    )
                    .load(self)?;
                db_locks.into_iter().map(Lock::from).collect()
            };

            Ok(locks)
        }
    }
}
