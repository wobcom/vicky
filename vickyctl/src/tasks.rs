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

impl TaskData {
    fn to_json(&self) -> serde_json::Value {
        let locks: serde_json::Value = self
            .lock_name
            .iter()
            .zip(self.lock_type.iter())
            .map(|(name, ty)| {
                json!({
                    "name": name,
                    "type": ty
                })
            })
            .collect();
        json!({
            "display_name": self.name,
            "flake_ref": {
                "flake": self.flake_url,
                "args": self.flake_arg
            },
            "locks": locks,
            "features": self.features
        })
    }
}
