pub mod messages;

use std::fmt;
use std::sync::Mutex;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// UI message severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSeverity {
    /// Debug information
    Debug,
    /// Informational message
    Info,
    /// Warning message
    Warning,
    /// Error message
    Error,
    /// Critical error message
    Critical,
}

/// UI update type
#[derive(Debug, Clone)]
pub enum UiUpdateType {
    /// Progress update
    Progress(f32),
    /// Status message
    Status(String),
    /// Log message
    Log(String, MessageSeverity),
    /// Dialog message
    Dialog(String, MessageSeverity),
}

impl PartialEq for UiUpdateType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (UiUpdateType::Progress(a), UiUpdateType::Progress(b)) => 
                (a - b).abs() < std::f32::EPSILON,
            (UiUpdateType::Status(a), UiUpdateType::Status(b)) => a == b,
            (UiUpdateType::Log(a, s1), UiUpdateType::Log(b, s2)) => a == b && s1 == s2,
            (UiUpdateType::Dialog(a, s1), UiUpdateType::Dialog(b, s2)) => a == b && s1 == s2,
            _ => false,
        }
    }
}

impl Eq for UiUpdateType {}

/// UI message for communication with UI providers
#[derive(Debug, Clone)]
pub struct UiMessage {
    /// Message type
    pub update_type: UiUpdateType,
    /// Source component
    pub source: String,
    /// Message timestamp
    pub timestamp: SystemTime,
}

/// Trait for UI providers
pub trait UiProvider: Send + Sync {
    /// Get the name of this provider
    fn name(&self) -> &'static str;
    
    /// Initialize the UI
    fn initialize(&mut self) -> Result<(), String>;
    
    /// Handle a UI message
    fn handle_message(&mut self, message: &UiMessage) -> Result<(), String>;
    
    /// Update the UI
    fn update(&mut self) -> Result<(), String>;
    
    /// Finalize/clean up the UI
    fn finalize(&mut self) -> Result<(), String>;
    
    /// Check if this UI provider supports interactive mode
    fn supports_interactive(&self) -> bool;
}

/// Basic console UI provider
#[derive(Debug)]
struct ConsoleUiProvider {
    initialized: bool,
}

impl ConsoleUiProvider {
    fn new() -> Self {
        Self {
            initialized: false,
        }
    }
    
    fn format_time(time: SystemTime) -> String {
        if let Ok(duration) = time.duration_since(UNIX_EPOCH) {
            let secs = duration.as_secs();
            format!("{:02}:{:02}:{:02}", 
                  (secs / 3600) % 24,
                  (secs / 60) % 60, 
                  secs % 60)
        } else {
            String::from("00:00:00")
        }
    }
}

impl UiProvider for ConsoleUiProvider {
    fn name(&self) -> &'static str {
        "console"
    }
    
    fn initialize(&mut self) -> Result<(), String> {
        self.initialized = true;
        println!("Console UI initialized");
        Ok(())
    }
    
    fn handle_message(&mut self, message: &UiMessage) -> Result<(), String> {
        let msg_type = match &message.update_type {
            UiUpdateType::Progress(val) => format!("Progress: {:.1}%", val * 100.0),
            UiUpdateType::Status(msg) => format!("Status: {}", msg),
            UiUpdateType::Log(msg, severity) => format!("{:?}: {}", severity, msg),
            UiUpdateType::Dialog(msg, severity) => format!("Dialog ({:?}): {}", severity, msg),
        };
        
        let time_str = Self::format_time(message.timestamp);
        println!("[{}] {}: {}", message.source, time_str, msg_type);
        
        Ok(())
    }
    
    fn update(&mut self) -> Result<(), String> {
        // Nothing to do for console UI
        Ok(())
    }
    
    fn finalize(&mut self) -> Result<(), String> {
        self.initialized = false;
        println!("Console UI finalized");
        Ok(())
    }
    
    fn supports_interactive(&self) -> bool {
        false
    }
}

/// Bridge between application logic and UI
pub struct UiBridge {
    providers: HashMap<String, Mutex<Box<dyn UiProvider>>>,
    default_provider: Option<String>,
    message_buffer: Mutex<Vec<UiMessage>>,
}

