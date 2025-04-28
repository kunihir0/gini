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
use crate::ui_bridge::{UiBridge, UiProvider, UiMessage, UiUpdateType, MessageSeverity};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
// SystemTime is already imported

// --- Mock UI Provider for Testing ---

#[derive(Debug)]
struct MockUiProvider {
    name: &'static str,
    init_called: Arc<Mutex<bool>>,
    finalize_called: Arc<Mutex<bool>>,
    messages_received: Arc<Mutex<VecDeque<UiMessage>>>, // Use VecDeque to check order easily
}

impl MockUiProvider {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            init_called: Arc::new(Mutex::new(false)),
            finalize_called: Arc::new(Mutex::new(false)),
            messages_received: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    // Helper methods to access tracker state for assertions
    fn was_initialized(&self) -> bool {
        *self.init_called.lock().unwrap()
    }

    fn was_finalized(&self) -> bool {
        *self.finalize_called.lock().unwrap()
    }

    fn get_received_messages(&self) -> VecDeque<UiMessage> {
        self.messages_received.lock().unwrap().clone()
    }
}

impl UiProvider for MockUiProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    fn initialize(&mut self) -> Result<(), String> {
        println!("MockUiProvider '{}' initialized", self.name);
        *self.init_called.lock().unwrap() = true;
        Ok(())
    }

    fn handle_message(&mut self, message: &UiMessage) -> Result<(), String> {
        println!("MockUiProvider '{}' received message: {:?}", self.name, message.update_type);
        self.messages_received.lock().unwrap().push_back(message.clone());
        Ok(())
    }

    fn update(&mut self) -> Result<(), String> {
        // No-op for mock
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), String> {
        println!("MockUiProvider '{}' finalized", self.name);
        *self.finalize_called.lock().unwrap() = true;
        Ok(())
    }

    fn supports_interactive(&self) -> bool {
        false // Mock doesn't support interactive mode
    }
}


// --- Test: UI Provider Registration and Default ---
#[test]
async fn test_ui_provider_registration_and_default() {
    let mut bridge = UiBridge::new(); // Creates with default "console" provider

    // Verify initial default provider
    assert_eq!(bridge.get_default_provider_name(), Some("console".to_string()), "Initial default should be console");

    // Create and register a mock provider
    let mock_provider = MockUiProvider::new("mock_ui");
    let mock_provider_name = mock_provider.name().to_string();
    bridge.register_provider(Box::new(mock_provider)).expect("Failed to register mock provider");

    // Verify default is still console (register doesn't automatically set default if one exists)
    assert_eq!(bridge.get_default_provider_name(), Some("console".to_string()), "Default should still be console after registering mock");

    // Set the mock provider as default
    bridge.set_default_provider(&mock_provider_name).expect("Failed to set mock provider as default");

    // Verify the default provider name is now the mock provider's name
    assert_eq!(bridge.get_default_provider_name(), Some(mock_provider_name.clone()), "Default provider name should be updated to mock");

    // Test setting a non-existent provider as default
    let result = bridge.set_default_provider("non_existent_provider");
    assert!(result.is_err(), "Setting a non-existent provider as default should fail");
    assert_eq!(bridge.get_default_provider_name(), Some(mock_provider_name), "Default provider should remain mock after failed set");
}


// --- Test: UI Bridge Message Dispatch ---
#[test]
async fn test_ui_bridge_message_dispatch() {
    let mut bridge = UiBridge::new();
    let mock_provider = MockUiProvider::new("message_dispatch_mock");
    let mock_name = mock_provider.name().to_string();
    let messages_tracker = mock_provider.messages_received.clone(); // Clone Arc for later access

    // Register and set mock as default
    bridge.register_provider(Box::new(mock_provider)).unwrap();
    bridge.set_default_provider(&mock_name).unwrap();

    // Send messages using helper methods
    bridge.log("TestSource", "Log message", MessageSeverity::Info).unwrap();
    bridge.status("TestSource", "Status update").unwrap();
    bridge.progress("TestSource", 0.5).unwrap();
    bridge.dialog("TestSource", "Dialog text", MessageSeverity::Warning).unwrap();

    // Verify messages received by the mock provider
    let received = messages_tracker.lock().unwrap();
    assert_eq!(received.len(), 4, "Should have received 4 messages");

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


// --- Test: UI Provider Lifecycle Calls ---
#[test]
async fn test_ui_provider_lifecycle_calls() {
    let mut bridge = UiBridge::new();
    let mock_provider = MockUiProvider::new("lifecycle_mock");

    // Clone trackers before moving the provider into the bridge
    let init_tracker = mock_provider.init_called.clone();
    let finalize_tracker = mock_provider.finalize_called.clone();

    // Register the provider
    bridge.register_provider(Box::new(mock_provider)).unwrap();

    // Verify initial state
    assert!(!*init_tracker.lock().unwrap(), "Initialize should not be called before bridge.initialize()");
    assert!(!*finalize_tracker.lock().unwrap(), "Finalize should not be called before bridge.finalize()");

    // Call initialize on the bridge
    bridge.initialize().expect("Bridge initialize failed");

    // Verify initialize was called
    assert!(*init_tracker.lock().unwrap(), "Initialize should be called after bridge.initialize()");
    assert!(!*finalize_tracker.lock().unwrap(), "Finalize should still be false after initialize");

    // Call finalize on the bridge
    bridge.finalize().expect("Bridge finalize failed");

    // Verify finalize was called
    assert!(*init_tracker.lock().unwrap(), "Initialize should still be true after finalize");
    assert!(*finalize_tracker.lock().unwrap(), "Finalize should be called after bridge.finalize()");
}
