use std::fmt::Display;

use json_elem::JsonElem;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

/// An object that is responsible to house error in JsonElem type
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct RemoteError {
    error: JsonElem,
}

impl RemoteError {
    /// Creates an Error object in JsonElem
    pub fn new(error: JsonElem) -> Self {
        Self { error }
    }
}

impl Display for RemoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl std::error::Error for RemoteError {}

/// A list of common error string.
#[derive(Debug, EnumString, Display, AsRefStr)]
pub enum CommonErrors {
    #[strum(serialize = "OK")]
    Ok,
    #[strum(serialize = "Object not found")]
    ObjectNotFound,
    #[strum(serialize = "client connection error")]
    ClientConnectionError,
    #[strum(serialize = "server connection error")]
    ServerConnectionError,
    #[strum(serialize = "serde parsing error")]
    SerdeParseError,
    #[strum(serialize = "remote connection error")]
    RemoteConnectionError,
    #[strum(serialize = "invalid response data")]
    InvalidResponseData,
}

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Atticus(atticus::Error),
    Others(String),
    Serde(serde_json::Error),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(err) => write!(f, "{}", err),
            Error::Atticus(err) => write!(f, "{}", err),
            Error::Others(err) => write!(f, "{}", err),
            Error::Serde(err) => write!(f, "{}", err),
        }
    }
}

impl From<atticus::Error> for Error {
    fn from(value: atticus::Error) -> Self {
        Self::Atticus(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}
