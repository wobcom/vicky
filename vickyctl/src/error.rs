use std::fmt::{Debug, Display, Formatter};
use yansi::Paint;

#[warn(dead_code)]
#[derive(Debug)]
pub enum Error {
    Dependency(String, String),
    Reqwest(reqwest::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Custom(String),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
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
            Error::Reqwest(e) => write!(f, "HTTP Error: {}", e),
            Error::Io(e) => write!(f, "Filesystem Error: {}", e),
            Error::Json(e) => write!(f, "Parser Error: {}", e),
            Error::Custom(ref str) => write!(f, "Custom Error: {}", str),
        }
    }
}

impl std::error::Error for Error {}
