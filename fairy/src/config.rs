use rocket::serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct OIDCConfig {
    pub(crate) issuer_url: String,
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct AppConfig {
    pub(crate) vicky_url: String,
    pub(crate) vicky_external_url: String,
    pub(crate) features: Vec<String>,
    pub(crate) oidc_config: OIDCConfig,
}
