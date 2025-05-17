#![cfg(test)]

use std::any::TypeId;
use crate::event::EventId;
use crate::event::error::EventSystemError;

#[test]
fn test_event_system_error_display() {
    let err_reg_name = EventSystemError::HandlerRegistrationFailedByName {
        event_name: "test.event".to_string(),
        reason: "Already exists".to_string(),
    };
    assert_eq!(
        format!("{}", err_reg_name),
        "Failed to register event handler for event name 'test.event'"
    );

    let err_reg_type = EventSystemError::HandlerRegistrationFailedByType {
        type_id: TypeId::of::<String>(), // Example TypeId
        reason: "Type conflict".to_string(),
    };
    // Note: TypeId's Debug format can be verbose and platform-dependent.
    // We'll check for the presence of the core message.
    assert!(format!("{}", err_reg_type)
        .starts_with("Failed to register event handler for event type ID"));

    let err_unreg = EventSystemError::HandlerUnregistrationFailed {
        id: 123 as EventId,
        reason: "Not found".to_string(),
    };
    assert_eq!(
        format!("{}", err_unreg),
        "Failed to unregister event handler with ID 123: Not found"
    );

    let err_dispatch = EventSystemError::DispatchError {
        event_name: "critical.event".to_string(),
        reason: "Handler panicked".to_string(),
    };
    assert_eq!(
        format!("{}", err_dispatch),
        "Event dispatch failed for event 'critical.event': Handler panicked"
    );

    let err_queue = EventSystemError::QueueOperationFailed {
        operation: "enqueue".to_string(),
        reason: "Queue full".to_string(),
    };
    assert_eq!(
        format!("{}", err_queue),
        "Event queue operation 'enqueue' failed: Queue full"
    );

    let err_data = EventSystemError::InvalidEventData {
        event_name: "user.action".to_string(),
        details: "Missing required field".to_string(),
    };
    assert_eq!(
        format!("{}", err_data),
        "Invalid event data for event 'user.action': Missing required field"
    );

    let err_poisoned = EventSystemError::DispatcherPoisoned {
        component: "handlers_map".to_string(),
    };
    assert_eq!(
        format!("{}", err_poisoned),
        "Attempted to operate on a poisoned event dispatcher component: handlers_map"
    );
    
    let err_internal = EventSystemError::InternalError("Something went wrong".to_string());
    assert_eq!(
        format!("{}", err_internal),
        "Internal event system error: Something went wrong"
    );
}

#[test]
fn test_event_system_error_debug_format() {
    let err = EventSystemError::HandlerRegistrationFailedByName {
        event_name: "debug.event".to_string(),
        reason: "Debug reason".to_string(),
    };
    // Check that Debug format contains the relevant fields.
    // Exact format can vary, so we check for substrings.
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("HandlerRegistrationFailedByName"));
    assert!(debug_str.contains("event_name: \"debug.event\""));
    assert!(debug_str.contains("reason: \"Debug reason\""));
}