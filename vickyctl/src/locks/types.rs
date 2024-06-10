use serde::{Deserialize, Serialize};

use crate::cli::LocksArgs;
use crate::tasks::Task;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PoisonedLock {
    Write {
        id: String,
        name: String,
        poisoned: Task,
    },
    Read {
        id: String,
        name: String,
        poisoned: Task,
    },
}

impl PoisonedLock {
    pub fn id(&self) -> &str {
        match self {
            PoisonedLock::Write { id, .. } => id,
            PoisonedLock::Read { id, .. } => id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            PoisonedLock::Write { name, .. } => name,
            PoisonedLock::Read { name, .. } => name,
        }
    }

    pub fn get_poisoned_by(&self) -> &Task {
        match self {
            PoisonedLock::Write { poisoned, .. } => poisoned,
            PoisonedLock::Read { poisoned, .. } => poisoned,
        }
    }

    pub fn get_type(&self) -> &'static str {
        match self {
            PoisonedLock::Write { .. } => "WRITE",
            PoisonedLock::Read { .. } => "READ",
        }
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
