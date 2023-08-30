
use serde::{Serialize, Deserialize};

use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TaskResult {
    SUCCESS,
    ERROR,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    NEW,
    RUNNING,
    FINISHED(TaskResult)

}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub status: TaskStatus,
}
