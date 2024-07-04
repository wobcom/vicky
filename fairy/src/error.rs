use thiserror::Error;

#[derive(Error, Debug)]
pub enum FairyError {
    #[error("Could not authenticate against OIDC provider")]
    OpenId,
}
