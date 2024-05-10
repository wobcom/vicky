use std::fmt::{Debug, Display, Formatter};
use yansi::Paint;

#[derive(Debug)]
pub enum Error {
    Dependency(String, String),
    Custom(String),
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
            Error::Custom(ref str) => write!(f, "Custom Error: {}", str),
        }
    }
}

impl std::error::Error for Error {}
