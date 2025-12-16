use crate::database::entities::lock::db_impl::DbLock;
use crate::database::entities::lock::Lock;
use crate::database::entities::task::db_impl::DbTask;
use bon::Builder;
use chrono::naive::serde::ts_seconds;
use chrono::naive::serde::ts_seconds_option;
use chrono::{NaiveDateTime, Utc};
use diesel::{AsExpression, FromSqlRow};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(tag = "result", rename_all = "UPPERCASE")]
pub enum TaskResult {
    Success,
    Error,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, FromSqlRow, AsExpression)]
#[serde(tag = "state", rename_all = "UPPERCASE")]
#[diesel(sql_type = db_impl::TaskStatusSqlType)]
pub enum TaskStatus {
    NeedsUserValidation,
    New,
    Running,
    Finished(TaskResult),
}

type FlakeURI = String;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlakeRef {
    pub flake: FlakeURI,
    pub args: Vec<String>,
}

impl FlakeRef {
    pub fn empty() -> Self {
        FlakeRef {
            flake: "".to_string(),
            args: vec![],
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Builder)]
#[builder(finish_fn(name = _build_unchecked))]
#[builder(derive(Debug, Clone))]
#[builder(on(String, into))]
pub struct Task {
    #[builder(field)]
    pub locks: Vec<Lock>,
    #[builder(field = FlakeRef::empty())]
    pub flake_ref: FlakeRef,
    #[builder(field)]
    pub features: Vec<String>,

    #[builder(default = Uuid::new_v4())]
    pub id: Uuid,
    #[builder(default = "Task")]
    pub display_name: String,
    #[builder(default = TaskStatus::New)]
    pub status: TaskStatus,
    #[serde(with = "ts_seconds")]
    #[builder(skip = Utc::now().naive_utc())]
    pub created_at: NaiveDateTime,
    #[serde(with = "ts_seconds_option")]
    pub claimed_at: Option<NaiveDateTime>,
    #[serde(with = "ts_seconds_option")]
    pub finished_at: Option<NaiveDateTime>,
    pub group: Option<String>,
}

impl AsRef<Task> for Task {
    fn as_ref(&self) -> &Task {
        self
    }
}

impl<T: task_builder::State> TaskBuilder<T> {
    pub fn read_lock<S: Into<String>>(mut self, name: S) -> Self {
        self.locks.push(Lock::read(name));
        self
    }

    pub fn write_lock<S: Into<String>>(mut self, name: S) -> Self {
        self.locks.push(Lock::write(name));
        self
    }

    pub fn locks(mut self, locks: Vec<Lock>) -> Self {
        self.locks = locks;
        self
    }

    pub fn flake<S: Into<FlakeURI>>(mut self, flake_uri: S) -> Self {
        self.flake_ref.flake = flake_uri.into();
        self
    }

    pub fn flake_arg<S: Into<String>>(mut self, flake_arg: S) -> Self {
        self.flake_ref.args.push(flake_arg.into());
        self
    }

    pub fn flake_args(mut self, args: Vec<String>) -> Self {
        self.flake_ref.args = args;
        self
    }

    pub fn requires_feature<S: Into<String>>(mut self, feature: S) -> Self {
        self.features.push(feature.into());
        self
    }

    pub fn requires_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }

    pub fn check_lock_conflict(&self) -> bool {
        self.locks
            .iter()
            .tuple_combinations()
            .any(|(a, b)| a.is_conflicting(b))
    }
}

impl<T: task_builder::IsComplete> TaskBuilder<T> {
    #[allow(clippy::result_large_err)]
    pub fn build(self) -> Result<Task, Self> {
        if self.check_lock_conflict() {
            return Err(self);
        }

        Ok(self._build_unchecked())
    }

    #[cfg(test)]
    pub fn build_unchecked(self) -> Task {
        self._build_unchecked()
    }

    #[cfg(test)]
    pub fn build_expect(self) -> Task {
        match self.build() {
            Ok(task) => task,
            Err(builder) => panic!("TaskBuilder::build() failed while building: {builder:?}"),
        }
    }
}

impl From<(DbTask, Vec<DbLock>)> for Task {
    fn from(value: (DbTask, Vec<DbLock>)) -> Self {
        let (task, locks) = value;
        Task {
            id: task.id,
            display_name: task.display_name,
            status: task.status,
            locks: locks.into_iter().map(Lock::from).collect(),
            flake_ref: FlakeRef {
                flake: task.flake_ref_uri,
                args: task.flake_ref_args,
            },
            features: task.features,
            created_at: task.created_at,
            claimed_at: task.claimed_at,
            finished_at: task.finished_at,
            group: task.group,
        }
    }
}

// this was on purpose because these macro-generated entity types
// mess up the whole namespace and HAVE to be scoped
pub mod db_impl {
    use crate::database::entities::task::{Task, TaskResult, TaskStatus};
    use crate::errors::VickyError;
    use crate::query::FilterParams;
    use chrono::NaiveDateTime;

