use crate::database::entities::lock::Lock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::database::entities::lock::db_impl::DbLock;
use crate::database::entities::task::db_impl::DbTask;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result")]
pub enum TaskResult {
    Success,
    Error,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state")]
pub enum TaskStatus {
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
}

impl Task {
    pub fn builder() -> TaskBuilder {
        TaskBuilder::default()
    }
}

impl From<TaskBuilder> for Task {
    fn from(builder: TaskBuilder) -> Self {
        builder.build()
    }
}

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
        self.locks.push(Lock::Read { name: name.into(), poisoned: None });
        self
    }

    pub fn with_write_lock<S: Into<String>>(mut self, name: S) -> Self {
        self.locks.push(Lock::Write { name: name.into(), poisoned: None });
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

    pub fn id(&self) -> Option<Uuid> {
        self.id
    }

    pub fn display_name(&self) -> &Option<String> {
        &self.display_name
    }

    pub fn status(&self) -> &TaskStatus {
        &self.status
    }

    pub fn locks(&self) -> &Vec<Lock> {
        &self.locks
    }

    pub fn flake_ref(&self) -> &FlakeRef {
        &self.flake_ref
    }

    pub fn features(&self) -> &Vec<String> {
        &self.features
    }

    pub fn build(self) -> Task {
        Task {
            id: self.id.unwrap_or_else(Uuid::new_v4),
            display_name: self.display_name.unwrap_or_else(|| "Task".to_string()),
            features: self.features,
            status: self.status,
            locks: self.locks,
            flake_ref: self.flake_ref,
        }
    }
}

impl From<(DbTask, Vec<DbLock>)> for Task {
    fn from(value: (DbTask, Vec<DbLock>)) -> Self {
        let (task, locks) = value;
        Task {
            id: task.id,
            display_name: task.display_name,
            status: task.status.as_str().try_into().expect("Database corrupted"),
            locks: locks.into_iter().map(Lock::from).collect(),
            flake_ref: FlakeRef {
                flake: task.flake_ref_uri,
                args: task.flake_ref_args,
            },
            features: task.features,
        }
    }
}

// this was on purpose because these macro-generated entity types
// mess up the whole namespace and HAVE to be scoped
pub mod db_impl {
    use crate::database::entities::task::{Task, TaskResult, TaskStatus};
    use crate::errors::VickyError;
    use diesel::{insert_into, update, AsChangeset, ExpressionMethods, Insertable, QueryDsl, Queryable, RunQueryDsl, Connection};
    use std::collections::HashMap;
    use std::fmt::Display;
    use uuid::Uuid;
    // these here are evil >:(
    use crate::database::entities::lock::db_impl::DbLock;
    use crate::database::schema::locks;
    use crate::database::schema::tasks;
    use itertools::Itertools;
    use serde::Serialize;
    use crate::database::schema::locks::task_id;

    #[derive(Insertable, Queryable, AsChangeset, Debug, Serialize)]
    #[diesel(table_name = tasks)]
    pub struct DbTask {
        pub id: Uuid,
        pub display_name: String,
        pub status: String,
        pub features: Vec<String>,
        pub flake_ref_uri: String,
        pub flake_ref_args: Vec<String>,
    }

    impl Display for TaskStatus {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let str = match self {
                TaskStatus::New => "NEW",
                TaskStatus::Running => "RUNNING",
                TaskStatus::Finished(r) => match r {
                    TaskResult::Success => "FINISHED::SUCCESS",
                    TaskResult::Error => "FINISHED::ERROR",
                },
            };
            write!(f, "{}", str)
        }
    }

    impl TryFrom<&str> for TaskStatus {
        type Error = &'static str;

        fn try_from(str: &str) -> Result<Self, Self::Error> {
            match str {
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
                status: task.status.to_string(),
                features: task.features,
                flake_ref_uri: task.flake_ref.flake,
                flake_ref_args: task.flake_ref.args,
            }
        }
    }

    pub trait TaskDatabase {
        fn get_all_tasks(&mut self) -> Result<Vec<Task>, VickyError>;
        fn get_task(&mut self, task_id: Uuid) -> Result<Option<Task>, VickyError>;
        fn put_task(&mut self, task: Task) -> Result<(), VickyError>;
        fn update_task(&mut self, task: &Task) -> Result<(), VickyError>;
    }

    impl TaskDatabase for diesel::pg::PgConnection {
        fn get_all_tasks(&mut self) -> Result<Vec<Task>, VickyError> {
            let db_tasks = tasks::table.load::<DbTask>(self)?;

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

        fn get_task(&mut self, tid: Uuid) -> Result<Option<Task>, VickyError> {
            let db_task = tasks::table.filter(tasks::id.eq(tid)).first::<DbTask>(self);
            let db_task = match db_task {
                Err(diesel::result::Error::NotFound) => return Ok(None),
                _ => db_task?,
            };
            let db_locks: Vec<DbLock> = locks::table.filter(task_id.eq(task_id)).load::<DbLock>(self)?;

            let task = (db_task, db_locks).into();

            Ok(Some(task))
        }

        fn put_task(&mut self, task: Task) -> Result<(), VickyError> {
            self.transaction(|conn| {
                let db_locks: Vec<DbLock> = task
                    .locks
                    .iter()
                    .map(|l| DbLock::from_lock(l, task.id))
                    .collect();
                let db_task: DbTask = task.into();
                
                insert_into(tasks::table).values(db_task).execute(conn)?;
                for mut db_lock in db_locks {
                    db_lock.id = None;
                    insert_into(locks::table).values(db_lock).execute(conn)?;
                }
                Ok(())
            })
        }

        fn update_task(&mut self, task: &Task) -> Result<(), VickyError> {
            update(tasks::table.filter(tasks::id.eq(task.id)))
                .set(tasks::status.eq(task.status.clone().to_string()))
                .execute(self)?;

            // FIXME: Conversion from DbLock to Lock drops id. No way to update locks here.
            //        this is just a workaround for now. Should behave fine though 
            //        and is more performant.
            if task.status == TaskStatus::Finished(TaskResult::Error) {
                update(locks::table.filter(task_id.eq(task.id)))
                    .set(locks::poisoned_by_task.eq(task.id))
                    .execute(self)?;
            }
            
            Ok(())
        }
    }
}
