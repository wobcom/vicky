use aws_sdk_s3::{operation::{upload_part::UploadPartError, complete_multipart_upload::CompleteMultipartUploadError, put_object::PutObjectError, get_object::GetObjectError}, primitives::ByteStreamError};
use log::error;
use rocket::{response::Responder, Request, http::Status};
use thiserror::Error;
use tokio::sync::broadcast::error::SendError;

#[derive(Error, Debug)]
pub enum VickyError {
    #[error("serde_json Error {source:?}")]
    SerdeJson {
        #[from] source: serde_json::Error,
    },

    #[error("serde_yaml Error {source:?}")]
    SerdeYaml {
        #[from] source: serde_yaml::Error,
    },
    #[error("etcd Error {source:?}")]
    EtcdClient {
        #[from] source: etcd_client::Error,
    },

    #[error("uuid Error {source:?}")]
    Uuid {
        #[from] source: uuid::Error,
    },

    #[error("HTTP Error {0:?}")]
    HttpError(Status),

    #[error("Scheduling Error {source:?}")]
    Scheduler {
        #[from] source: SchedulerError,
    },

    #[error("Push Error {source:?}")]
    PushError {
        #[from] source: SendError<(String, String)>,
    },

    #[error("S3 Client Error {source:?}")]
    S3ClientError {
        #[from] source: S3ClientError,
    }
}

#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("Invalid Scheduling")]
    GeneralSchedulingError
}


#[derive(Error, Debug)]
pub enum S3ClientError {
    #[error("Object Already Exists")]
    ObjectAlreadyExistsError,

    #[error("SDK Error {source:?}")]
    SdkError {
        #[from] source: aws_sdk_s3::Error,
    },

    #[error("SDK Error {source:?}")]
    SdkError2 {
        #[from] source: aws_sdk_s3::error::SdkError<UploadPartError>,
    },

    #[error("SDK Error {source:?}")]
    SdkError3 {
        #[from] source: aws_sdk_s3::error::SdkError<CompleteMultipartUploadError>,
    },

    #[error("SDK Error {source:?}")]
    SdkError5 {
        #[from] source: aws_sdk_s3::error::SdkError<PutObjectError>,
    },

    #[error("SDK Error {source:?}")]
    SdkError6 {
        #[from] source: aws_sdk_s3::error::SdkError<GetObjectError>,
    },

    #[error("SDK Error {source:?}")]
    SdkError4 {
        #[from] source: ByteStreamError,
    }

}



impl<'r, 'o: 'r> Responder<'r, 'o> for VickyError {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'o> {
        // log `self` to your favored error tracker, e.g.
        // sentry::capture_error(&self);

        {
            error!("Error: {}", self);
            Status::InternalServerError.respond_to(req)
        }
    }
}