use aws_sdk_s3::{
    operation::{get_object::GetObjectError, put_object::PutObjectError},
    primitives::ByteStreamError,
};
use log::error;
use rocket::{http::Status, response::Responder, Request};
use thiserror::Error;
use tokio::sync::broadcast::error::SendError;

#[derive(Error, Debug)]
pub enum VickyError {
    #[error("serde_json Error {source:?}")]
    SerdeJson {
        #[from]
        source: serde_json::Error,
    },

    #[error("serde_yaml Error {source:?}")]
    SerdeYaml {
        #[from]
        source: serde_yaml::Error,
    },
    #[error("etcd Error {source:?}")]
    EtcdClient {
        #[from]
        source: etcd_client::Error,
    },

    #[error("Scheduling Error {source:?}")]
    Scheduler {
        #[from]
        source: SchedulerError,
    },

    #[error("Log Push Error {source:?}")]
    PushError {
        #[from]
        source: SendError<(String, String)>,
    },

    #[error("S3 Client Error {source:?}")]
    S3ClientError {
        #[from]
        source: S3ClientError,
    },
}

#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("Invalid Scheduling")]
    GeneralSchedulingError,
}

#[derive(Error, Debug)]
pub enum S3ClientError {
    #[error("Object Already Exists")]
    ObjectAlreadyExistsError,

    #[error(transparent)]
    SdkError {
        #[from]
        source: aws_sdk_s3::Error,
    },

    #[error(transparent)]
    SdkPutObjectError {
        #[from]
        source: aws_sdk_s3::error::SdkError<PutObjectError>,
    },

    #[error(transparent)]
    SdkGetObjectError {
        #[from]
        source: aws_sdk_s3::error::SdkError<GetObjectError>,
    },

    #[error(transparent)]
    ByteStreamError {
        #[from]
        source: ByteStreamError,
    },
}

impl<'r, 'o: 'r> Responder<'r, 'o> for VickyError {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'o> {
        // log `self` to your favored error tracker, e.g.
        // sentry::capture_error(&self);
        error!("Error: {}", self);

        Status::InternalServerError.respond_to(req)
    }
}
