use crate::{humanize, LocksArgs};
use crate::error::Error;
use crate::http_client::prepare_client;

pub fn get_locks_endpoint(locks_args: &LocksArgs) -> &'static str {
    if locks_args.active {
        "api/v1/locks/active"
    } else {
        "api/v1/locks/poisoned"
    }
}

pub fn show_locks(locks_args: &LocksArgs) -> Result<(), Error> {
    if locks_args.ctx.humanize {
        humanize::ensure_jless("lock")?;
    }

    let client = prepare_client(&locks_args.ctx)?;
    let request = client
        .get(format!(
            "{}/{}",
            locks_args.ctx.vicky_url, get_locks_endpoint(locks_args) 
        ))
        .build()?;
    let response = client.execute(request)?.error_for_status()?;

    let text = response.text()?;
    humanize::handle_user_response(&locks_args.ctx, &text)?;
    Ok(())
}
