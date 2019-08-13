use failure::Fail;
use sled;
use std::convert::From;
use std::io;
use std::string;

pub type Result<T> = std::result::Result<T, KvsError>;

#[derive(Debug, Fail)]
pub enum KvsError {
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),

    #[fail(display = "{}", _0)]
    Serde(#[cause] serde_json::Error),

    #[fail(display = "{}", _0)]
    Sled(#[cause] sled::Error),

    #[fail(display = "{}", _0)]
    UTF8Error(#[cause] string::FromUtf8Error),

    /// Removing non-existent key error
    #[fail(display = "Key not found")]
    KeyNotFound,

    #[fail(display = "Unexpected command type found")]
    UnexpectedCommandType,
}

impl From<io::Error> for KvsError {
    fn from(error: io::Error) -> Self {
        KvsError::Io(error)
    }
}

impl From<serde_json::Error> for KvsError {
    fn from(error: serde_json::Error) -> Self {
        KvsError::Serde(error)
    }
}

impl From<sled::Error> for KvsError {
    fn from(error: sled::Error) -> Self {
        KvsError::Sled(error)
    }
}

impl From<string::FromUtf8Error> for KvsError {
    fn from(error: string::FromUtf8Error) -> Self {
        KvsError::UTF8Error(error)
    }
}
