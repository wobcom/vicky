use log::error;
use rocket::{http::Status, response::Responder, Request};
use thiserror::Error;
use tokio::sync::broadcast::error::SendError;
use vickylib::errors::VickyError;

use crate::events::GlobalEvent;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("GlobalEvent Push Error {source:?}")]
    PushError2 {
        #[from]
        source: SendError<GlobalEvent>,
    },

    #[error("Vicky Error {source:?}")]
    VickyError {
        #[from]
        source: VickyError,
    },

    #[error("HTTP Error {0:?}")]
    HttpError(Status),

    #[error("uuid Error {source:?}")]
    Uuid {
        #[from]
        source: uuid::Error,
    },

    #[error("Migration Error {0:?}")]
    MigrationError(String),

    #[error("JWKS Error {source:?}")]
    JWKSError {
        #[from]
        source: jwtk::Error,
    },

    #[error("JWTFormatError {0:?}")]
    JWTFormatError(String),

    #[error("UserAccountError {0:?}")]
    UserAccountError(String),

    #[error("ReqwestError {source:?}")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for AppError {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'o> {
        // log `self` to your favored error tracker, e.g.
        // sentry::capture_error(&self);
        error!("Error: {}", self);

        match self {
            Self::HttpError(x) => x.respond_to(req),
            _ => Status::InternalServerError.respond_to(req),
        }
    }
}
