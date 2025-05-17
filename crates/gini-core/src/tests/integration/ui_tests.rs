#![cfg(test)]

use tokio::test;
use std::time::SystemTime;

// Removed redundant import line below

#[test]
async fn test_ui_message_creation() {
    // Test UI message creation
    let message_id = "test_id";
    let action = "test_action";
    
    // Create a UiMessage directly instead of using a constructor
    let message = UiMessage {
        update_type: UiUpdateType::Status(action.to_string()),
        source: message_id.to_string(),
        timestamp: SystemTime::now(),
    };
    
    // Test message format
    assert_eq!(message.source, message_id);
    assert!(matches!(message.update_type, UiUpdateType::Status(ref s) if s == action));
}

// Add necessary imports if not already present
use crate::ui_bridge::{UnifiedUiManager, UnifiedUiInterface, UiMessage, UserInput, UiUpdateType, MessageSeverity, error::UiBridgeError};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::fmt::Debug;
// SystemTime is already imported

// --- Mock UI Interface for Testing ---

#[derive(Debug)]
struct MockUiInterface {
    name: &'static str,
    init_called: Arc<Mutex<bool>>,
    finalize_called: Arc<Mutex<bool>>,
    update_called: Arc<Mutex<bool>>,
    handle_message_called: Arc<Mutex<bool>>,
    send_input_called: Arc<Mutex<bool>>,
    messages_received: Arc<Mutex<VecDeque<UiMessage>>>, // Use VecDeque to check order easily
    inputs_sent: Arc<Mutex<VecDeque<UserInput>>>,
}

#[allow(dead_code)] // Allow dead code for test helper methods
impl MockUiInterface {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            init_called: Arc::new(Mutex::new(false)),
            finalize_called: Arc::new(Mutex::new(false)),
            update_called: Arc::new(Mutex::new(false)),
            handle_message_called: Arc::new(Mutex::new(false)),
            send_input_called: Arc::new(Mutex::new(false)),
            messages_received: Arc::new(Mutex::new(VecDeque::new())),
            inputs_sent: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    // Helper methods to access tracker state for assertions
    fn was_initialized(&self) -> bool {
        *self.init_called.lock().unwrap()
    }

    fn was_finalized(&self) -> bool {
        *self.finalize_called.lock().unwrap()
    }
    
    fn was_updated(&self) -> bool {
        *self.update_called.lock().unwrap()
    }

    fn get_received_messages(&self) -> VecDeque<UiMessage> {
        self.messages_received.lock().unwrap().clone()
    }
    
    fn get_sent_inputs(&self) -> VecDeque<UserInput> {
        self.inputs_sent.lock().unwrap().clone()
    }
}

impl UnifiedUiInterface for MockUiInterface {
    fn name(&self) -> &str { // Changed from &'static str
        self.name
    }

    fn initialize(&mut self) -> Result<(), UiBridgeError> {
        log::debug!("MockUiInterface '{}' initialized", self.name);
        *self.init_called.lock().unwrap() = true;
        Ok(())
    }

    fn handle_message(&mut self, message: &UiMessage) -> Result<(), UiBridgeError> {
        log::debug!("MockUiInterface '{}' received message: {:?}", self.name, message.update_type);
        *self.handle_message_called.lock().unwrap() = true;
        self.messages_received.lock().unwrap().push_back(message.clone());
        Ok(())
    }
    
    fn send_input(&mut self, input: UserInput) -> Result<(), UiBridgeError> {
        log::debug!("MockUiInterface '{}' send_input called with: {:?}", self.name, input);
        *self.send_input_called.lock().unwrap() = true;
        self.inputs_sent.lock().unwrap().push_back(input);
        Ok(())
    }

    fn update(&mut self) -> Result<(), UiBridgeError> {
        log::debug!("MockUiInterface '{}' update called", self.name);
        *self.update_called.lock().unwrap() = true;
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), UiBridgeError> {
        log::debug!("MockUiInterface '{}' finalized", self.name);
        *self.finalize_called.lock().unwrap() = true;
        Ok(())
    }

    fn supports_interactive(&self) -> bool {
        true // Mock supports interactive mode for testing send_input
    }
}


