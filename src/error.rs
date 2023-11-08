use core::fmt;
use std::{error, io};

#[derive(Debug, PartialEq, Eq)]
pub enum AirapErrorKind {
    Io,
    Audio,
    Feature,
    Unsupported,
}

#[derive(Debug)]
pub struct AirapError {
    pub kind: AirapErrorKind,
    pub message: String,
}

impl AirapError {
    pub fn new<S: Into<String>>(message: S, kind: AirapErrorKind) -> AirapError {
        AirapError {
            message: message.into(),
            kind,
        }
    }

    pub fn audio<S: Into<String>>(message: S) -> AirapError {
        Self::new(message, AirapErrorKind::Audio)
    }

    pub fn feature<S: Into<String>>(message: S) -> AirapError {
        Self::new(message, AirapErrorKind::Feature)
    }

    pub fn unsupported<S: Into<String>>(message: S) -> AirapError {
        Self::new(message, AirapErrorKind::Unsupported)
    }
}

impl error::Error for AirapError {}
impl fmt::Display for AirapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Airap Error {}", self.message,)
    }
}

impl From<io::Error> for AirapError {
    fn from(error: io::Error) -> Self {
        Self::new(format!("Failed to read input: {error}"), AirapErrorKind::Io)
    }
}

impl From<pulse::error::PAErr> for AirapError {
    fn from(value: pulse::error::PAErr) -> Self {
        Self::new(
            format!("Pulseaudio Error: {:?}", value.to_string()),
            AirapErrorKind::Io,
        )
    }
}
