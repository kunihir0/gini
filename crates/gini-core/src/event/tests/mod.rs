// Event system test module
#[cfg(test)]
mod dispatcher_tests;
#[cfg(test)]
mod manager_tests;
#[cfg(test)]
mod types_tests;
#[cfg(test)]
mod error_tests; // Add the new test module

#[cfg(test)]
mod tests {
    use crate::event::EventPriority;

    #[test]
    fn test_event_priority_default() {
        assert_eq!(EventPriority::default(), EventPriority::Normal);
    }

    #[test]
    fn test_event_priority_values() {
        assert_eq!(EventPriority::Low as u32, 0);
        assert_eq!(EventPriority::Normal as u32, 1);
        assert_eq!(EventPriority::High as u32, 2);
        assert_eq!(EventPriority::Critical as u32, 3);
    }

    #[tokio::test]
    async fn test_event_dispatch() {
        use crate::event::{EventResult, Event};
        use std::sync::{Arc, Mutex};
        use crate::event::manager::{EventManager, DefaultEventManager};
        use crate::event::types::TestEvent;

        let event_manager = DefaultEventManager::new();
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        let handler = move |_event: &dyn Event| {
            let mut called = called_clone.lock().unwrap();
            *called = true;
            async move { EventResult::Continue }
        };

        event_manager.register_handler("test_event", Box::new(move |event: &dyn Event| Box::pin(handler(event)))).await;

        let event = TestEvent::new("test_event");
        event_manager.dispatch(&event).await;

        assert!(*called.lock().unwrap(), "Handler should have been called");
    }
}
