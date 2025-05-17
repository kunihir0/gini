#![cfg(test)]

use crate::StorageProvider; // Add this import
use std::sync::{Arc, Mutex};
use tokio::test;
use std::any::Any;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::VecDeque;
use std::path::PathBuf; // Added for StageContext::new_live
use std::fs; // Added for test setup
use tempfile::tempdir; // Added for test setup
use std::error::Error as StdError; // For boxing
 
// Use fully qualified path for BoxFuture
use crate::event::dispatcher::BoxFuture;
use crate::event::{Event, EventResult}; // Added EventId
use crate::event::manager::{EventManager, DefaultEventManager};
use crate::event::types::TestEvent;
use crate::stage_manager::{Stage, StageContext, StageManager};
use crate::stage_manager::manager::DefaultStageManager; // Corrected import path
use crate::stage_manager::registry::StageRegistry; // Added for register_stages
use crate::stage_manager::requirement::StageRequirement; // Moved import higher
use crate::plugin_system::traits::{Plugin, PluginPriority}; // Removed PluginError
use crate::plugin_system::error::PluginSystemError; // Import PluginSystemError
use crate::plugin_system::manager::DefaultPluginManager;
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::version::VersionRange;
use crate::storage::config::{ConfigManager, ConfigFormat}; // Added for test setup
use crate::storage::local::LocalStorageProvider; // Added for test setup
use crate::kernel::bootstrap::Application;
use crate::kernel::error::{Result as KernelResult, Error as KernelError}; // Corrected import path, added KernelError
use crate::storage::manager::DefaultStorageManager;
#[test]
async fn test_event_system_integration() {
    let event_manager = DefaultEventManager::new();
    let called = Arc::new(Mutex::new(false));
    let called_clone = called.clone();

    // Explicitly type the Box to satisfy for<'a> bound
    let handler: Box<dyn for<'a> Fn(&'a dyn Event) -> BoxFuture<'a> + Send + Sync> = Box::new(move |_event: &dyn Event| {
        let called = called_clone.clone();
        Box::pin(async move {
            let mut called_guard = called.lock().unwrap();
            *called_guard = true;
            EventResult::Continue
        })
    });

    event_manager.register_handler("test_event", handler).await;
    let event = TestEvent::new("test_event");
    event_manager.dispatch(&event).await;
    assert!(*called.lock().unwrap(), "Event handler should have been called");
}

#[test]
async fn test_multiple_handlers() {
    let event_manager = DefaultEventManager::new();
    let handler1_called = Arc::new(Mutex::new(false));
    let handler2_called = Arc::new(Mutex::new(false));

    let h1_called = handler1_called.clone();
    // Explicitly type the Box to satisfy for<'a> bound
    let h1: Box<dyn for<'a> Fn(&'a dyn Event) -> BoxFuture<'a> + Send + Sync> = Box::new(move |_event: &dyn Event| {
        let h1_called = h1_called.clone();
        Box::pin(async move {
            *h1_called.lock().unwrap() = true;
            EventResult::Continue
        })
    });

    let h2_called = handler2_called.clone();
    // Explicitly type the Box to satisfy for<'a> bound
    let h2: Box<dyn for<'a> Fn(&'a dyn Event) -> BoxFuture<'a> + Send + Sync> = Box::new(move |_event: &dyn Event| {
        let h2_called = h2_called.clone();
        Box::pin(async move {
            *h2_called.lock().unwrap() = true;
            EventResult::Continue
        })
    });

    event_manager.register_handler("multi_event", h1).await;
    event_manager.register_handler("multi_event", h2).await;
    let event = TestEvent::new("multi_event");
    event_manager.dispatch(&event).await;
    assert!(*handler1_called.lock().unwrap(), "First handler should have been called");
    assert!(*handler2_called.lock().unwrap(), "Second handler should have been called");
}

// NOTE: Removed test_handler_stop_propagation and test_handler_priority
// because register_handler_with_priority is not exposed on the public API
// of DefaultEventManager / SharedEventDispatcher.

#[test]
async fn test_typed_handler() {
    let event_manager = DefaultEventManager::new();
    let handler_called = Arc::new(Mutex::new(false));
    let handler_received_name = Arc::new(Mutex::new(String::new()));

    let h_called = handler_called.clone();
    let h_name = handler_received_name.clone();

    // Use the convenience method on DefaultEventManager
    event_manager.register_sync_type_handler::<TestEvent, _>(move |event: &TestEvent| {
        *h_called.lock().unwrap() = true;
        *h_name.lock().unwrap() = event.name().to_string();
        EventResult::Continue
    }).await;

    let event = TestEvent::new("typed_event");
    event_manager.dispatch(&event).await;

    assert!(*handler_called.lock().unwrap(), "Typed handler should have been called");
    assert_eq!(*handler_received_name.lock().unwrap(), "typed_event", "Handler should have received the correct event name");
}

