use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::sync::Mutex;

use crate::event::{Event, EventPriority, EventResult};
use crate::event::manager::{EventManager, DefaultEventManager, BoxedEvent};
use crate::event::dispatcher::{sync_event_handler}; // Import helpers

// Test event implementation
#[derive(Debug, Clone)]
struct TestEvent {
    pub name: &'static str,
    pub data: String,
    pub priority: EventPriority,
}

impl TestEvent {
    fn new(name: &'static str, data: &str) -> Self {
        Self {
            name,
            data: data.to_string(),
            priority: EventPriority::Normal,
        }
    }

    fn with_priority(mut self, priority: EventPriority) -> Self {
        self.priority = priority;
        self
    }
}

impl Event for TestEvent {
    fn name(&self) -> &'static str {
        self.name
    }

    fn priority(&self) -> EventPriority {
        self.priority
    }

    fn is_cancelable(&self) -> bool {
        false
    }

    fn clone_event(&self) -> Box<dyn Event> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[tokio::test]
async fn test_manager_initialization() {
    // Simple initialization check - just ensure it creates without panic
    let _manager = DefaultEventManager::new();
    // The dispatcher is guaranteed to exist, so no need for is_some() check
}

#[tokio::test]
async fn test_dispatch_event() {
    let manager = DefaultEventManager::new();
    let counter = Arc::new(AtomicU32::new(0));

    // Register a handler using the correct closure format via the manager's method
    let counter_clone = Arc::clone(&counter);
    let handler = sync_event_handler(move |_event| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        EventResult::Continue
    });

    // Register handler using the manager's public API
    manager.register_handler("test.event", handler).await;

    // Create and dispatch event
    let event = TestEvent::new("test.event", "manager dispatch test");
    manager.dispatch(&event).await;

    assert_eq!(counter.load(Ordering::SeqCst), 1, "Handler should have been called");
}

#[tokio::test]
async fn test_boxed_event() {
    // Test boxing and unboxing events
    let original = TestEvent::new("test.event", "boxed event test")
        .with_priority(EventPriority::High);

    let boxed: BoxedEvent = Box::new(original.clone());

    // Verify properties are maintained
    assert_eq!(boxed.name(), "test.event");
    assert_eq!(boxed.priority(), EventPriority::High);

    // Test downcast
    if let Some(unboxed) = boxed.as_any().downcast_ref::<TestEvent>() {
        assert_eq!(unboxed.data, "boxed event test");
    } else {
        panic!("Failed to downcast BoxedEvent to TestEvent");
    }
}

#[tokio::test]
async fn test_queue_event() {
    let manager = DefaultEventManager::new();
    let counter = Arc::new(AtomicU32::new(0));

    // Register a handler
    let counter_clone = Arc::clone(&counter);
    let handler = sync_event_handler(move |_event| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        EventResult::Continue
    });

    manager.register_handler("test.event", handler).await;

    // Queue events
    let event1 = Box::new(TestEvent::new("test.event", "queue test 1"));
    let event2 = Box::new(TestEvent::new("test.event", "queue test 2"));

    manager.queue_event(event1).await;
    manager.queue_event(event2).await;

    // Process queue
    let processed = manager.process_queue().await;

    assert_eq!(processed, 2, "Both events should be processed");
    assert_eq!(counter.load(Ordering::SeqCst), 2, "Handler should be called twice");
}

#[tokio::test]
async fn test_unregister_handler() {
    let manager = DefaultEventManager::new();
    let counter = Arc::new(AtomicU32::new(0));

    // Register a handler
    let counter_clone = Arc::clone(&counter);
    let handler = sync_event_handler(move |_event| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        EventResult::Continue
    });

    let id = manager.register_handler("test.event", handler).await;

    // Create and dispatch an event to verify handler works
    let event1 = TestEvent::new("test.event", "before unregister");
    manager.dispatch(&event1).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1, "Handler should be called");

    // Unregister the handler
    let result = manager.unregister_handler(id).await;
    assert!(result, "Unregister should return true for successful unregistration");

    // Dispatch another event, handler should no longer be called
    let event2 = TestEvent::new("test.event", "after unregister");
    manager.dispatch(&event2).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1, "Handler should not be called after unregistration");
}

// Removed test_register_type_handler as it's not part of EventManager public API

#[tokio::test]
async fn test_multiple_handlers() {
    let manager = DefaultEventManager::new();
    let execution_order = Arc::new(Mutex::new(Vec::<String>::new()));

    // Register handlers that will record their execution order
    for name in ["first", "second", "third"] {
        let name_str = name.to_string();
        let order_tracker = Arc::clone(&execution_order);

        // Create handler function that records execution
        let handler = sync_event_handler(move |_event| {
            let tracker = Arc::clone(&order_tracker);
            let handler_name = name_str.clone();

            // Use try_lock to avoid deadlock in tests
            let _ = tracker.try_lock().map(|mut order| {
                order.push(handler_name.clone());
            });

            EventResult::Continue
        });

        manager.register_handler("multi.event", handler).await;
    }

    // Dispatch event
    let event = TestEvent::new("multi.event", "multiple handlers test");
    manager.dispatch(&event).await;

    // Give a moment for async operations to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify all handlers were executed
    let order = execution_order.lock().await;
    assert_eq!(order.len(), 3, "All three handlers should have executed");
    // Note: Order isn't guaranteed by HashMap iteration within dispatcher,
    // so just check count. If order matters, dispatcher needs adjustment.
}

#[tokio::test]
async fn test_event_data_handling() {
    let manager = DefaultEventManager::new();
    let counter = Arc::new(AtomicU32::new(0));
    let data_recorder = Arc::new(Mutex::new(String::new()));

    // Create a handler that records event data
    let counter_clone = Arc::clone(&counter);
    let data_recorder_clone = Arc::clone(&data_recorder);

    // Register handler that extracts data from event
    let handler = sync_event_handler(move |event| {
        counter_clone.fetch_add(1, Ordering::SeqCst);

        // Try to extract data if this is a TestEvent
        if let Some(test_event) = event.as_any().downcast_ref::<TestEvent>() {
            let _ = data_recorder_clone.try_lock().map(|mut data| {
                *data = test_event.data.clone();
            });
        }

        EventResult::Continue
    });

    manager.register_handler("test.event", handler).await;

    // Dispatch an event with specific data
    let event = TestEvent::new("test.event", "specific test data");
    manager.dispatch(&event).await;

    // Give time for async operation to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Verify the handler correctly processed the event data
    let recorded_data = data_recorder.lock().await.clone();
    assert_eq!(recorded_data, "specific test data", "Handler should extract and record event data");
    assert_eq!(counter.load(Ordering::SeqCst), 1, "Handler should be called once");
}