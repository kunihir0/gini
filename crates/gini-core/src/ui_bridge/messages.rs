use std::fmt;
use crate::ui_bridge::MessageSeverity;

/// UI message types for different UI providers
#[derive(Debug, Clone)]
pub enum UiMessageType {
    /// Command message - execute a command
    Command(String),
    /// Query message - request information
    Query(String),
    /// Response message - response to a query
    Response(String),
    /// Event message - notify of an event
    Event(String, MessageSeverity),
}

/// Message result from UI provider
#[derive(Debug, Clone)]
pub enum MessageResult {
    /// No response needed
    None,
    /// Text response
    Text(String),
    /// Boolean response
    Boolean(bool),
    /// Numeric response
    Number(f64),
    /// Error response
    Error(String),
}

impl fmt::Display for MessageResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageResult::None => write!(f, "<no response>"),
            MessageResult::Text(text) => write!(f, "{}", text),
            MessageResult::Boolean(value) => write!(f, "{}", value),
            MessageResult::Number(value) => write!(f, "{}", value),
            MessageResult::Error(err) => write!(f, "Error: {}", err),
        }
    }
}

/// Utility functions for creating common messages
pub mod util {
    use super::*;
    use crate::ui_bridge::{UiMessage, UiUpdateType};
    
    /// Create an info log message
    pub fn info(source: &str, message: &str) -> UiMessage {
        UiMessage {
            update_type: UiUpdateType::Log(message.to_string(), MessageSeverity::Info),
            source: source.to_string(),
            timestamp: std::time::SystemTime::now(),
        }
    }
    
    /// Create a warning log message
    pub fn warning(source: &str, message: &str) -> UiMessage {
        UiMessage {
            update_type: UiUpdateType::Log(message.to_string(), MessageSeverity::Warning),
            source: source.to_string(),
            timestamp: std::time::SystemTime::now(),
        }
    }
    
    /// Create an error log message
    pub fn error(source: &str, message: &str) -> UiMessage {
        UiMessage {
            update_type: UiUpdateType::Log(message.to_string(), MessageSeverity::Error),
            source: source.to_string(),
            timestamp: std::time::SystemTime::now(),
        }
    }
    
    /// Create a status message
    pub fn status(source: &str, message: &str) -> UiMessage {
        UiMessage {
            update_type: UiUpdateType::Status(message.to_string()),
            source: source.to_string(),
            timestamp: std::time::SystemTime::now(),
        }
    }
    
    /// Create a progress message
    pub fn progress(source: &str, value: f32) -> UiMessage {
        UiMessage {
            update_type: UiUpdateType::Progress(value),
            source: source.to_string(),
            timestamp: std::time::SystemTime::now(),
        }
    }
}