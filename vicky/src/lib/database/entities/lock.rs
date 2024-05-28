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
    use diesel::{Identifiable, Insertable, Queryable, Selectable};
    use uuid::Uuid;

    use crate::database::entities::Lock;
    use crate::database::schema::locks;

    #[derive(Insertable, Selectable, Identifiable, Queryable, Debug)]
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
}
