use async_trait::async_trait;
use etcd_client::GetOptions;
use serde::{Deserialize, Serialize};

use uuid::Uuid;

use crate::{errors::VickyError, etcd::client::ClientExt};

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Lock {
    WRITE { name: String },
    READ { name: String },
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

#[async_trait]
pub trait DocumentClient {
    async fn get_all_tasks(&self) -> Result<Vec<Task>, VickyError>;
    async fn get_task(&self, task_id: Uuid) -> Result<Option<Task>, VickyError>;
    async fn put_task(&self, task: &Task) -> Result<(), VickyError>;
}

#[async_trait]
impl DocumentClient for etcd_client::Client {
    async fn get_all_tasks(&self) -> Result<Vec<Task>, VickyError> {
        let mut kv = self.kv_client();
        let get_options: GetOptions = GetOptions::new().with_prefix().with_sort(
            etcd_client::SortTarget::Create,
            etcd_client::SortOrder::Descend,
        );
        let tasks: Vec<Task> = kv
            .get_yaml_list(
                "vicky.wobcom.de/task/manifest".to_string(),
                Some(get_options),
            )
            .await?;
        Ok(tasks)
    }

    async fn get_task(&self, task_id: Uuid) -> Result<Option<Task>, VickyError> {
        let mut kv = self.kv_client();
        let key = format!("vicky.wobcom.de/task/manifest/{}", task_id);
        let task: Option<Task> = kv.get_yaml(key.clone(), None).await?;
        Ok(task)
    }

    async fn put_task(&self, task: &Task) -> Result<(), VickyError> {
        let mut kv = self.kv_client();
        let key = format!("vicky.wobcom.de/task/manifest/{}", task.id);
        kv.put_yaml(key, &task, None).await?;
        Ok(())
    }
}