    // these here are evil >:(
    use crate::database::entities::lock::db_impl::{DbLock, NewDbLock};
    use crate::database::schema::locks;
    use crate::database::schema::tasks;
    use diesel::deserialize::FromSql;
    use diesel::pg::PgValue;
    use diesel::serialize::{IsNull, Output, ToSql};
    use diesel::{
        AsChangeset, BoolExpressionMethods, Connection, ExpressionMethods, Insertable, QueryDsl,
        QueryId, Queryable, RunQueryDsl, SqlType,
    };
    use itertools::Itertools;
    use serde::Serialize;
    use std::collections::HashMap;
    use std::fmt::Display;
    use std::io::Write;
    use uuid::Uuid;

    #[derive(SqlType, QueryId)]
    #[diesel(postgres_type(name = "TaskStatus_Type"))]
    pub struct TaskStatusSqlType;

    impl ToSql<TaskStatusSqlType, diesel::pg::Pg> for TaskStatus {
        fn to_sql<'b>(
            &'b self,
            out: &mut Output<'b, '_, diesel::pg::Pg>,
        ) -> diesel::serialize::Result {
            out.write_all(self.to_string().as_bytes())?;
            Ok(IsNull::No)
        }
    }

    impl FromSql<TaskStatusSqlType, diesel::pg::Pg> for TaskStatus {
        fn from_sql(bytes: PgValue<'_>) -> diesel::deserialize::Result<Self> {
            let task_status_str = String::from_utf8(bytes.as_bytes().to_vec())?;
            Ok(Self::try_from(task_status_str.as_str()).map_err(|e| e.to_string())?)
        }
    }

    #[derive(Insertable, Queryable, AsChangeset, Debug, Serialize)]
    #[diesel(table_name = tasks)]
    pub struct DbTask {
        pub id: Uuid,
        pub display_name: String,
        pub status: TaskStatus,
        pub features: Vec<String>,
        pub flake_ref_uri: String,
        pub flake_ref_args: Vec<String>,
        pub created_at: NaiveDateTime,
        pub claimed_at: Option<NaiveDateTime>,
        pub finished_at: Option<NaiveDateTime>,
        pub group: Option<String>,
    }

    pub const STATE_NEEDS_USER_VALIDATION_STR: &str = "NEEDS_USER_VALIDATION";
    pub const STATE_NEW_STR: &str = "NEW";
    pub const STATE_RUNNING_STR: &str = "RUNNING";
    pub const STATE_FINISHED_SUCCESS_STR: &str = "FINISHED::SUCCESS";
    pub const STATE_FINISHED_ERROR_STR: &str = "FINISHED::ERROR";

    impl Display for TaskStatus {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let str = match self {
                TaskStatus::NeedsUserValidation => STATE_NEEDS_USER_VALIDATION_STR,
                TaskStatus::New => STATE_NEW_STR,
                TaskStatus::Running => STATE_RUNNING_STR,
                TaskStatus::Finished(r) => match r {
                    TaskResult::Success => STATE_FINISHED_SUCCESS_STR,
                    TaskResult::Error => STATE_FINISHED_ERROR_STR,
                },
            };
            write!(f, "{str}")
        }
    }

    impl TryFrom<&str> for TaskStatus {
        type Error = &'static str;

        fn try_from(str: &str) -> Result<Self, Self::Error> {
            match str {
                STATE_NEEDS_USER_VALIDATION_STR => Ok(TaskStatus::NeedsUserValidation),
                STATE_NEW_STR => Ok(TaskStatus::New),
                STATE_RUNNING_STR => Ok(TaskStatus::Running),
                STATE_FINISHED_SUCCESS_STR => Ok(TaskStatus::Finished(TaskResult::Success)),
                STATE_FINISHED_ERROR_STR => Ok(TaskStatus::Finished(TaskResult::Error)),
                _ => Err("Could not deserialize to TaskStatus"),
            }
        }
    }

    impl From<Task> for DbTask {
        fn from(task: Task) -> DbTask {
            DbTask {
                id: task.id,
                display_name: task.display_name,
                status: task.status,
                features: task.features,
                flake_ref_uri: task.flake_ref.flake,
                flake_ref_args: task.flake_ref.args,
                created_at: task.created_at,
                claimed_at: task.claimed_at,
                finished_at: task.finished_at,
                group: task.group,
            }
        }
    }

    pub trait TaskDatabase {
        fn count_all_tasks<F: Into<FilterParams>>(
            &mut self,
            task_status: Option<TaskStatus>,
            filters: F,
        ) -> Result<i64, VickyError>;
        fn get_all_tasks_filtered<F: Into<FilterParams>>(
            &mut self,
            task_status: Option<TaskStatus>,
            filters: F,
        ) -> Result<Vec<Task>, VickyError>;
        fn get_all_tasks(&mut self) -> Result<Vec<Task>, VickyError>;
        fn get_task(&mut self, task_id: Uuid) -> Result<Option<Task>, VickyError>;
        fn put_task(&mut self, task: Task) -> Result<(), VickyError>;
        fn update_task(&mut self, task: &Task) -> Result<(), VickyError>;
        fn confirm_task(&mut self, task_id: Uuid) -> Result<(), VickyError>;
        fn has_task(&mut self, task_id: Uuid) -> Result<bool, VickyError>;
    }

    impl TaskDatabase for diesel::pg::PgConnection {
        fn count_all_tasks<F: Into<FilterParams>>(
            &mut self,
            task_status: Option<TaskStatus>,
            filters: F,
        ) -> Result<i64, VickyError> {
            let filters = filters.into();
            let mut tasks_count_b = tasks::table.into_boxed();

            if let Some(task_status) = task_status {
                tasks_count_b = tasks_count_b.filter(tasks::status.eq(task_status))
            }

            if let Some(group) = filters.group {
                tasks_count_b = tasks_count_b.filter(tasks::group.eq(group))
            }

            let tasks_count: i64 = tasks_count_b.count().first(self)?;

            Ok(tasks_count)
        }

        fn get_all_tasks_filtered<F: Into<FilterParams>>(
            &mut self,
            task_status: Option<TaskStatus>,
            filters: F,
        ) -> Result<Vec<Task>, VickyError> {
            let filters = filters.into();

            let mut db_tasks_build = tasks::table.into_boxed();

            if let Some(task_status) = task_status {
                db_tasks_build = db_tasks_build.filter(tasks::status.eq(task_status))
            }

            if let Some(r_limit) = filters.limit {
                db_tasks_build = db_tasks_build.limit(r_limit)
            }
            if let Some(r_offset) = filters.offset {
                db_tasks_build = db_tasks_build.offset(r_offset)
            }
            if let Some(group) = filters.group {
                db_tasks_build = db_tasks_build.filter(tasks::group.eq(group))
            }

            let db_tasks = db_tasks_build
                .order(tasks::created_at.desc())
                .load::<DbTask>(self)?;

            // prefetching all locks here, so we don't run into the N+1 Query Problem and distribute them
            let all_locks = locks::table.load::<DbLock>(self).unwrap_or_else(|_| vec![]);

            let mut lock_map: HashMap<_, Vec<DbLock>> = all_locks
                .into_iter()
                .map(|db_lock| (db_lock.task_id, db_lock))
                .into_group_map();

            let real_tasks: Vec<Task> = db_tasks
                .into_iter()
                .map(|t| {
                    let real_locks = lock_map.remove(&t.id).unwrap_or_default();

                    (t, real_locks).into()
                })
                .collect();

            Ok(real_tasks)
        }

        fn get_all_tasks(&mut self) -> Result<Vec<Task>, VickyError> {
            self.get_all_tasks_filtered(None, None)
        }

        fn get_task(&mut self, tid: Uuid) -> Result<Option<Task>, VickyError> {
            let db_task = tasks::table.filter(tasks::id.eq(tid)).first::<DbTask>(self);
            let db_task = match db_task {
                Err(diesel::result::Error::NotFound) => return Ok(None),
                _ => db_task?,
            };
            let db_locks: Vec<DbLock> = locks::table
                .filter(locks::task_id.eq(tid))
                .load::<DbLock>(self)?;

            let task = (db_task, db_locks).into();

            Ok(Some(task))
        }

        fn put_task(&mut self, task: Task) -> Result<(), VickyError> {
            self.transaction(|conn| {
                let db_locks: Vec<NewDbLock> = task
                    .locks
                    .iter()
                    .map(|l| NewDbLock::from_lock(l, task.id))
                    .collect();
                let db_task: DbTask = task.into();

                diesel::insert_into(tasks::table)
                    .values(db_task)
                    .execute(conn)?;
                diesel::insert_into(locks::table)
                    .values(db_locks)
                    .execute(conn)?;
                Ok(())
            })
        }

        fn update_task(&mut self, task: &Task) -> Result<(), VickyError> {
            diesel::update(tasks::table.filter(tasks::id.eq(task.id)))
                .set((
                    tasks::status.eq(task.status),
                    tasks::claimed_at.eq(task.claimed_at),
                    tasks::finished_at.eq(task.finished_at),
                ))
                .execute(self)?;

            // FIXME: Conversion from DbLock to Lock drops id. No way to update locks here.
            //        this is just a workaround for now. Should behave fine though
            //        and is more performant.
            if task.status == TaskStatus::Finished(TaskResult::Error) {
                diesel::update(locks::table.filter(locks::task_id.eq(task.id)))
                    .set(locks::poisoned_by_task.eq(task.id))
                    .execute(self)?;
            }

            Ok(())
        }

        fn confirm_task(&mut self, task_id: Uuid) -> Result<(), VickyError> {
            diesel::update(
                tasks::table.filter(
                    tasks::id
                        .eq(task_id)
                        .and(tasks::status.eq(TaskStatus::NeedsUserValidation)),
                ),
            )
            .set(tasks::status.eq(TaskStatus::New))
            .execute(self)?;

            Ok(())
        }

        fn has_task(&mut self, tid: Uuid) -> Result<bool, VickyError> {
            let task_count: i64 = tasks::table
                .filter(tasks::id.eq(tid))
                .count()
                .get_result(self)?;

            Ok(task_count > 0)
        }
    }
}
