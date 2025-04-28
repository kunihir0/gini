#[cfg(test)]
mod tests {
    use std::time::SystemTime;
    use std::sync::{Arc, Mutex};

    use crate::ui_bridge::{UiBridge, UiMessage, UiUpdateType, MessageSeverity, UiProvider};

    /// Test basic UiBridge creation and default settings
    #[test]
    fn test_ui_bridge_creation() {
        let bridge = UiBridge::new();
        
        // Verify a default provider exists
        assert!(!bridge.providers.is_empty());
        
        // Verify it's the "console" provider
        assert_eq!(bridge.get_default_provider_name(), Some("console".to_string()));
    }

    /// Test sending a single message
    #[test]
    fn test_send_message() {
        let bridge = UiBridge::new();
        let message = UiMessage {
            update_type: UiUpdateType::Status("test message".to_string()),
            source: "test_source".to_string(),
            timestamp: SystemTime::now(),
        };

        let result = bridge.send_message(message);
        assert!(result.is_ok(), "Sending a message should succeed");
    }

    /// Test convenience methods for sending different message types
    #[test]
    fn test_message_helper_methods() {
        let bridge = UiBridge::new();
        
        // Test progress message
        let progress_result = bridge.progress("test_source", 0.5);
        assert!(progress_result.is_ok(), "Progress message failed");
        
        // Test status message
        let status_result = bridge.status("test_source", "Status update");
        assert!(status_result.is_ok(), "Status message failed");
        
        // Test log message
        let log_result = bridge.log("test_source", "Log entry", MessageSeverity::Info);
        assert!(log_result.is_ok(), "Log message failed");
        
        // Test dialog message
        let dialog_result = bridge.dialog("test_source", "Dialog content", MessageSeverity::Warning);
        assert!(dialog_result.is_ok(), "Dialog message failed");
    }

    /// Test registering a custom provider and setting it as default
    #[test]
    fn test_register_custom_provider() {
        struct TestProvider {}
        
        impl UiProvider for TestProvider {
            fn name(&self) -> &'static str { "test_provider" }
            fn initialize(&mut self) -> Result<(), String> { Ok(()) }
            fn handle_message(&mut self, _: &UiMessage) -> Result<(), String> { Ok(()) }
            fn update(&mut self) -> Result<(), String> { Ok(()) }
            fn finalize(&mut self) -> Result<(), String> { Ok(()) }
            fn supports_interactive(&self) -> bool { true }
        }
        
        let mut bridge = UiBridge::new();
        
        // Register the custom provider
        let register_result = bridge.register_provider(Box::new(TestProvider {}));
        assert!(register_result.is_ok(), "Provider registration failed");
        
        // Set it as default
        let set_default_result = bridge.set_default_provider("test_provider");
        assert!(set_default_result.is_ok(), "Setting default provider failed");
        
        // Verify it's set as default
        assert_eq!(
            bridge.get_default_provider_name(),
            Some("test_provider".to_string()),
            "Default provider should be updated"
        );
    }

    /// Test the message delivery to a mock provider
    #[test]
    fn test_send_message_with_mock_provider() {
        struct MockUiProvider {
            handle_message_called: Arc<Mutex<bool>>,
        }

        impl MockUiProvider {
            fn new() -> (Self, Arc<Mutex<bool>>) {
                let handle_message_called = Arc::new(Mutex::new(false));
                (Self { handle_message_called: handle_message_called.clone() }, handle_message_called)
            }
        }

        impl UiProvider for MockUiProvider {
            fn name(&self) -> &'static str {
                "mock"
            }

            fn initialize(&mut self) -> Result<(), String> {
                Ok(())
            }

            fn handle_message(&mut self, _message: &UiMessage) -> Result<(), String> {
                *self.handle_message_called.lock().unwrap() = true;
                Ok(())
            }

            fn update(&mut self) -> Result<(), String> {
                Ok(())
            }

            fn finalize(&mut self) -> Result<(), String> {
                Ok(())
            }

            fn supports_interactive(&self) -> bool {
                false
            }
        }

        let (mock_provider, handle_message_called) = MockUiProvider::new();

        let mut bridge = UiBridge::new();
        bridge.register_provider(Box::new(mock_provider)).unwrap();
        bridge.set_default_provider("mock").unwrap();

        let message = UiMessage {
            update_type: UiUpdateType::Status("test message".to_string()),
            source: "test".to_string(),
            timestamp: SystemTime::now(),
        };

        bridge.send_message(message).unwrap();

        assert!(*handle_message_called.lock().unwrap(), "Message handler should have been called");
    }

    /// Test UiBridge initialization and update lifecycle
    #[test]
    fn test_lifecycle_methods() {
        let mut bridge = UiBridge::new();
        
        // Test initialization
        assert!(bridge.initialize().is_ok(), "UiBridge initialization failed");
        
        // Test update
        assert!(bridge.update().is_ok(), "UiBridge update failed");
        
        // Test finalization
        assert!(bridge.finalize().is_ok(), "UiBridge finalization failed");
    }

    /// Test UiMessage equality implementation
    #[test]
    fn test_message_equality() {
        let timestamp = SystemTime::now();
        
        // Create two identical messages
        let msg1 = UiMessage {
            update_type: UiUpdateType::Status("test".to_string()),
            source: "source".to_string(),
            timestamp,
        };
        
        let msg2 = UiMessage {
            update_type: UiUpdateType::Status("test".to_string()),
            source: "source".to_string(),
            timestamp,
        };
        
        // Create a different message
        let msg3 = UiMessage {
            update_type: UiUpdateType::Log("different".to_string(), MessageSeverity::Info),
            source: "source".to_string(),
            timestamp,
        };
        
        // Check equality implementation
        assert_eq!(msg1.update_type, msg2.update_type, "Identical update types should be equal");
        assert_ne!(msg1.update_type, msg3.update_type, "Different update types should not be equal");
    }
}