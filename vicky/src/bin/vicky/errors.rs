use log::error;
use rocket::{http::Status, response::Responder, Request};
use thiserror::Error;
use tokio::sync::broadcast::error::SendError;
use vickylib::errors::VickyError;

use crate::events::GlobalEvent;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("broadcast failed: {source}")]
    PushError2 {
        #[from]
        source: SendError<GlobalEvent>,
    },

    #[error("service error: {source}")]
    VickyError {
        #[from]
        source: Box<VickyError>,
    },

    #[error("http {0}")]
    HttpError(Status),

    #[error("bad uuid: {source}")]
    Uuid {
        #[from]
        source: uuid::Error,
    },

    #[error("migration failed: {0}")]
    MigrationError(String),

    #[error("jwks check failed: {source}")]
    JWKSError {
        #[from]
        source: jwtk::Error,
    },

    #[error("invalid jwt: {0}")]
    JWTFormatError(String),

    #[error("reqwest error: {source}")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
    },
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

impl From<VickyError> for AppError {
    fn from(source: VickyError) -> Self {
        AppError::VickyError {
            source: Box::new(source),
        }
    }
}

impl From<vickylib::errors::S3ClientError> for AppError {
    fn from(source: vickylib::errors::S3ClientError) -> Self {
        AppError::VickyError {
            source: Box::new(source.into()),
        }
    }
}
