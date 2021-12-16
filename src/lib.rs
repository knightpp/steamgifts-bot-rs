use std::num::ParseIntError;
pub mod account;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("expected status code 200, got {0}")]
    StatusCode(u16),
    #[error("json failed: {0}")]
    Json(&'static str),
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("surf error: {0}")]
    Surf(String),
    #[error("failed logging in: {0}")]
    Login(&'static str),
    #[error("failed entering GA: {0}")]
    Enter(String),
    #[error("unknown error")]
    Unknown,
    #[error("could not parse integer from string")]
    ParseError(#[from] ParseIntError),
    #[error("parse duration error: {0}")]
    HumanTimeError(#[from] humantime::DurationError),
}

impl From<surf::Error> for Error {
    fn from(err: surf::Error) -> Self {
        Error::Surf(err.to_string())
    }
}
