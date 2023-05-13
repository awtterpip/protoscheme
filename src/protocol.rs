use serde::{Serialize, Deserialize};
use thiserror::Error;

use crate::{SerializationError, DeserializationError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Receiver has been dropped")]
    ReceiverDropped,
    #[error("IO Error: {0}")]
    IOError(std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(SerializationError),
    #[error("Deserialization error: {0}")]
    DeserializationError(DeserializationError),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IOError(e)
    }
}

impl From<SerializationError> for Error {
    fn from(e: SerializationError) -> Self {
        Self::SerializationError(e)
    }
}

impl From<DeserializationError> for Error {
    fn from(e: DeserializationError) -> Self {
        Self::DeserializationError(e)
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Message {
    Signal {
        name: u64,
        data: Vec<u8>,
    },
    Request {
        id: u64,
        name: u64,
        data: Vec<u8>,
    },
    Response {
        id: u64,
        data: Vec<u8>,
    }
}

pub trait Protocol {
    
}