use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

// TODO: TEMPORARY!! you know what has to be done... stop wasting time and do it

#[derive(Debug, Deserialize)]
pub struct FlakeRef {
    pub flake: String,
    pub args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "result")]
pub enum TaskResult {
    Success,
    Error,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "state")]
pub enum TaskStatus {
    New,
    Running,
    Finished(TaskResult),
}

#[derive(Debug, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub display_name: String,
    pub status: TaskStatus,
    pub flake_ref: FlakeRef,
}