// --- Test: UI Interface Registration and Default ---
#[test]
async fn test_ui_interface_registration_and_default() {
    let manager = UnifiedUiManager::new(); // Creates with default "console" interface

    // Verify initial default interface
    assert_eq!(manager.get_default_interface_name(), Some("console".to_string()), "Initial default should be console");

    // Create and register a mock interface
    let mock_interface = MockUiInterface::new("mock_ui");
    let mock_interface_name = mock_interface.name().to_string();
    manager.register_interface(Box::new(mock_interface)).expect("Failed to register mock interface");

    // Verify default is still console (register doesn't automatically set default if one exists and current default is Some)
    // This behavior depends on the implementation of register_interface.
    // The current implementation sets as default only if no default exists.
    // Since "console" is the initial default, it should remain "console".
    assert_eq!(manager.get_default_interface_name(), Some("console".to_string()), "Default should still be console after registering mock");

    // Set the mock interface as default
    manager.set_default_interface(&mock_interface_name).expect("Failed to set mock interface as default");

    // Verify the default interface name is now the mock interface's name
    assert_eq!(manager.get_default_interface_name(), Some(mock_interface_name.clone()), "Default interface name should be updated to mock");

    // Test setting a non-existent interface as default
    let result = manager.set_default_interface("non_existent_interface");
    assert!(result.is_err(), "Setting a non-existent interface as default should fail");
    assert_eq!(manager.get_default_interface_name(), Some(mock_interface_name), "Default interface should remain mock after failed set");
}


// --- Test: UnifiedUiManager Message Dispatch ---
#[test]
async fn test_unified_ui_manager_message_dispatch() {
    let manager = UnifiedUiManager::new();
    let mock_interface = MockUiInterface::new("message_dispatch_mock");
    let mock_name = mock_interface.name().to_string();
    let messages_tracker = mock_interface.messages_received.clone(); // Clone Arc for later access

    // Register and set mock as default (though broadcast goes to all, not just default)
    manager.register_interface(Box::new(mock_interface)).unwrap();
    manager.set_default_interface(&mock_name).unwrap();

    // Send messages using helper methods
    manager.log("TestSource", "Log message", MessageSeverity::Info).unwrap();
    manager.status("TestSource", "Status update").unwrap();
    manager.progress("TestSource", 0.5).unwrap();
    manager.dialog("TestSource", "Dialog text", MessageSeverity::Warning).unwrap();

    // Verify messages received by the mock interface
    let received = messages_tracker.lock().unwrap();
    // Note: The default "console" interface also receives these messages.
    // We are only checking our mock_interface.
    assert_eq!(received.len(), 4, "Mock interface should have received 4 messages");

    // Check message types and content (order matters due to VecDeque)
    assert!(matches!(received[0].update_type, UiUpdateType::Log(ref m, s) if m == "Log message" && s == MessageSeverity::Info));
    assert_eq!(received[0].source, "TestSource");

    assert!(matches!(received[1].update_type, UiUpdateType::Status(ref m) if m == "Status update"));
    assert_eq!(received[1].source, "TestSource");

    assert!(matches!(received[2].update_type, UiUpdateType::Progress(p) if (p - 0.5).abs() < f32::EPSILON));
    assert_eq!(received[2].source, "TestSource");

    assert!(matches!(received[3].update_type, UiUpdateType::Dialog(ref m, s) if m == "Dialog text" && s == MessageSeverity::Warning));
    assert_eq!(received[3].source, "TestSource");
}


// --- Test: UI Interface Lifecycle Calls via UnifiedUiManager ---
#[test]
async fn test_ui_interface_lifecycle_calls() {
    let manager = UnifiedUiManager::new(); // Manager is not mutable here due to Arc<Mutex<>> fields
    let mock_interface = MockUiInterface::new("lifecycle_mock");

    // Clone trackers before moving the interface into the manager
    let init_tracker = mock_interface.init_called.clone();
    let update_tracker = mock_interface.update_called.clone(); // Added update tracker
    let finalize_tracker = mock_interface.finalize_called.clone();

    // Register the interface
    manager.register_interface(Box::new(mock_interface)).unwrap();

    // Verify initial state
    assert!(!*init_tracker.lock().unwrap(), "Initialize should not be called before manager.initialize_all()");
    assert!(!*update_tracker.lock().unwrap(), "Update should not be called yet");
    assert!(!*finalize_tracker.lock().unwrap(), "Finalize should not be called before manager.finalize_all()");

    // Call initialize_all on the manager
    manager.initialize_all().expect("Manager initialize_all failed");

    // Verify initialize was called
    assert!(*init_tracker.lock().unwrap(), "Initialize should be called after manager.initialize_all()");
    assert!(!*update_tracker.lock().unwrap(), "Update should still be false after initialize_all");
    assert!(!*finalize_tracker.lock().unwrap(), "Finalize should still be false after initialize_all");

    // Call update_all on the manager
    manager.update_all().expect("Manager update_all failed");
    assert!(*update_tracker.lock().unwrap(), "Update should be called after manager.update_all()");


    // Call finalize_all on the manager
    manager.finalize_all().expect("Manager finalize_all failed");

    // Verify finalize was called
    assert!(*init_tracker.lock().unwrap(), "Initialize should still be true after finalize_all");
    assert!(*finalize_tracker.lock().unwrap(), "Finalize should be called after manager.finalize_all()");
}
