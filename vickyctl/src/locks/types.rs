use crate::cli::LocksArgs;
use crate::tasks::Task;
use serde::{Deserialize, Serialize};
use vickylib::database::entities::LockKind;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoisonedLock {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: LockKind,
    pub poisoned: Task,
}

impl PoisonedLock {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn poisoned_by(&self) -> &Task {
        &self.poisoned
    }

    pub fn kind(&self) -> LockKind {
        self.kind
    }
}

pub enum LockType {
    Poisoned,
    Active,
}

impl From<&LocksArgs> for LockType {
    fn from(value: &LocksArgs) -> Self {
        match (value.poisoned, value.active) {
            (true, false) | (false, false) => LockType::Poisoned,
            (false, true) => LockType::Active,
            (_, _) => panic!("Cannot use active and poisoned flags at the same time."),
        }
    }
}
