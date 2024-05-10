use crate::AppContext;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION};
use std::error::Error;

pub fn prepare_client(ctx: &AppContext) -> Result<Client, Box<dyn Error>> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert(AUTHORIZATION, ctx.vicky_token.parse().unwrap());
    let client = Client::builder()
        .default_headers(default_headers)
        // TODO?: .https_only(true)
        .user_agent(format!("VickyCTL/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("HTTP Client could not be built");
    Ok(client)
}
