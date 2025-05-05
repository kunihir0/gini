use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::sync::Mutex;

use crate::event::{Event, EventPriority, EventResult};
use crate::event::dispatcher::{EventDispatcher, create_dispatcher, sync_event_handler, sync_typed_handler};

// Test event implementation
#[derive(Debug, Clone)]
struct TestEvent {
    pub name: &'static str,
    pub data: String,
    pub priority: EventPriority,
    pub cancelable: bool,
}

impl TestEvent {
    fn new(name: &'static str, data: &str) -> Self {
        Self {
            name,
            data: data.to_string(),
            priority: EventPriority::Normal,
            cancelable: false,
        }
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
        self.cancelable
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
async fn test_handler_registration_and_dispatch() {
    let mut dispatcher = EventDispatcher::new();
    let counter = Arc::new(AtomicU32::new(0));

    // Register handler using sync_event_handler helper
    let counter_clone = Arc::clone(&counter);
    let handler_fn = sync_event_handler(move |event: &dyn Event| {
        assert_eq!(event.name(), "test.event");
        counter_clone.fetch_add(1, Ordering::SeqCst);
        EventResult::Continue
    });

    let handler_id = dispatcher.register_handler("test.event", handler_fn);
    assert!(handler_id > 0, "Handler ID should be positive");

    // Dispatch an event
    let event = TestEvent::new("test.event", "test data");
    let result = dispatcher.dispatch_internal(&event).await;

    // Verify handler was called
    assert_eq!(result, EventResult::Continue);
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Dispatch a different event (should not trigger handler)
    let other_event = TestEvent::new("other.event", "other data");
    dispatcher.dispatch_internal(&other_event).await;

    // Counter should still be 1
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_typed_handler_registration_and_dispatch() {
    let mut dispatcher = EventDispatcher::new();
    let counter = Arc::new(AtomicU32::new(0));
    let data_recorder = Arc::new(Mutex::new(String::new()));

    // Register typed handler using sync_typed_handler helper
    let counter_clone = Arc::clone(&counter);
    let data_recorder_clone = Arc::clone(&data_recorder);
    let handler_fn = sync_typed_handler(move |event: &TestEvent| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        // Use try_lock for simplicity in test, avoid await inside sync closure
        let _ = data_recorder_clone.try_lock().map(|mut data| *data = event.data.clone());
        EventResult::Continue
    });

    let handler_id = dispatcher.register_type_handler::<TestEvent>(handler_fn);
    assert!(handler_id > 0, "Handler ID should be positive");

    // Dispatch an event of the correct type
    let event = TestEvent::new("test.event", "typed data");
    let result = dispatcher.dispatch_internal(&event).await;

    // Verify handler was called and processed type correctly
    assert_eq!(result, EventResult::Continue);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert_eq!(*data_recorder.lock().await, "typed data");

    // Dispatch an event of a different type (should not trigger typed handler)
    #[derive(Debug, Clone)] struct AnotherEvent;
    impl Event for AnotherEvent {
        fn name(&self) -> &'static str { "another.event" }
        fn clone_event(&self) -> Box<dyn Event> { Box::new(self.clone()) }
        fn as_any(&self) -> &dyn std::any::Any { self }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    }
    dispatcher.dispatch_internal(&AnotherEvent).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1, "Typed handler should not run for different event type");

}


#[tokio::test]
async fn test_handler_unregistration() {
    let mut dispatcher = EventDispatcher::new();
    let counter = Arc::new(AtomicU32::new(0));

    // Register handler
    let counter_clone = Arc::clone(&counter);
    let handler_fn = sync_event_handler(move |_event| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        EventResult::Continue
    });

    let handler_id = dispatcher.register_handler("test.event", handler_fn);

    // Unregister the handler
    let result = dispatcher.unregister_handler(handler_id);
    assert!(result, "Unregistration should return true for success");

    // Dispatch an event - should not trigger handler
    let event = TestEvent::new("test.event", "data");
    dispatcher.dispatch_internal(&event).await;

    assert_eq!(counter.load(Ordering::SeqCst), 0, "Handler should not be called after unregistering");

