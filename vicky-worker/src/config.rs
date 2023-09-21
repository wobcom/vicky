use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) vicky_url: String,
    pub(crate) machine_token: String,
}
