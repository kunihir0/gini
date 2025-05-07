use std::sync::Arc;

use super::{UiConnector, UiMessage};

/// Manages connections to various UI frontends.
///
/// This struct holds references to active UI connectors and provides
/// methods to register new connectors and broadcast messages to all
/// connected UIs.
#[derive(Debug, Default)]
pub struct UIManager {
    connectors: Vec<Arc<dyn UiConnector>>,
}

impl UIManager {
    /// Creates a new, empty `UIManager`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new UI connector with the manager.
    ///
    /// The connector must implement the `UiConnector` trait and be wrapped
    /// in an `Arc` for shared ownership.
    pub fn register_connector(&mut self, connector: Arc<dyn UiConnector>) {
        // Clone the Arc to keep ownership for the println! below, while moving one reference into the Vec
        self.connectors.push(connector.clone());
        // Optionally, send an initial "connected" message or similar
        // connector.handle_message(UiMessage::System("Connected".to_string()));
        println!("UIManager: Registered connector: {}", connector.name()); // Temporary logging
    }

    /// Broadcasts a message to all registered UI connectors.
    ///
    /// Each connector's `handle_message` method will be called with the
    /// provided message. Errors from individual connectors are ignored
    /// during broadcast (consider adding error handling/logging).
    pub fn broadcast_message(&self, message: UiMessage) {
        println!("UIManager: Broadcasting message: {:?}", message); // Temporary logging
        for connector in &self.connectors {
            // Clone the message if it needs to be owned by each connector,
            // depending on the UiMessage type and handle_message signature.
            // If UiMessage is Clone, this is straightforward.
            connector.handle_message(message.clone());
        }
    }

    // Potential future method:
    // pub fn get_connector(&self, name: &str) -> Option<&Arc<dyn UiConnector>> {
    //     self.connectors.iter().find(|c| c.name() == name)
    // }

    // Potential future method for removing connectors:
    // pub fn unregister_connector(&mut self, name: &str) {
    //     self.connectors.retain(|c| c.name() != name);
    // }
}

#[cfg(test)]
mod tests {
    // Import necessary types for tests
    use super::*; // Keep only one super import
    use crate::ui_bridge::{UserInput, UiMessage, UiUpdateType, MessageSeverity};
    use std::sync::{Arc, Mutex};
    use std::time::SystemTime; // Import SystemTime for message creation

    // A simple mock connector for testing
    #[derive(Debug)]
    struct MockConnector {
        name: String,
        received_messages: Arc<Mutex<Vec<UiMessage>>>,
    }

    impl MockConnector {
         fn new(name: &str, received_messages: Arc<Mutex<Vec<UiMessage>>>) -> Self {
            Self { name: name.to_string(), received_messages }
        }
    }

    impl UiConnector for MockConnector {
        fn name(&self) -> &str {
            &self.name
        }

        fn handle_message(&self, message: UiMessage) {
             println!("MockConnector [{}]: Received message: {:?}", self.name, message); // Added print
            let mut messages = self.received_messages.lock().unwrap();
            messages.push(message);
        }

        // Basic implementation, can be expanded if needed for tests
        fn send_input(&self, _input: UserInput) -> Result<(), String> {
            println!("MockConnector [{}]: send_input called (no-op)", self.name); // Added print
            Ok(())
        }
    }

    #[test]
    fn test_new_manager_is_empty() {
        let manager = UIManager::new();
        assert!(manager.connectors.is_empty());
    }

    #[test]
    fn test_register_connector() {
        let mut manager = UIManager::new();
        let received_messages = Arc::new(Mutex::new(Vec::new()));
        let connector = Arc::new(MockConnector::new("test1", received_messages.clone()));

        assert_eq!(manager.connectors.len(), 0);
        manager.register_connector(connector.clone());
        assert_eq!(manager.connectors.len(), 1);
        assert_eq!(manager.connectors[0].name(), "test1");
    }

    #[test]
    fn test_broadcast_message() {
        let mut manager = UIManager::new();
        let received1 = Arc::new(Mutex::new(Vec::new()));
        let received2 = Arc::new(Mutex::new(Vec::new()));

        let connector1 = Arc::new(MockConnector::new("conn1", received1.clone()));
        let connector2 = Arc::new(MockConnector::new("conn2", received2.clone()));

        manager.register_connector(connector1);
        manager.register_connector(connector2);

        // Create a UiMessage using the struct definition
        let message = UiMessage {
            update_type: UiUpdateType::Log("Hello, world!".to_string(), MessageSeverity::Info),
            source: "test".to_string(),
            timestamp: SystemTime::now(),
        };
        manager.broadcast_message(message.clone());

        // Check if both connectors received the message
        {
            let messages1 = received1.lock().unwrap();
            assert_eq!(messages1.len(), 1);
            // Update matches! to check the struct field and enum variant
            assert!(matches!(messages1[0].update_type, UiUpdateType::Log(ref s, MessageSeverity::Info) if s == "Hello, world!"));
        }
        {
            let messages2 = received2.lock().unwrap();
            assert_eq!(messages2.len(), 1);
             assert!(matches!(messages2[0].update_type, UiUpdateType::Log(ref s, MessageSeverity::Info) if s == "Hello, world!"));
        }

         // Create another UiMessage
         let message2 = UiMessage {
             update_type: UiUpdateType::Log("System Alert".to_string(), MessageSeverity::Warning), // Use Warning for variety
             source: "test".to_string(),
             timestamp: SystemTime::now(),
         };
         manager.broadcast_message(message2.clone());

         {
            let messages1 = received1.lock().unwrap();
            assert_eq!(messages1.len(), 2);
            // Update matches! for the second message
            assert!(matches!(messages1[1].update_type, UiUpdateType::Log(ref s, MessageSeverity::Warning) if s == "System Alert"));
        }
        {
            let messages2 = received2.lock().unwrap();
            assert_eq!(messages2.len(), 2);
            assert!(matches!(messages2[1].update_type, UiUpdateType::Log(ref s, MessageSeverity::Warning) if s == "System Alert"));
        }
    }
}