use std::fmt::{Debug, Display, Formatter};
use yansi::Paint;
use crate::http_client::{format_http_msg};

#[derive(Debug)]
pub enum Error {
    Dependency(String, String),
    Reqwest(reqwest::Error),
    ReqwestDetailed(reqwest::Error, String),
    Io(std::io::Error),
    Json(serde_json::Error),
    Unauthenticated(),
    #[allow(dead_code)]
    Custom(&'static str),
    Anyhow(anyhow::Error),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}

impl From<(reqwest::Error, String)> for Error {
    fn from(e: (reqwest::Error, String)) -> Self {
        Error::ReqwestDetailed(e.0, e.1)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::Anyhow(e)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Dependency(ref prog, ref dependent) => {
                write!(
                    f,
                    "{} {} {} {}",
                    "Dependency Error:".bright_red(),
                    prog.bold(),
                    "is not installed as a dependency on this system, but needs to be for"
                        .bright_red(),
                    dependent.bright_red()
                )
            }
            Error::Reqwest(e) => write!(f, "{}", format_http_msg(e.status(), &e.to_string())),
            Error::Io(e) => write!(f, "Filesystem Error: {}", e),
            Error::Json(e) => write!(f, "Parser Error: {}", e),
            Error::Custom(ref str) => write!(f, "Custom Error: {}", str),
            Error::Anyhow(ref str) => write!(f, "Unknown Error: {}", str),
            Error::Unauthenticated() => write!(f, "Unauthenticated"),
            Error::ReqwestDetailed(e, ref detail) => {
                write!(f, "{}", format_http_msg(e.status(), detail))
            }
        }
    }
}

impl std::error::Error for Error {}
