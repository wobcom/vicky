use crate::AuthState;
use crate::error::Error;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION};
use reqwest::StatusCode;
use yansi::Paint;

pub fn prepare_client(auth_state: &AuthState) -> Result<(Client, String), Error> {

    let base_url: String;
    let auth_token: String = "".to_owned();

    match auth_state {
        AuthState::EnvironmentAuthenticated(envConfig) => {
            base_url = envConfig.url.clone();
        },
        AuthState::FileAuthenticated(fileCfg) => {
            base_url = fileCfg.vicky_url.clone();
        },
        AuthState::Unauthenticated => {
            return Err(Error::Unauthenticated())
        },
    }

    let mut default_headers = HeaderMap::new();
    default_headers.insert(AUTHORIZATION, auth_token.parse().unwrap());
    let client = Client::builder()
        .default_headers(default_headers)
        .user_agent(format!("vickyctl/{}", env!("CARGO_PKG_VERSION")))
        .build()?;
    Ok((client, base_url))
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
