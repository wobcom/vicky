use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Lock {
    WRITE { name: String },
    READ { name: String },
}

impl Lock {
    pub fn is_conflicting(&self, other: &Lock) -> bool {
        match (self, other) {
            (Lock::WRITE { name: name1 }, Lock::WRITE { name: name2 })
            | (Lock::READ { name: name1 }, Lock::WRITE { name: name2 })
            | (Lock::WRITE { name: name1 }, Lock::READ { name: name2 }) => name1 == name2,
            _ => false,
        }
    }
}
