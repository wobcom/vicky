use ratatui::widgets::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppContext, humanize, LocksArgs};
use crate::error::Error;
use crate::http_client::prepare_client;

// TODO: REFACTOR EVERYTHING

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result")]
pub enum TaskResult {
    Success,
    Error,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state")]
pub enum TaskStatus {
    New,
    Running,
    Finished(TaskResult),
}

type FlakeURI = String;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlakeRef {
    pub flake: FlakeURI,
    pub args: Vec<String>,
}

type Maow = u8; // this does not exist. look away. it's all for a reason.

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: Uuid,
    pub display_name: String,
    pub status: TaskStatus,
    pub locks: Vec<Maow>,
    pub flake_ref: FlakeRef,
    pub features: Vec<String>,
}

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

impl<'a> From<&'a PoisonedLock> for Row<'a> {
    fn from(value: &'a PoisonedLock) -> Self {
        let poisoned_by = value.get_poisoned_by();
        let task_name = poisoned_by.display_name.as_str();
        let name = value.name();
        let ty = value.get_type();
        let uri = poisoned_by.flake_ref.flake.as_str();
        Row::new(vec![name, ty, task_name, uri])
    }
}

enum LockType {
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

fn get_locks_endpoint(lock_type: LockType, detailed: bool) -> &'static str {
    match lock_type {
        LockType::Poisoned => match detailed {
            false => "api/v1/locks/poisoned",
            true => "api/v1/locks/poisoned_detailed",
        },
        LockType::Active => "api/v1/locks/active",
    }
}

fn fetch_locks_raw(ctx: &AppContext, lock_type: LockType, detailed: bool) -> Result<String, Error> {
    let client = prepare_client(ctx)?;
    let request = client
        .get(format!(
            "{}/{}",
            ctx.vicky_url,
            get_locks_endpoint(lock_type, detailed)
        ))
        .build()?;
    let response = client.execute(request)?.error_for_status()?;

    let locks = response.text()?;
    Ok(locks)
}

pub(crate) fn fetch_detailed_poisoned_locks(ctx: &AppContext) -> Result<Vec<PoisonedLock>, Error> {
    let raw_locks = fetch_locks_raw(ctx, LockType::Poisoned, true)?;
    let locks: Vec<PoisonedLock> = serde_json::from_str(&raw_locks)?;
    Ok(locks)
}

pub fn show_locks(locks_args: &LocksArgs) -> Result<(), Error> {
    if locks_args.ctx.humanize {
        humanize::ensure_jless("lock")?;
    }
    if locks_args.active && locks_args.poisoned {
        return Err(Error::Custom(
            "Cannot use active and poisoned lock type at the same time.",
        ));
    }

    let locks_json = fetch_locks_raw(&locks_args.ctx, LockType::from(locks_args), false)?;

    humanize::handle_user_response(&locks_args.ctx, &locks_json)?;
    Ok(())
}
