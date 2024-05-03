use crate::TasksArgs;
use log::debug;
use std::error::Error;
use crate::http_client::prepare_client;
use crate::humanize::handle_user_response;

#[allow(dead_code)]
pub fn show_tasks(tasks_args: &TasksArgs) -> Result<(), Box<dyn Error>> {
    let client = prepare_client(&tasks_args.ctx)?;
    let request = client
        .get(format!("{}/{}", tasks_args.ctx.vicky_url, "api/v1/tasks"))
        .build()?;
    let response = client.execute(request)?.error_for_status()?;

    let text = response.text()?;
    debug!("got response from server, presenting output");
    handle_user_response(&tasks_args.ctx, &text)?;
    Ok(())
}
