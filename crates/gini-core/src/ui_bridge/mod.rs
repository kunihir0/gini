//! # Gini Core UI Bridge
//!
//! This module facilitates communication between the `gini-core` application and
//! various user interface (UI) implementations. It provides a standardized way for
//! the core to send updates and information to the UI, and for the UI to send
//! user input or commands back to the core.
//!
//! ## Key Components & Concepts:
//!
//! - **[`UiProvider`] Trait**: Defines the contract for different UI implementations
//!   (e.g., console, graphical). Implementors of this trait handle the actual
//!   rendering of UI elements and capturing of user input.
//! - **[`UiBridge`] Struct**: Manages a collection of `UiProvider` instances. It is
//!   responsible for dispatching messages from the core to all registered UI
//!   providers.
//! - **[`UiMessage`] Struct**: The standard format for messages sent from the core
//!   to the UI. It includes various [`UiUpdateType`]s (e.g., text, progress) and
//!   [`MessageSeverity`].
//! - **[`UserInput`] Struct**: Represents input received from the user via a UI.
//! - **[`UiConnector`] Trait & [`UIManager`](unified_interface::UIManager)**:
//!   The `UIManager` (re-exported from the `unified_interface` submodule) and its
//!   associated `UiConnector` trait provide a higher-level interface for managing
//!   UI interactions and state, potentially coordinating multiple `UiBridge` instances
//!   or providing more complex UI logic.
//! - **Submodules**:
//!     - `messages`: Defines the structure of messages like `UiMessage`, `UserInput`,
//!       `UiUpdateType`, and `MessageSeverity`.
//!     - `unified_interface`: Contains the `UIManager` and `UiConnector` for a more
//!       abstracted UI management layer.
//!     - `error`: Defines UI bridge specific error types like [`UiBridgeError`](error::UiBridgeError).
//!
//! The UI bridge aims to decouple the core application logic from specific UI
//! technologies, allowing for flexibility in how the application is presented
//! to and interacts with the user.
pub mod messages;
// pub mod manager; // Removed manager module
pub mod error;
pub mod unified_interface;

// pub use manager::UIManager; // Removed UIManager export
pub use unified_interface::UnifiedUiInterface; // Export UnifiedUiInterface
use crate::ui_bridge::error::UiBridgeError; // Import UiBridgeError
use crate::event::{EventManager, types::PingCommandEvent}; // Added EventManager and PingCommandEvent
use log; // Import log crate
use crate::kernel::component::KernelComponent;
use crate::kernel::error as KernelErrorPkg; // Alias to avoid conflict with local error module
use async_trait::async_trait;

use std::sync::{Arc, Mutex}; // Added Arc
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fmt::Debug; // Import Debug

// --- Added Definitions ---

/// Represents input received from a user via a UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserInput {
    /// Simple text input.
    Text(String),
    /// A specific command with arguments.
    Command(String, Vec<String>),
    /// Confirmation response (e.g., yes/no).
    Confirmation(bool),
    // Add other input types as necessary (e.g., selection from a list)
}

// --- End Added Definitions ---


// Old UiConnector trait removed.

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

// Old UiProvider trait removed.

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

impl UnifiedUiInterface for ConsoleUiProvider {
    fn name(&self) -> &str { // Changed from &'static str
        "console"
    }

    fn initialize(&mut self) -> Result<(), UiBridgeError> {
        self.initialized = true;
        println!("Console UI initialized");
        Ok(())
    }