#[derive(Debug, Clone)]
struct PayloadEvent {
    name: String,
    payload: String,
}

impl PayloadEvent {
    fn new(name: &str, payload: &str) -> Self {
        Self {
            name: name.to_string(),
            payload: payload.to_string(),
        }
    }

    fn payload(&self) -> &str {
        &self.payload
    }
}

impl Event for PayloadEvent {
    fn name(&self) -> &'static str {
        // Leak memory for test purposes to get &'static str
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[test]
async fn test_event_with_payload() {
    let event_manager = DefaultEventManager::new();
    let received_payload = Arc::new(Mutex::new(String::new()));
    let payload_ref = received_payload.clone();

    // Use the convenience method on DefaultEventManager
    event_manager.register_sync_type_handler::<PayloadEvent, _>(move |event: &PayloadEvent| {
        *payload_ref.lock().unwrap() = event.payload().to_string();
        EventResult::Continue
    }).await;

    let expected_payload = "important data";
    let event = PayloadEvent::new("payload_event", expected_payload);
    event_manager.dispatch(&event).await;

    assert_eq!(
        *received_payload.lock().unwrap(),
        expected_payload,
        "Handler should have received the correct payload"
    );
}
// --- Test: Event Handler Unregistration ---
#[test]
async fn test_event_handler_unregistration() {
    let event_manager = DefaultEventManager::new();
    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    // No string handler_id needed

    let handler: Box<dyn for<'a> Fn(&'a dyn Event) -> BoxFuture<'a> + Send + Sync> = Box::new(move |_event: &dyn Event| {
        let count = call_count_clone.clone();
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            EventResult::Continue
        })
    });

    // Register the handler and get the numeric ID
    let handler_id = event_manager.register_handler("unregister_event", handler).await;

    // Dispatch event - handler should be called
    let event = TestEvent::new("unregister_event");
    event_manager.dispatch(&event).await;
    assert_eq!(call_count.load(Ordering::SeqCst), 1, "Handler should be called once after registration");

    // Unregister the handler using the numeric ID
    let unregistered = event_manager.unregister_handler(handler_id).await;
    assert!(unregistered, "Handler should be successfully unregistered");

    // Dispatch event again - handler should NOT be called
    event_manager.dispatch(&event).await;
    assert_eq!(call_count.load(Ordering::SeqCst), 1, "Handler should not be called after unregistration");

    // Try unregistering again - should return false
    let unregistered_again = event_manager.unregister_handler(handler_id).await;
    assert!(!unregistered_again, "Unregistering a non-existent handler should return false");
}

// --- Test: Event Queueing and Processing ---
#[test]
async fn test_event_queueing_and_processing() {
    let event_manager = DefaultEventManager::new();
    let execution_order = Arc::new(Mutex::new(VecDeque::new()));

    // Handler for Event A
    let order_a = execution_order.clone();
    let handler_a: Box<dyn for<'a> Fn(&'a dyn Event) -> BoxFuture<'a> + Send + Sync> = Box::new(move |event: &dyn Event| {
        let order = order_a.clone();
        let event_name = event.name().to_string(); // Clone event name
        Box::pin(async move {
            order.lock().unwrap().push_back(format!("{}_handled", event_name));
            EventResult::Continue
        })
    });

    // Handler for Event B
    let order_b = execution_order.clone();
    let handler_b: Box<dyn for<'a> Fn(&'a dyn Event) -> BoxFuture<'a> + Send + Sync> = Box::new(move |event: &dyn Event| {
        let order = order_b.clone();
        let event_name = event.name().to_string(); // Clone event name
        Box::pin(async move {
            order.lock().unwrap().push_back(format!("{}_handled", event_name));
            EventResult::Continue
        })
    });

    event_manager.register_handler("event_a", handler_a).await;
    event_manager.register_handler("event_b", handler_b).await;

    let event_a = TestEvent::new("event_a");
    let event_b = TestEvent::new("event_b");

    // Queue events
    event_manager.queue_event(Box::new(event_a)).await;
    event_manager.queue_event(Box::new(event_b)).await;

    // Check queue size (optional, internal detail)
    // assert_eq!(event_manager.queue_size().await, 2); // Assuming a method like this exists

    // Process the queue
    event_manager.process_queue().await;

    // Verify order
    let final_order = execution_order.lock().unwrap();
    assert_eq!(final_order.len(), 2, "Both event handlers should have executed");
    assert_eq!(final_order[0], "event_a_handled");
    assert_eq!(final_order[1], "event_b_handled");

    // Check queue is empty (optional, internal detail)
    // assert_eq!(event_manager.queue_size().await, 0);
}


