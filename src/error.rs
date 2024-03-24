use std::fmt::Display;

use json_elem::jsonelem::JsonElem;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

/// An object that is responsible to house error in JsonElem type
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Error {
    error: JsonElem,
}

impl Error {
    /// Creates an Error object in JsonElem
    pub fn new(error: JsonElem) -> Self {
        Self { error }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl std::error::Error for Error {}

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