    // Unregistering non-existent handler should fail
    let result_nonexistent = dispatcher.unregister_handler(999);
    assert!(!result_nonexistent, "Non-existent handler unregistration should fail");
}

#[tokio::test]
async fn test_stopping_propagation() {
    let mut dispatcher = EventDispatcher::new();
    let counter1 = Arc::new(AtomicU32::new(0));
    let counter2 = Arc::new(AtomicU32::new(0));

    // First handler - stops propagation
    let counter1_clone = Arc::clone(&counter1);
    let handler1 = sync_event_handler(move |_event| {
        counter1_clone.fetch_add(1, Ordering::SeqCst);
        EventResult::Stop
    });

    // Second handler - should not be called
    let counter2_clone = Arc::clone(&counter2);
    let handler2 = sync_event_handler(move |_event| {
        counter2_clone.fetch_add(1, Ordering::SeqCst);
        EventResult::Continue
    });

    // Register handlers (order matters)
    dispatcher.register_handler("test.event", handler1);
    dispatcher.register_handler("test.event", handler2);

    // Dispatch event
    let event = TestEvent::new("test.event", "stop propagation test");
    let result = dispatcher.dispatch_internal(&event).await;

    assert_eq!(result, EventResult::Stop);
    assert_eq!(counter1.load(Ordering::SeqCst), 1, "First handler should be called");
    assert_eq!(counter2.load(Ordering::SeqCst), 0, "Second handler should not be called");
}


#[tokio::test]
async fn test_event_queue() {
    let mut dispatcher = EventDispatcher::new();
    let counter = Arc::new(AtomicU32::new(0));

    // Register handler
    let counter_clone = Arc::clone(&counter);
    let handler = sync_event_handler(move |event| {
        if event.name() == "test.event" {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }
        EventResult::Continue
    });

    dispatcher.register_handler("test.event", handler);

    // Queue events
    dispatcher.queue_event(Box::new(TestEvent::new("test.event", "queue test 1")));
    dispatcher.queue_event(Box::new(TestEvent::new("test.event", "queue test 2")));
    dispatcher.queue_event(Box::new(TestEvent::new("other.event", "should not trigger")));

    assert_eq!(dispatcher.queue_size(), 3);

    // Process queue
    let processed = dispatcher.process_queue_internal().await;

    assert_eq!(processed, 3, "All 3 events should be processed");
    assert_eq!(counter.load(Ordering::SeqCst), 2, "Only test.event handlers should be triggered");
    assert_eq!(dispatcher.queue_size(), 0, "Queue should be empty after processing");
}

#[tokio::test]
async fn test_shared_dispatcher_registration_and_dispatch() {
    // Create a shared dispatcher
    let shared_dispatcher = create_dispatcher();
    let counter = Arc::new(AtomicU32::new(0));

    // Register handler using the shared interface
    let counter_clone = Arc::clone(&counter);
    let handler = sync_event_handler(move |_event| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        EventResult::Continue
    });

    // Register handler via shared dispatcher's method
    let handler_id = shared_dispatcher.register_handler("test.event", handler).await.unwrap();
    assert!(handler_id > 0);


    // Create and dispatch event via shared dispatcher
    let event = TestEvent::new("test.event", "shared dispatcher test");
    let result = shared_dispatcher.dispatch(&event).await.unwrap();

    assert_eq!(result, EventResult::Continue);
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Clone dispatcher and check they share state
    let dispatcher_clone = shared_dispatcher.clone();
    let event2 = TestEvent::new("test.event", "shared dispatcher clone test");
    let _result2 = dispatcher_clone.dispatch(&event2).await.unwrap();

    assert_eq!(counter.load(Ordering::SeqCst), 2, "Clone should share handler registry");

    // Test unregistering via shared dispatcher
    let unregistered = shared_dispatcher.unregister_handler(handler_id).await.unwrap();
    assert!(unregistered);

    let event3 = TestEvent::new("test.event", "after unregister");
    shared_dispatcher.dispatch(&event3).await.unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 2, "Handler should not run after unregistering via shared dispatcher");

}