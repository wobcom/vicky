use crate::database::entities::lock::db_impl::DbLock;
use crate::database::entities::lock::Lock;
use crate::database::entities::task::db_impl::DbTask;
use chrono::naive::serde::ts_seconds;
use chrono::naive::serde::ts_seconds_option;
use chrono::{NaiveDateTime, Utc};
use delegate::delegate;
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

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: Uuid,
    pub display_name: String,
    pub status: TaskStatus,
    pub locks: Vec<Lock>,
    pub flake_ref: FlakeRef,
    pub features: Vec<String>,
    #[serde(with = "ts_seconds")]
    pub created_at: NaiveDateTime,
    #[serde(with = "ts_seconds_option")]
    pub claimed_at: Option<NaiveDateTime>,
    #[serde(with = "ts_seconds_option")]
    pub finished_at: Option<NaiveDateTime>,
}

impl AsRef<Task> for Task {
    fn as_ref(&self) -> &Task {
        self
    }
}

impl Task {
    pub fn builder() -> TaskBuilder {
        TaskBuilder::default()
    }
}

impl TryFrom<TaskBuilder> for Task {
    type Error = TaskBuilder;

    fn try_from(value: TaskBuilder) -> Result<Self, Self::Error> {
        value.build()
    }
}

#[derive(Clone, Debug)]
pub struct TaskBuilder {
    id: Option<Uuid>,
    display_name: Option<String>,
    status: TaskStatus,
    locks: Vec<Lock>,
    flake_ref: FlakeRef,
    features: Vec<String>,
}

impl Default for TaskBuilder {
    fn default() -> Self {
        TaskBuilder {
            id: None,
            display_name: None,
            status: TaskStatus::New,
            locks: Vec::new(),
            flake_ref: FlakeRef {
                flake: "".to_string(),
                args: Vec::new(),
            },
            features: Vec::new(),
        }
    }
}

impl TaskBuilder {
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_display_name<S: Into<String>>(mut self, display_name: S) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_read_lock<S: Into<String>>(mut self, name: S) -> Self {
        self.locks.push(Lock::read(name));
        self
    }

    pub fn with_write_lock<S: Into<String>>(mut self, name: S) -> Self {
        self.locks.push(Lock::write(name));
        self
    }

    pub fn with_locks(mut self, locks: Vec<Lock>) -> Self {
        self.locks = locks;
        self
    }

    pub fn with_flake<S: Into<FlakeURI>>(mut self, flake_uri: S) -> Self {
        self.flake_ref.flake = flake_uri.into();
        self
    }

    pub fn with_flake_arg<S: Into<String>>(mut self, flake_arg: S) -> Self {
        self.flake_ref.args.push(flake_arg.into());
        self
    }

    pub fn with_flake_args(mut self, args: Vec<String>) -> Self {
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

    delegate! {
        to self {
            #[field]
            pub fn id(&self) -> Option<Uuid>;
            #[field]
            #[expr($.as_ref())]
            pub fn display_name(&self) -> Option<&String>;
            #[field]
            pub fn status(&self) -> TaskStatus;
            #[field(&)]
            pub fn locks(&self) -> &[Lock];
            #[field(&)]
            pub fn flake_ref(&self) -> &FlakeRef;
            #[field(&)]
            pub fn features(&self) -> &[String];
        }
    }

    pub fn check_lock_conflict(&self) -> bool {
        self.locks
            .iter()
            .tuple_combinations()
            .any(|(a, b)| a.is_conflicting(b))
    }

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

    fn _build_unchecked(self) -> Task {
        Task {
            id: self.id.unwrap_or_else(Uuid::new_v4),
            display_name: self.display_name.unwrap_or_else(|| "Task".to_string()),
            features: self.features,
            status: self.status,
            locks: self.locks,
            flake_ref: self.flake_ref,
            created_at: Utc::now().naive_utc(),
            claimed_at: None,
            finished_at: None,
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
    }

    impl Display for TaskStatus {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let str = match self {
                TaskStatus::NeedsUserValidation => "NEEDS_USER_VALIDATION",
                TaskStatus::New => "NEW",
                TaskStatus::Running => "RUNNING",
                TaskStatus::Finished(r) => match r {
                    TaskResult::Success => "FINISHED::SUCCESS",
                    TaskResult::Error => "FINISHED::ERROR",
                },
            };
            write!(f, "{str}")
        }
    }

    impl TryFrom<&str> for TaskStatus {
        type Error = &'static str;

        fn try_from(str: &str) -> Result<Self, Self::Error> {
            match str {
                "NEEDS_USER_VALIDATION" => Ok(TaskStatus::NeedsUserValidation),
                "NEW" => Ok(TaskStatus::New),
                "RUNNING" => Ok(TaskStatus::Running),
                "FINISHED::SUCCESS" => Ok(TaskStatus::Finished(TaskResult::Success)),
                "FINISHED::ERROR" => Ok(TaskStatus::Finished(TaskResult::Error)),
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
            }
        }
    }

    pub trait TaskDatabase {
        fn count_all_tasks(&mut self, task_status: Option<TaskStatus>) -> Result<i64, VickyError>;
        fn get_all_tasks_filtered(
            &mut self,
            task_status: Option<TaskStatus>,
            filter_params: Option<FilterParams>,
        ) -> Result<Vec<Task>, VickyError>;
        fn get_all_tasks(&mut self) -> Result<Vec<Task>, VickyError>;
        fn get_task(&mut self, task_id: Uuid) -> Result<Option<Task>, VickyError>;
        fn put_task(&mut self, task: Task) -> Result<(), VickyError>;
        fn update_task(&mut self, task: &Task) -> Result<(), VickyError>;
        fn confirm_task(&mut self, task_id: Uuid) -> Result<(), VickyError>;
        fn has_task(&mut self, task_id: Uuid) -> Result<bool, VickyError>;
    }

    impl TaskDatabase for diesel::pg::PgConnection {
        fn count_all_tasks(&mut self, task_status: Option<TaskStatus>) -> Result<i64, VickyError> {
            let mut tasks_count_b = tasks::table.into_boxed();

            if let Some(task_status) = task_status {
                tasks_count_b = tasks_count_b.filter(tasks::status.eq(task_status))
            }

            let tasks_count: i64 = tasks_count_b.count().first(self)?;

            Ok(tasks_count)
        }

        fn get_all_tasks_filtered(
            &mut self,
            task_status: Option<TaskStatus>,
            filter_params: Option<FilterParams>,
        ) -> Result<Vec<Task>, VickyError> {
            let mut db_tasks_build = tasks::table.into_boxed();

            if let Some(task_status) = task_status {
                db_tasks_build = db_tasks_build.filter(tasks::status.eq(task_status))
            }

            let limit = filter_params.clone().and_then(|x| x.limit);
            let offset = filter_params.clone().and_then(|x| x.offset);

            if let Some(r_limit) = limit {
                db_tasks_build = db_tasks_build.limit(r_limit)
            }
            if let Some(r_offset) = offset {
                db_tasks_build = db_tasks_build.offset(r_offset)
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
