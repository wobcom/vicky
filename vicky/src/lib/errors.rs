use aws_sdk_s3::{
    operation::{get_object::GetObjectError, put_object::PutObjectError},
    primitives::ByteStreamError,
};
use log::error;
use rocket::{http::Status, response::Responder, Request};
use thiserror::Error;
use tokio::sync::broadcast::error::SendError;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum VickyError {
    #[error("json error: {source}")]
    SerdeJson {
        #[from]
        source: serde_json::Error,
    },

    #[error("database error: {source}")]
    Diesel {
        #[from]
        source: diesel::result::Error,
    },

    #[error("scheduler error: {source}")]
    Scheduler {
        #[from]
        source: SchedulerError,
    },

    #[error("log broadcast failed: {source}")]
    PushError {
        #[from]
        source: SendError<(Uuid, String)>,
    },

    #[error("s3 client error: {source}")]
    S3ClientError {
        #[from]
        source: Box<S3ClientError>,
    },
}

#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("no valid schedule")]
    GeneralSchedulingError,
    #[error("lock already owned")]
    LockAlreadyOwnedError,
}

#[derive(Error, Debug)]
pub enum S3ClientError {
    #[error("object already exists")]
    ObjectAlreadyExistsError,

    #[error(transparent)]
    SdkError { source: Box<aws_sdk_s3::Error> },

    #[error(transparent)]
    SdkPutObjectError {
        source: Box<aws_sdk_s3::error::SdkError<PutObjectError>>,
    },

    #[error(transparent)]
    SdkGetObjectError {
        source: Box<aws_sdk_s3::error::SdkError<GetObjectError>>,
    },

    #[error(transparent)]
    ByteStreamError {
        #[from]
        source: ByteStreamError,
    },

    #[error("invalid utf8 in log object: {source}")]
    Utf8 {
        #[from]
        source: std::string::FromUtf8Error,
    },
}

impl<'r, 'o: 'r> Responder<'r, 'o> for VickyError {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'o> {
        // log `self` to your favored error tracker, e.g.
        // sentry::capture_error(&self);
        error!("Error: {self}");

        Status::InternalServerError.respond_to(req)
    }
}

impl From<S3ClientError> for VickyError {
    fn from(source: S3ClientError) -> Self {
        VickyError::S3ClientError {
            source: Box::new(source),
        }
    }
}

impl From<aws_sdk_s3::Error> for S3ClientError {
    fn from(source: aws_sdk_s3::Error) -> Self {
        S3ClientError::SdkError {
            source: Box::new(source),
        }
    }
}

impl From<aws_sdk_s3::error::SdkError<PutObjectError>> for S3ClientError {
    fn from(source: aws_sdk_s3::error::SdkError<PutObjectError>) -> Self {
        S3ClientError::SdkPutObjectError {
            source: Box::new(source),
        }
    }
}

impl From<aws_sdk_s3::error::SdkError<GetObjectError>> for S3ClientError {
    fn from(source: aws_sdk_s3::error::SdkError<GetObjectError>) -> Self {
        S3ClientError::SdkGetObjectError {
            source: Box::new(source),
        }
    }
}
