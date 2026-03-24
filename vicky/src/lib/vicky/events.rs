use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GlobalEvent {
    TaskAdd,
    TaskUpdate { uuid: uuid::Uuid },
}