    fn handle_message(&mut self, message: &UiMessage) -> Result<(), UiBridgeError> {
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

    fn send_input(&mut self, input: UserInput) -> Result<(), UiBridgeError> {
        // ConsoleUiProvider is primarily for output and does not process input by default.
        // This could be extended if interactive console input is desired.
        log::debug!("ConsoleUiProvider received send_input call with: {:?}. This is a no-op for now.", input);
        Ok(())
    }

    fn update(&mut self) -> Result<(), UiBridgeError> {
        // Nothing to do for console UI
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), UiBridgeError> {
        self.initialized = false;
        println!("Console UI finalized");
        Ok(())
    }

    fn supports_interactive(&self) -> bool {
        false // Console is not interactive by default in this setup
    }
}

/// Manages multiple UI interfaces and facilitates communication between the core application and them.
#[derive(Debug, Clone)] // Added Clone
pub struct UnifiedUiManager {
    interfaces: Arc<Mutex<HashMap<String, Arc<Mutex<Box<dyn UnifiedUiInterface>>>>>>,
    default_interface: Arc<Mutex<Option<String>>>,
    message_buffer: Arc<Mutex<Vec<UiMessage>>>,
    event_manager: Arc<dyn EventManager>, // Added EventManager
    // Note: UserInput is currently submitted via a direct method call to UnifiedUiManager.
    // Alternative patterns like channels or callbacks could be considered for future enhancements
    // if more complex input routing or decoupling is required.
}

impl UnifiedUiManager {
    /// Create a new UI manager. Initially, it contains a default console UI.
    pub fn new(event_manager: Arc<dyn EventManager>) -> Self {
        let mut interfaces_map = HashMap::new();
        let console_interface = Arc::new(Mutex::new(Box::new(ConsoleUiProvider::new()) as Box<dyn UnifiedUiInterface>));
        let console_name = console_interface.lock().unwrap().name().to_string(); // Lock to get name
        interfaces_map.insert(console_name.clone(), console_interface);

        Self {
            interfaces: Arc::new(Mutex::new(interfaces_map)),
            default_interface: Arc::new(Mutex::new(Some(console_name))),
            message_buffer: Arc::new(Mutex::new(Vec::new())),
            event_manager,
        }
    }
    
    /// Register a UI interface.
    pub fn register_interface(&self, interface: Box<dyn UnifiedUiInterface>) -> Result<(), UiBridgeError> {
        let name = interface.name().to_string();
        let mut interfaces_guard = self.interfaces.lock().map_err(|e| UiBridgeError::LockError { entity: "interfaces map".to_string(), operation: format!("register_interface - lock: {}", e) })?;
        if interfaces_guard.contains_key(&name) {
            return Err(UiBridgeError::RegistrationFailed(format!("Interface with name '{}' already exists.", name)));
        }
        interfaces_guard.insert(name.clone(), Arc::new(Mutex::new(interface)));
        
        let mut default_interface_guard = self.default_interface.lock().map_err(|e| UiBridgeError::LockError { entity: "default_interface".to_string(), operation: format!("register_interface - default lock: {}", e) })?;
        if default_interface_guard.is_none() {
            *default_interface_guard = Some(name);
        }
        
        Ok(())
    }
    
    /// Set the default UI interface.
    pub fn set_default_interface(&self, name: &str) -> Result<(), UiBridgeError> {
        let interfaces_guard = self.interfaces.lock().map_err(|e| UiBridgeError::LockError { entity: "interfaces map".to_string(), operation: format!("set_default_interface - lock: {}", e) })?;
        if interfaces_guard.contains_key(name) {
            let mut default_interface_guard = self.default_interface.lock().map_err(|e| UiBridgeError::LockError { entity: "default_interface".to_string(), operation: format!("set_default_interface - default lock: {}", e) })?;
            *default_interface_guard = Some(name.to_string());
            Ok(())
        } else {
            Err(UiBridgeError::InterfaceNotFound(name.to_string()))
        }
    }
    
    /// Broadcasts a message to all registered UI interfaces.
    pub fn broadcast_message(&self, message: UiMessage) -> Result<(), UiBridgeError> {
        // Buffer the message
        self.message_buffer.lock().map_err(|e|
            UiBridgeError::LockError {
                entity: "MessageBuffer".to_string(),
                operation: format!("send_message - buffer lock: {}", e)
            }
        )?.push(message.clone());
        
        // Buffer the message
        self.message_buffer.lock().map_err(|e|
            UiBridgeError::LockError {
                entity: "MessageBuffer".to_string(),
                operation: format!("broadcast_message - buffer lock: {}", e)
            }
        )?.push(message.clone());
        
        // Try to send to all interfaces
        let interfaces_guard = self.interfaces.lock().map_err(|e| UiBridgeError::LockError { entity: "interfaces map".to_string(), operation: format!("broadcast_message - lock: {}", e) })?;
        for (name, interface_arc_mutex) in interfaces_guard.iter() {
            match interface_arc_mutex.lock() {
                Ok(mut interface) => {
                    if let Err(e) = interface.handle_message(&message) {
                        log::error!("Failed to send message to UI interface '{}': {}", name, e);
                        // Individual interface errors are logged, broadcast_message itself doesn't fail for this.
                    }
                },
                Err(e) => {
                    log::error!("Failed to lock UI interface '{}' for broadcast_message: {}", name, e);
                }
            }
        }
        
        Ok(())
    }

