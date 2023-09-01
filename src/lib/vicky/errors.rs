use log::error;
use rocket::{response::Responder, Request, http::Status};
use thiserror::Error;

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

    #[error("HTTP Error {source:?}")]
    Http {
        #[from] source: HTTPError,
    },

    #[error("Scheduling Error {source:?}")]
    Scheduler {
        #[from] source: SchedulerError,
    }
}

#[derive(Error, Debug)]
pub enum HTTPError {
    #[error("Resource Not Found")]
    NotFound
}

#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("Invalid Scheduling")]
    GeneralSchedulingError
}



impl<'r, 'o: 'r> Responder<'r, 'o> for VickyError {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'o> {
        // log `self` to your favored error tracker, e.g.
        // sentry::capture_error(&self);

        match self {
            // in our simplistic example, we're happy to respond with the default 500 responder in all cases 
            _ => {
                error!("Error: {}", self);
                Status::InternalServerError.respond_to(req)
            }
        }
    }
}