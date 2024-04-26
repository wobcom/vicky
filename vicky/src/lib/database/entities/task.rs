use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::database::entities::lock::Lock;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result")]
pub enum TaskResult {
    SUCCESS,
    ERROR,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state")]
pub enum TaskStatus {
    NEW,
    RUNNING,
    FINISHED(TaskResult),
}

type FlakeURI = String;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlakeRef {
    pub flake: FlakeURI,
    pub args: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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
            status: TaskStatus::NEW,
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
        self.locks.push(Lock::READ { name: name.into() });
        self
    }

    pub fn with_write_lock<S: Into<String>>(mut self, name: S) -> Self {
        self.locks.push(Lock::WRITE { name: name.into() });
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

// this was on purpose because these macro-generated entity types
// mess up the whole namespace and HAVE to be scoped
pub mod db_impl {
    use crate::database::entities::task::{Task, TaskResult, TaskStatus};
    use crate::errors::VickyError;
    use async_trait::async_trait;
    use diesel::{Insertable, Queryable, Selectable};
    use uuid::Uuid;
    use crate::database::entities::lock::Lock;
    // these here are evil >:(
    use crate::database::schema::locks;
    use crate::database::schema::tasks;

    #[derive(Insertable, Queryable)]
    #[diesel(table_name = tasks)]
    struct DbTask {
        pub id: Uuid,
        pub display_name: Option<String>,
        pub status: Option<String>,
        pub flake_ref_uri: Option<String>,
        pub flake_ref_args: Option<String>,
    }

    impl ToString for TaskStatus {
        fn to_string(&self) -> String {
            match self {
                TaskStatus::NEW => "NEW",
                TaskStatus::RUNNING => "RUNNING",
                TaskStatus::FINISHED(r) => match r {
                    TaskResult::SUCCESS => "FINISHED::SUCCESS",
                    TaskResult::ERROR => "FINISHED::ERROR",
                },
            }.to_string()
        }
    }

    impl Into<DbTask> for Task {
        fn into(self) -> DbTask {
            DbTask {
                id: self.id,
                display_name: Some(self.display_name),
                status: Some(self.status.to_string()),
                flake_ref_uri: Some(self.flake_ref.flake),
                flake_ref_args: Some(self.flake_ref.args.join("||")),
            }
        }
    }

    #[derive(Insertable, Queryable)]
    #[diesel(table_name = locks)]
    struct DbLock {
        id: Option<i32>,
        task_id: Uuid,
        name: String,
        type_: String,
    }

    impl DbLock {
        fn from_lock(lock: Lock, task_id: Uuid) -> Self {
            match lock {
                Lock::WRITE { name } => DbLock { id: None, task_id, name, type_: "WRITE".to_string() },
                Lock::READ { name } => DbLock { id: None, task_id, name, type_: "READ".to_string() },
            }
        }
    }

    impl Into<Lock> for DbLock {
        fn into(self) -> Lock {
            match self.type_.as_str() {
                "WRITE" => Lock::WRITE { name: self.name },
                "READ" => Lock::READ { name: self.name },
                _ => panic!(
                    "Can't parse lock from database lock. Database corrupted? \
                Expected READ or WRITE but found {} as type at key {}.",
                    self.type_,
                    self.id.unwrap_or(-1)
                ),
            }
        }
    }

    #[async_trait]
    pub trait TaskDatabase {
        async fn get_all_tasks(&mut self) -> Result<Vec<Task>, VickyError>;
        async fn get_task(&self, task_id: Uuid) -> Result<Option<Task>, VickyError>;
        async fn put_task(&mut self, task: &Task) -> Result<(), VickyError>;
    }

    impl TaskDatabase for diesel::pg::PgConnection {
        async fn get_all_tasks(mut self) -> Result<Vec<Task>, VickyError> {
            // very evil >>:(
            use self::tasks::dsl::*;
            
            todo!()
        }

        async fn get_task(&self, task_id: Uuid) -> Result<Option<Task>, VickyError> {
            // so evil >:O
            use self::tasks::dsl::*;

            todo!();
        }

        async fn put_task(&mut self, task: &Task) -> Result<(), VickyError> {
            // even more evil >;(
            use self::tasks::dsl::*;
            
            todo!();
        }
    }
}
