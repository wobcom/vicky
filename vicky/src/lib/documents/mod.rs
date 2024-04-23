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
    WRITE { object: String },
    READ { object: String },
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
