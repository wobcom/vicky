use std::io::{ErrorKind, Write};
use std::process::{Command, Stdio};

use log::debug;
use which::which;
use crate::cli::AppContext;

use crate::error::Error;

pub fn ensure_jless(needed_for: &str) -> Result<(), Error> {
    which("jless")
        .map_err(|_| {
            Error::Dependency(
                "jless".to_string(),
                format!("the --humanize flag to work with `{needed_for}`"),
            )
        })
        .map(|_| ())
}

pub fn handle_user_response(ctx: &AppContext, json: &str) -> Result<(), Error> {
    let data: serde_json::Value = serde_json::from_str(json)?;
    let pretty_json = serde_json::to_string_pretty(&data)?;
    if ctx.humanize {
        humanize(&pretty_json)?;
    } else {
        println!("{pretty_json}");
    }
    Ok(())
}
pub fn humanize(text: &str) -> Result<(), Error> {
    debug!("spawning `jless` as a child process for human data view");
    let mut child = Command::new("jless")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    child
        .stdin
        .as_ref()
        .ok_or_else(|| {
            std::io::Error::new(
                ErrorKind::BrokenPipe,
                "Could not take stdin pipe from jless",
            )
        })?
        .write_all(text.as_bytes())?;
    child.wait()?;
    Ok(())
}
