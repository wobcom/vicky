use crate::AppContext;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION};
use crate::error::Error;

pub fn prepare_client(ctx: &AppContext) -> Result<Client, Error> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert(AUTHORIZATION, ctx.vicky_token.parse().unwrap());
    let client = Client::builder()
        .default_headers(default_headers)
        .user_agent(format!("vickyctl/{}", env!("CARGO_PKG_VERSION")))
        .build()?;
    Ok(client)
}