    /// Submits user input received from a specific UI interface.
    /// The manager is responsible for routing this input to the core application logic.
    pub async fn submit_user_input(&self, input: UserInput, source_interface_name: &str) -> Result<(), UiBridgeError> {
        log::debug!("Processing user input from '{}': {:?}", source_interface_name, input);

        match input {
            UserInput::Text(text_content) => {
                if text_content.trim().to_lowercase() == "ping" {
                    log::info!(
                        "Received 'ping' command from UI interface: '{}'. Attempting to emit PingCommandEvent.",
                        source_interface_name
                    );
                    let event = PingCommandEvent {
                        source_id: Some(source_interface_name.to_string()),
                    };
                    
                    let _: () = self.event_manager.queue_event(Box::new(event)).await; // Explicitly type annotate
                    // Temporarily removed problematic log line for diagnostics
                    Ok(())
                } else {
                    log::info!(
                        "Unhandled text input from '{}': {}",
                        source_interface_name,
                        text_content
                    );
                    Ok(())
                }
            }
            UserInput::Command(command, args) => {
                log::info!(
                    "Unhandled command input from '{}': {} with args {:?}",
                    source_interface_name,
                    command,
                    args
                );
                Ok(())
            }
            UserInput::Confirmation(value) => {
                log::info!(
                    "Unhandled confirmation input from '{}': {}",
                    source_interface_name,
                    value
                );
                Ok(())
            }
        }
    }
    
    /// Initialize all registered UI interfaces.
    // Changed to take &self as KernelComponent methods take &self
    pub fn initialize_all(&self) -> Result<(), UiBridgeError> {
        let interfaces_guard = self.interfaces.lock().map_err(|e| UiBridgeError::LockError { entity: "interfaces map".to_string(), operation: format!("initialize_all - lock: {}", e) })?;
        for (name, interface_arc_mutex) in interfaces_guard.iter() {
            let mut interface = interface_arc_mutex.lock().map_err(|e|
                UiBridgeError::LockError {
                    entity: format!("UnifiedUiInterface '{}'", name),
                    operation: format!("initialize_all - interface lock failed: {}", e)
                }
            )?;
            interface.initialize().map_err(|e|
                UiBridgeError::LifecycleMethodFailed {
                    interface_name: name.to_string(),
                    method: "initialize".to_string(),
                    source: Box::new(e)
                }
            )?;
        }
        Ok(())
    }
    
    /// Update all registered UI interfaces.
    // Changed to take &self
    pub fn update_all(&self) -> Result<(), UiBridgeError> {
        let mut collected_errors: Vec<UiBridgeError> = Vec::new();
        let interfaces_guard = self.interfaces.lock().map_err(|e| UiBridgeError::LockError { entity: "interfaces map".to_string(), operation: format!("update_all - lock: {}", e) })?;
        for (name, interface_arc_mutex) in interfaces_guard.iter() {
            match interface_arc_mutex.lock() {
                Ok(mut interface) => {
                    if let Err(e) = interface.update() {
                        collected_errors.push(UiBridgeError::LifecycleMethodFailed {
                            interface_name: name.to_string(),
                            method: "update".to_string(),
                            source: Box::new(e)
                        });
                    }
                },
                Err(e) => {
                    collected_errors.push(UiBridgeError::LockError {
                        entity: format!("UnifiedUiInterface '{}'", name),
                        operation: format!("update_all - interface lock failed: {}", e)
                    });
                }
            }
        }
        
        if collected_errors.is_empty() {
            Ok(())
        } else {
            Err(UiBridgeError::MultipleInterfaceFailures(collected_errors))
        }
    }
    