impl UiBridge {
    /// Create a new UI bridge with console provider
    pub fn new() -> Self {
        let mut bridge = Self {
            providers: HashMap::new(),
            default_provider: None,
            message_buffer: Mutex::new(Vec::new()),
        };
        
        // Add default console provider
        let console_provider = Box::new(ConsoleUiProvider::new());
        let console_name = console_provider.name().to_string();
        bridge.providers.insert(console_name.clone(), Mutex::new(console_provider));
        bridge.default_provider = Some(console_name);
        
        bridge
    }
    
    /// Register a UI provider
    pub fn register_provider(&mut self, provider: Box<dyn UiProvider>) -> Result<(), String> {
        let name = provider.name().to_string();
        self.providers.insert(name.clone(), Mutex::new(provider));
        
        // Set as default if no default exists
        if self.default_provider.is_none() {
            self.default_provider = Some(name);
        }
        
        Ok(())
    }
    
    /// Set the default provider
    pub fn set_default_provider(&mut self, name: &str) -> Result<(), String> {
        if self.providers.contains_key(name) {
            self.default_provider = Some(name.to_string());
            Ok(())
        } else {
            Err(format!("UI provider not found: {}", name))
        }
    }
    
    /// Send a message to all UI providers
    pub fn send_message(&self, message: UiMessage) -> Result<(), String> {
        // Buffer the message
        if let Ok(mut buffer) = self.message_buffer.lock() {
            buffer.push(message.clone());
        } else {
            return Err("Failed to lock message buffer".to_string());
        }
        
        // Try to send to all providers
        for (name, provider_mutex) in &self.providers {
            match provider_mutex.lock() {
                Ok(mut provider) => {
                    if let Err(e) = provider.handle_message(&message) {
                        eprintln!("Failed to send message to UI provider '{}': {}", name, e);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to lock UI provider '{}': {}", name, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Initialize all providers
    pub fn initialize(&mut self) -> Result<(), String> {
        for (name, provider_mutex) in &self.providers {
            match provider_mutex.lock() {
                Ok(mut provider) => {
                    if let Err(e) = provider.initialize() {
                        return Err(format!("Failed to initialize UI provider '{}': {}", name, e));
                    }
                },
                Err(e) => {
                    return Err(format!("Failed to lock UI provider '{}': {}", name, e));
                }
            }
        }
        Ok(())
    }
    
    /// Update all UI providers
    pub fn update(&mut self) -> Result<(), String> {
        for (name, provider_mutex) in &self.providers {
            match provider_mutex.lock() {
                Ok(mut provider) => {
                    if let Err(e) = provider.update() {
                        eprintln!("Failed to update UI provider '{}': {}", name, e);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to lock UI provider '{}': {}", name, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Finalize all providers
    pub fn finalize(&mut self) -> Result<(), String> {
        for (name, provider_mutex) in &self.providers {
            match provider_mutex.lock() {
                Ok(mut provider) => {
                    if let Err(e) = provider.finalize() {
                        eprintln!("Failed to finalize UI provider '{}': {}", name, e);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to lock UI provider '{}': {}", name, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Create a progress message
    pub fn progress(&self, source: &str, progress: f32) -> Result<(), String> {
        self.send_message(UiMessage {
            update_type: UiUpdateType::Progress(progress),
            source: source.to_string(),
            timestamp: SystemTime::now(),
        })
    }
    
    /// Create a status message
    pub fn status(&self, source: &str, message: &str) -> Result<(), String> {
        self.send_message(UiMessage {
            update_type: UiUpdateType::Status(message.to_string()),
            source: source.to_string(),
            timestamp: SystemTime::now(),
        })
    }
    
    /// Create a log message
    pub fn log(&self, source: &str, message: &str, severity: MessageSeverity) -> Result<(), String> {
        self.send_message(UiMessage {
            update_type: UiUpdateType::Log(message.to_string(), severity),
            source: source.to_string(),
            timestamp: SystemTime::now(),
        })
    }
    
    /// Create a dialog message
    pub fn dialog(&self, source: &str, message: &str, severity: MessageSeverity) -> Result<(), String> {
        self.send_message(UiMessage {
            update_type: UiUpdateType::Dialog(message.to_string(), severity),
            source: source.to_string(),
            timestamp: SystemTime::now(),
        })
    }

    /// Get the name of the current default provider
    pub fn get_default_provider_name(&self) -> Option<String> {
        self.default_provider.clone()
    }
}

impl Default for UiBridge {
    fn default() -> Self {
        Self::new()
    }

}

// Test module declaration
#[cfg(test)]
mod tests;