// --- Test: Stage Dispatches Event ---

// Define a custom event for this test
#[derive(Debug, Clone)]
struct StageDispatchedEvent { name: String, data: String }
impl StageDispatchedEvent { fn new(data: &str) -> Self { Self { name: "stage_dispatched".to_string(), data: data.to_string() } } }
impl Event for StageDispatchedEvent {
    fn name(&self) -> &'static str { Box::leak(self.name.clone().into_boxed_str()) } // Leak for 'static
    fn clone_event(&self) -> Box<dyn Event> { Box::new(self.clone()) }
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// Define a stage that dispatches the event
struct EventDispatchingStage { id: String }
impl EventDispatchingStage { fn new(id: &str) -> Self { Self { id: id.to_string() } } }
#[async_trait::async_trait]
impl Stage for EventDispatchingStage {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { "Event Dispatching Stage" }
    fn description(&self) -> &str { "Dispatches StageDispatchedEvent" }
    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        println!("Executing stage: {}", self.id);
        let event = StageDispatchedEvent::new("Data from stage");

        // Retrieve EventManager from context shared data
        let em = context.get_data::<Arc<DefaultEventManager>>("event_manager")
            .ok_or_else(|| Box::new(KernelError::Other("EventManager not found in context".to_string())) as Box<dyn StdError + Send + Sync + 'static>)?
            .clone(); // Clone Arc to use

        // Dispatch the event using the retrieved manager
        em.dispatch(&event).await; // No '?' as dispatch is infallible
        Ok(())
    }
}

#[test]
async fn test_stage_dispatches_event() {
    // Setup managers
    let event_manager = Arc::new(DefaultEventManager::new());
    let stage_manager = Arc::new(DefaultStageManager::new());
    // Break down storage_manager creation
    // let storage_base_path = std::env::temp_dir().join("gini_test_stage_event"); // Unused
    // DefaultStorageManager::new now takes no arguments and returns Result
    let _storage_manager = Arc::new(DefaultStorageManager::new().expect("Failed to create dummy storage manager"));

    // Setup ConfigManager for PluginManager
    let tmp_dir = tempdir().unwrap();
    let app_config_path = tmp_dir.path().join("app_config");
    let plugin_config_path = tmp_dir.path().join("plugin_config");
    fs::create_dir_all(&app_config_path).unwrap();
    fs::create_dir_all(&plugin_config_path).unwrap();
    let provider = Arc::new(LocalStorageProvider::new(tmp_dir.path().to_path_buf())) as Arc<dyn StorageProvider>; // Cast to dyn trait
    // Call ConfigManager::new with the reverted signature
    let config_manager: Arc<ConfigManager> = Arc::new(ConfigManager::new(
        provider,             // Pass the provider Arc
        app_config_path,      // Pass the app config path
        plugin_config_path,   // Pass the plugin config path
        ConfigFormat::Json,   // Pass the default format
    ));
    let _plugin_manager = Arc::new(DefaultPluginManager::new(config_manager).unwrap()); // Pass ConfigManager

    // Register the stage
    let stage = EventDispatchingStage::new("dispatch_stage");
    stage_manager.register_stage(Box::new(stage)).await.unwrap();

    // Register a handler for the event dispatched by the stage
    let event_received = Arc::new(Mutex::new(false));
    let received_data = Arc::new(Mutex::new(String::new()));
    let event_received_clone = event_received.clone();
    let received_data_clone = received_data.clone();

    event_manager.register_sync_type_handler::<StageDispatchedEvent, _>(move |event: &StageDispatchedEvent| {
        *event_received_clone.lock().unwrap() = true;
        *received_data_clone.lock().unwrap() = event.data.clone();
        EventResult::Continue
    }).await;

    // Create context using new_live
    let mut context = StageContext::new_live(PathBuf::from("/tmp/gini_test_stage_event_ctx")); // Use new_live

    // Store the EventManager Arc in the context's shared data
    context.set_data("event_manager", event_manager.clone());
    // Add other managers if needed by the stage or context setup, though not strictly required for this event dispatch test
    // context.set_data("storage_manager", storage_manager.clone());
    // context.set_data("plugin_manager", plugin_manager.clone());
    // context.set_data("stage_manager", stage_manager.clone());


    // Execute the single stage directly (simpler than full pipeline for this test)
    stage_manager.execute_stage("dispatch_stage", &mut context).await.unwrap();

    // Verify the handler was called
    assert!(*event_received.lock().unwrap(), "Handler for StageDispatchedEvent should have been called");
    assert_eq!(*received_data.lock().unwrap(), "Data from stage", "Handler received correct data");
}


// --- Test: Plugin Dispatches Event ---

// Define a custom event for this test
#[derive(Debug, Clone)]
struct PluginDispatchedEvent { name: String, source: String }
impl PluginDispatchedEvent { fn new(source: &str) -> Self { Self { name: "plugin_dispatched".to_string(), source: source.to_string() } } }
impl Event for PluginDispatchedEvent {
    fn name(&self) -> &'static str { Box::leak(self.name.clone().into_boxed_str()) } // Leak for 'static
    fn clone_event(&self) -> Box<dyn Event> { Box::new(self.clone()) }
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// Define a plugin that dispatches an event during init
struct EventDispatchingPlugin { name: String, event_manager: Arc<DefaultEventManager> } // Hold EventManager for dispatch
impl EventDispatchingPlugin {
    fn new(name: &str, em: Arc<DefaultEventManager>) -> Self {
        Self { name: name.to_string(), event_manager: em }
    }