    /// Finalize all registered UI interfaces.
    // Changed to take &self
    pub fn finalize_all(&self) -> Result<(), UiBridgeError> {
        let mut collected_errors: Vec<UiBridgeError> = Vec::new();
        let interfaces_guard = self.interfaces.lock().map_err(|e| UiBridgeError::LockError { entity: "interfaces map".to_string(), operation: format!("finalize_all - lock: {}", e) })?;
        for (name, interface_arc_mutex) in interfaces_guard.iter() {
            match interface_arc_mutex.lock() {
                Ok(mut interface) => {
                    if let Err(e) = interface.finalize() {
                        collected_errors.push(UiBridgeError::LifecycleMethodFailed {
                            interface_name: name.to_string(),
                            method: "finalize".to_string(),
                            source: Box::new(e)
                        });
                    }
                },
                Err(e) => {
                    collected_errors.push(UiBridgeError::LockError {
                        entity: format!("UnifiedUiInterface '{}'", name),
                        operation: format!("finalize_all - interface lock failed: {}", e)
                    });
                }
            }
        }
        
        if collected_errors.is_empty() {
            Ok(())
        } else {
            Err(UiBridgeError::MultipleInterfaceFailures(collected_errors))
        }
    }
    
    /// Helper to create and broadcast a progress message.
    pub fn progress(&self, source: &str, progress_value: f32) -> Result<(), UiBridgeError> {
        self.broadcast_message(UiMessage {
            update_type: UiUpdateType::Progress(progress_value),
            source: source.to_string(),
            timestamp: SystemTime::now(),
        })
    }
    
    /// Helper to create and broadcast a status message.
    pub fn status(&self, source: &str, message_text: &str) -> Result<(), UiBridgeError> {
        self.broadcast_message(UiMessage {
            update_type: UiUpdateType::Status(message_text.to_string()),
            source: source.to_string(),
            timestamp: SystemTime::now(),
        })
    }
    
    /// Helper to create and broadcast a log message.
    pub fn log(&self, source: &str, message_text: &str, severity_level: MessageSeverity) -> Result<(), UiBridgeError> {
        self.broadcast_message(UiMessage {
            update_type: UiUpdateType::Log(message_text.to_string(), severity_level),
            source: source.to_string(),
            timestamp: SystemTime::now(),
        })
    }
    
    /// Helper to create and broadcast a dialog message.
    pub fn dialog(&self, source: &str, message_text: &str, severity_level: MessageSeverity) -> Result<(), UiBridgeError> {
        self.broadcast_message(UiMessage {
            update_type: UiUpdateType::Dialog(message_text.to_string(), severity_level),
            source: source.to_string(),
            timestamp: SystemTime::now(),
        })
    }

    /// Get the name of the current default UI interface.
    pub fn get_default_interface_name(&self) -> Option<String> {
        self.default_interface.lock().unwrap().clone() // Lock to access
    }
}

// Default implementation removed as UnifiedUiManager now requires an EventManager.
// #[async_trait]
// impl Default for UnifiedUiManager {
//     fn default() -> Self {
//         // This would require a way to get a default EventManager,
//         // which might not be feasible or desirable.
//         // Consider if Default is truly needed or if explicit construction
//         // with dependencies is always preferred.
//         panic!("UnifiedUiManager cannot be default-initialized without an EventManager.");
//     }
// }

#[async_trait]
impl KernelComponent for UnifiedUiManager {
    fn name(&self) -> &'static str {
        "UnifiedUiManager"
    }

    async fn initialize(&self) -> KernelErrorPkg::Result<()> {
        log::info!("Initializing UnifiedUiManager...");
        self.initialize_all().map_err(KernelErrorPkg::Error::from)
    }

    async fn start(&self) -> KernelErrorPkg::Result<()> {
        log::info!("Starting UnifiedUiManager (running update_all)...");
        // Typically, start might involve more, but for now, update_all is a placeholder.
        self.update_all().map_err(KernelErrorPkg::Error::from)
    }

    async fn stop(&self) -> KernelErrorPkg::Result<()> {
        log::info!("Stopping UnifiedUiManager...");
        self.finalize_all().map_err(KernelErrorPkg::Error::from)
    }
}


// Test module declaration
#[cfg(test)]
mod tests;