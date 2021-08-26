use std::fmt::{Display, Formatter};

pub use anyhow::Result;

#[derive(Copy, Clone, Debug)]
pub enum ProtocolError {
    UnknownCommand,
    InvalidUtf8,
    InvalidLength, // Not all packet contents were parsed
    IncompleteBuffer, // length field > buffer size
    NoSuchHandle,
}

impl From<std::string::FromUtf8Error> for ProtocolError {
    fn from(_: std::string::FromUtf8Error) -> Self {
        ProtocolError::InvalidUtf8
    }
}

impl Display for ProtocolError {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        write!(fmt, "{:?}", self)
    }
}

impl std::error::Error for ProtocolError {}
