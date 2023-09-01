use rocket::{response::Responder, Request, Response, http::Status};
use thiserror::Error;
use vickylib::etcd::client::ClientError;
use uuid;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP Error {source:?}")]
    SerdeJson {
        #[from] source: serde_json::Error,
    },
    #[error("etcd Error {source:?}")]
    ClientError {
        #[from] source: ClientError,
    },

    #[error("uuid Error {source:?}")]
    UuidError {
        #[from] source: uuid::Error,
    },

    #[error("HTTP Error {source:?}")]
    HTTPError {
        #[from] source: HTTPError,
    },

    #[error("Scheduling Error {source:?}")]
    SchedulerError {
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



impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
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