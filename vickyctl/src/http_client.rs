use crate::cli::AppContext;
use crate::error::Error;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap};
use yansi::Paint;

pub fn prepare_client(ctx: &AppContext) -> Result<Client, Error> {
    let mut default_headers = HeaderMap::new();
    let auth_header = ctx
        .vicky_token
        .parse()
        .map_err(|_| Error::Custom("VICKY_TOKEN is not a valid header value"))?;
    default_headers.insert(AUTHORIZATION, auth_header);
    let client = Client::builder()
        .default_headers(default_headers)
        .user_agent(format!("vickyctl/{}", env!("CARGO_PKG_VERSION")))
        .build()?;
    Ok(client)
}

pub fn print_http(status: Option<StatusCode>, msg: &str) {
    println!("{}", format_http_msg(status, msg));
}

pub fn format_http_msg(status: Option<StatusCode>, msg: &str) -> String {
    let prefix = if let Some(code) = status {
        if code.is_informational() {
            code.resetting()
        } else if code.is_redirection() {
            code.yellow()
        } else if code.is_success() {
            code.bright_green()
        } else {
            code.bright_red()
        }
        .bold()
        .to_string()
    } else {
        "HTTP Send Error".bold().bright_red().to_string()
    };
    format!("[ {prefix} ] {msg}")
}
