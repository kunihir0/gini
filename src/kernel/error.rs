use std::error::Error as StdError;
use std::fmt;
use std::result::Result as StdResult;

/// Custom error type for the OSX-Forge application
#[derive(Debug)]
pub enum Error {
    /// Initialization error
    Init(String),
    /// Plugin system error
    Plugin(String),
    /// Stage execution error
    Stage(String),
    /// Storage error
    Storage(String),
    /// Event system error
    Event(String),
    /// Generic error with message
    Other(String),
}

/// Shorthand for Result with our Error type
pub type Result<T> = StdResult<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Init(msg) => write!(f, "Initialization error: {}", msg),
            Error::Plugin(msg) => write!(f, "Plugin error: {}", msg),
            Error::Stage(msg) => write!(f, "Stage error: {}", msg),
            Error::Storage(msg) => write!(f, "Storage error: {}", msg),
            Error::Event(msg) => write!(f, "Event error: {}", msg),
            Error::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl StdError for Error {}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::Other(msg.to_string())
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::Other(msg)
    }
}