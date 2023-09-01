
use serde::{Serialize, Deserialize};

use uuid::Uuid;

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
    FINISHED(TaskResult)

}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Lock {
    WRITE {
        object: String
    },
    READ {
        object: String
    },
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
}



