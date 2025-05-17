#[cfg(test)]
mod tests {
    use std::time::SystemTime;
    use std::sync::{Arc, Mutex};
    use std::fmt::Debug;

    use crate::ui_bridge::{UnifiedUiManager, UiMessage, UserInput, UiUpdateType, MessageSeverity, UnifiedUiInterface, error::UiBridgeError};

    /// Test basic UnifiedUiManager creation and default settings
    #[test]
    fn test_unified_ui_manager_creation() {
        let manager = UnifiedUiManager::new();
        
        // Verify a default interface exists
        let interfaces_guard = manager.interfaces.lock().unwrap();
        assert!(!interfaces_guard.is_empty(), "Manager should have at least one interface after creation.");
        
        // Verify it's the "console" interface
        assert_eq!(manager.get_default_interface_name(), Some("console".to_string()));
    }

    /// Test sending a single message
    #[test]
    fn test_broadcast_message() {
        let manager = UnifiedUiManager::new();
        let message = UiMessage {
            update_type: UiUpdateType::Status("test message".to_string()),
            source: "test_source".to_string(),
            timestamp: SystemTime::now(),
        };

        let result = manager.broadcast_message(message);
        assert!(result.is_ok(), "Broadcasting a message should succeed");
    }

    /// Test convenience methods for sending different message types
    #[test]
    fn test_message_helper_methods() {
        let manager = UnifiedUiManager::new();
        
        // Test progress message
        let progress_result = manager.progress("test_source", 0.5);
        assert!(progress_result.is_ok(), "Progress message failed");
        
        // Test status message
        let status_result = manager.status("test_source", "Status update");
        assert!(status_result.is_ok(), "Status message failed");
        
        // Test log message
        let log_result = manager.log("test_source", "Log entry", MessageSeverity::Info);
        assert!(log_result.is_ok(), "Log message failed");
        
        // Test dialog message
        let dialog_result = manager.dialog("test_source", "Dialog content", MessageSeverity::Warning);
        assert!(dialog_result.is_ok(), "Dialog message failed");
    }

    #[derive(Debug)] // Added Debug derive
    struct TestInterface {
        // Minimal state for testing if needed
        init_called: bool,
        finalize_called: bool,
        update_called: bool,
    }
    
    impl TestInterface {
        fn new() -> Self {
            Self {
                init_called: false,
                finalize_called: false,
                update_called: false,
            }
        }
    }
        
    impl UnifiedUiInterface for TestInterface {
        fn name(&self) -> &str { "test_interface" }
        fn initialize(&mut self) -> Result<(), UiBridgeError> { self.init_called = true; Ok(()) }
        fn handle_message(&mut self, _: &UiMessage) -> Result<(), UiBridgeError> { Ok(()) }
        fn send_input(&mut self, _input: UserInput) -> Result<(), UiBridgeError> { Ok(()) }
        fn update(&mut self) -> Result<(), UiBridgeError> { self.update_called = true; Ok(()) }
        fn finalize(&mut self) -> Result<(), UiBridgeError> { self.finalize_called = true; Ok(()) }
        fn supports_interactive(&self) -> bool { true }
    }

    /// Test registering a custom interface and setting it as default
    #[test]
    fn test_register_custom_interface() {
        let manager = UnifiedUiManager::new(); // manager is not mutable here
        
        // Register the custom interface
        let register_result = manager.register_interface(Box::new(TestInterface::new()));
        assert!(register_result.is_ok(), "Interface registration failed: {:?}", register_result.err());
        
        // Set it as default
        let set_default_result = manager.set_default_interface("test_interface");
        assert!(set_default_result.is_ok(), "Setting default interface failed: {:?}", set_default_result.err());
        
        // Verify it's set as default
        assert_eq!(
            manager.get_default_interface_name(),
            Some("test_interface".to_string()),
            "Default interface should be updated"
        );
    }

    /// Test the message delivery to a mock interface
    #[test]
    fn test_broadcast_message_with_mock_interface() {
        #[derive(Debug)]
        struct MockUiInterface {
            handle_message_called: Arc<Mutex<bool>>,
            last_message: Arc<Mutex<Option<UiMessage>>>,
        }

        impl MockUiInterface {
            fn new() -> (Self, Arc<Mutex<bool>>, Arc<Mutex<Option<UiMessage>>>) {
                let handle_message_called = Arc::new(Mutex::new(false));
                let last_message = Arc::new(Mutex::new(None));
                (
                    Self {
                        handle_message_called: handle_message_called.clone(),
                        last_message: last_message.clone(),
                    },
                    handle_message_called,
                    last_message
                )
            }
        }

        impl UnifiedUiInterface for MockUiInterface {
            fn name(&self) -> &str { "mock_interface" }
            fn initialize(&mut self) -> Result<(), UiBridgeError> { Ok(()) }
            fn handle_message(&mut self, message: &UiMessage) -> Result<(), UiBridgeError> {
                *self.handle_message_called.lock().unwrap() = true;
                *self.last_message.lock().unwrap() = Some(message.clone());
                Ok(())
            }
            fn send_input(&mut self, _input: UserInput) -> Result<(), UiBridgeError> { Ok(()) }
            fn update(&mut self) -> Result<(), UiBridgeError> { Ok(()) }
            fn finalize(&mut self) -> Result<(), UiBridgeError> { Ok(()) }
            fn supports_interactive(&self) -> bool { false }
        }

        let (mock_interface, handle_message_called, _last_message) = MockUiInterface::new();

        let manager = UnifiedUiManager::new();
        manager.register_interface(Box::new(mock_interface)).unwrap();
        // No need to set default for broadcast

        let message_content = "test message for mock".to_string();
        let message = UiMessage {
            update_type: UiUpdateType::Status(message_content.clone()),
            source: "test_mock_source".to_string(),
            timestamp: SystemTime::now(),
        };

        manager.broadcast_message(message.clone()).unwrap();

        assert!(*handle_message_called.lock().unwrap(), "Message handler should have been called on mock interface");
        // Optionally, check if the correct message was received
        // let received_msg_opt = last_message.lock().unwrap();
        // assert!(received_msg_opt.is_some(), "Mock interface should have received a message");
        // if let Some(received_msg) = received_msg_opt.as_ref() {
        //     assert_eq!(received_msg.source, message.source);
        //     // Add more specific checks if needed
        // }
    }

    /// Test UnifiedUiManager lifecycle methods
    #[test]
    fn test_lifecycle_methods() {
        let manager = UnifiedUiManager::new(); // manager is not mutable here
        
        // Test initialization
        assert!(manager.initialize_all().is_ok(), "UnifiedUiManager initialization failed");
        
        // Test update
        assert!(manager.update_all().is_ok(), "UnifiedUiManager update failed");
        
        // Test finalization
        assert!(manager.finalize_all().is_ok(), "UnifiedUiManager finalization failed");
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