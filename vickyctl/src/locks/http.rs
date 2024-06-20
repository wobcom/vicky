use reqwest::blocking::Client;

use crate::error::Error;
use crate::locks::types::{LockType, PoisonedLock};

pub fn get_locks_endpoint(lock_type: LockType, detailed: bool) -> &'static str {
    match lock_type {
        LockType::Poisoned => match detailed {
            false => "api/v1/locks/poisoned",
            true => "api/v1/locks/poisoned_detailed",
        },
        LockType::Active => "api/v1/locks/active",
    }
}

pub fn fetch_locks_raw(
    client: &Client, 
    vicky_url: String,
    lock_type: LockType,
    detailed: bool,
) -> Result<String, Error> {
    let request = client
        .get(format!(
            "{}/{}",
            vicky_url,
            get_locks_endpoint(lock_type, detailed)
        ))
        .build()?;
    let response = client.execute(request)?.error_for_status()?;

    let locks = response.text()?;
    Ok(locks)
}

pub fn fetch_detailed_poisoned_locks(client: &Client, vicky_url: String) -> Result<Vec<PoisonedLock>, Error> {
    let raw_locks = fetch_locks_raw(client, vicky_url, LockType::Poisoned, true)?;
    let locks: Vec<PoisonedLock> = serde_json::from_str(&raw_locks)?;
    Ok(locks)
}

pub fn unlock_lock(client: &Client, vicky_url: String, lock_to_clear: &PoisonedLock) -> Result<(), Error> {
    let request = client
        .patch(format!(
            "{}/api/v1/locks/unlock/{}",
            vicky_url,
            lock_to_clear.id()
        ))
        .build()?;
    client.execute(request)?.error_for_status()?;
    Ok(())
}