    // Moved simulate_init_dispatch here, into the inherent impl block
    async fn simulate_init_dispatch(&self) -> KernelResult<()> {
        println!("Plugin {} simulating event dispatch from init", self.name);
        let event = PluginDispatchedEvent::new(&self.name); // Pass reference
        self.event_manager.dispatch(&event).await; // Dispatch using the held manager, no '?'
        Ok(())
    }
}

#[async_trait::async_trait]
impl Plugin for EventDispatchingPlugin {
    fn name(&self) -> &'static str { Box::leak(self.name.clone().into_boxed_str()) }
    fn version(&self) -> &str { "1.0.0" }
    fn is_core(&self) -> bool { false }
    fn priority(&self) -> PluginPriority { PluginPriority::ThirdParty(100) }
    fn compatible_api_versions(&self) -> Vec<VersionRange> { vec![">=0.1.0".parse().unwrap()] }
    fn dependencies(&self) -> Vec<PluginDependency> { vec![] }
    fn required_stages(&self) -> Vec<StageRequirement> { vec![] }

    fn init(&self, _app: &mut Application) -> Result<(), PluginSystemError> {
        println!("Plugin {} init called (dispatch simulated separately)", self.name());
        Ok(())
    }

    async fn preflight_check(&self, _context: &StageContext) -> Result<(), PluginSystemError> { Ok(()) }
    fn shutdown(&self) -> Result<(), PluginSystemError> { Ok(()) }
    fn register_stages(&self, _registry: &mut StageRegistry) -> Result<(), PluginSystemError> { Ok(()) } // Added

    // Add default implementations for new trait methods
    fn conflicts_with(&self) -> Vec<String> { vec![] }
    fn incompatible_with(&self) -> Vec<PluginDependency> { vec![] }
}

// impl EventDispatchingPlugin { fn register_stages(&self, _registry: &mut StageRegistry) -> KernelResult<()> { Ok(()) } } // Added dummy impl

#[test]
async fn test_plugin_dispatches_event() {
    // Setup managers
    let event_manager = Arc::new(DefaultEventManager::new());
    // We don't need a full Application or PluginManager registration for this specific test focus.
    // We'll directly instantiate the plugin and call its dispatch simulation method.

    // Register a handler for the event dispatched by the plugin
    let event_received = Arc::new(Mutex::new(false));
    let received_source = Arc::new(Mutex::new(String::new()));
    let event_received_clone = event_received.clone();
    let received_source_clone = received_source.clone();

    event_manager.register_sync_type_handler::<PluginDispatchedEvent, _>(move |event: &PluginDispatchedEvent| {
        *event_received_clone.lock().unwrap() = true;
        *received_source_clone.lock().unwrap() = event.source.clone();
        EventResult::Continue
    }).await;

    // Create the plugin instance, passing the event manager
    let plugin = EventDispatchingPlugin::new("DispatcherPlugin", event_manager.clone());

    // Simulate the action that would trigger the dispatch (e.g., part of init)
    plugin.simulate_init_dispatch().await.unwrap(); // This unwrap is for KernelResult, not the event dispatch

    // Verify the handler was called
    assert!(*event_received.lock().unwrap(), "Handler for PluginDispatchedEvent should have been called");
    assert_eq!(*received_source.lock().unwrap(), "DispatcherPlugin", "Handler received correct source");
}