use diesel::{AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::database::entities::Task;
use crate::database::entities::lock::db_impl::DbLock;
use crate::database::entities::task::db_impl::DbTask;

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    clap::ValueEnum,
    strum::Display,
    strum::IntoStaticStr,
    FromSqlRow,
    AsExpression,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[diesel(sql_type = db_impl::LockKindSqlType)]
pub enum LockKind {
    Read,
    Write,
}

impl LockKind {
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

    pub fn clear_poison(&mut self) {
        self.poisoned_by = None;
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

        PoisonedLock {
            id: lock.id,
            name: lock.name,
            kind: lock.lock_type,
            poisoned: Task::from((task, vec![])),
        }
    }
}

pub mod db_impl {
    use clap::ValueEnum;
    use diesel::deserialize::FromSql;
    use diesel::pg::PgValue;
    use diesel::prelude::*;
    use diesel::serialize::{IsNull, Output, ToSql};
    use diesel::{SqlType, update};
    use serde::Serialize;
    use std::io::Write;
    use uuid::Uuid;

    use crate::database::entities::lock::{Lock, LockKind, PoisonedLock};
    use crate::database::entities::task::TaskStatus;
    use crate::database::entities::task::db_impl::DbTask;
    use crate::database::schema::{locks, tasks};
    use crate::errors::VickyError;

    #[derive(SqlType)]
    #[diesel(postgres_type(name = "LockKind_Type"))]
    pub struct LockKindSqlType;

    impl ToSql<LockKindSqlType, diesel::pg::Pg> for LockKind {
        fn to_sql<'b>(
            &'b self,
            out: &mut Output<'b, '_, diesel::pg::Pg>,
        ) -> diesel::serialize::Result {
            out.write_all(self.to_string().as_bytes())?;
            Ok(IsNull::No)
        }
    }

    impl FromSql<LockKindSqlType, diesel::pg::Pg> for LockKind {
        fn from_sql(bytes: PgValue<'_>) -> diesel::deserialize::Result<Self> {
            let lock_kind_str = String::from_utf8(bytes.as_bytes().to_vec())?;
            LockKind::from_str(&lock_kind_str, true).map_err(|e| e.into())
        }
    }

    #[derive(Selectable, Identifiable, Queryable, Debug, Serialize)]
    #[diesel(table_name = locks)]
    pub struct DbLock {
        pub id: Uuid,
        pub task_id: Uuid,
        pub name: String,
        pub lock_type: LockKind,
        pub poisoned_by_task: Option<Uuid>,
    }

    #[derive(Insertable, Debug)]
    #[diesel(table_name = locks)]
    pub struct NewDbLock {
        pub task_id: Uuid,
        pub name: String,
        pub lock_type: LockKind,
        pub poisoned_by_task: Option<Uuid>,
    }

    impl NewDbLock {
        pub fn from_lock(lock: &Lock, task_id: Uuid) -> Self {
            NewDbLock {
                task_id,
                name: lock.name.clone(),
                lock_type: lock.kind,
                poisoned_by_task: lock.poisoned_by,
            }
        }
    }

    impl From<DbLock> for Lock {
        fn from(lock: DbLock) -> Lock {
            Lock {
                name: lock.name,
                kind: lock.lock_type,
                poisoned_by: lock.poisoned_by_task,
            }
        }
    }

    pub trait LockDatabase {
        fn get_poisoned_locks(&mut self) -> Result<Vec<Lock>, VickyError>;
        fn get_poisoned_locks_with_tasks(&mut self) -> Result<Vec<PoisonedLock>, VickyError>;
        fn get_active_locks(&mut self) -> Result<Vec<Lock>, VickyError>;
        fn poison_all_locks_by_task(&mut self, task_id: Uuid) -> Result<usize, VickyError>;
        fn unlock_lock(&mut self, lock_uuid: &Uuid) -> Result<usize, VickyError>;
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
            let locks = locks::table
                .select(locks::all_columns)
                .left_join(tasks::table.on(locks::task_id.eq(tasks::id)))
                .filter(
                    locks::poisoned_by_task
                        .is_not_null()
                        .or(tasks::status.eq(TaskStatus::Running)),
                )
                .load::<DbLock>(self)?
                .into_iter()
                .map(Lock::from)
                .collect();

            Ok(locks)
        }

        fn poison_all_locks_by_task(&mut self, task_id: Uuid) -> Result<usize, VickyError> {
            let affected = update(locks::table.filter(locks::task_id.eq(task_id)))
                .set(locks::poisoned_by_task.eq(task_id))
                .execute(self)?;
            Ok(affected)
        }

        fn unlock_lock(&mut self, lock_uuid: &Uuid) -> Result<usize, VickyError> {
            let affected = update(locks::table.filter(locks::id.eq(lock_uuid)))
                .set(locks::poisoned_by_task.eq(None::<Uuid>))
                .execute(self)?;
            Ok(affected)
        }
    }
}
