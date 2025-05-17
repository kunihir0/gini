//! # Gini Core Event System Errors
//!
//! Defines error types specific to the Gini Event System.
//!
//! This module includes [`EventError`], the primary enum encompassing various
//! errors that can occur during event processing, dispatch, or management,
//! such as issues with event handling, listener registration, or event
//! lifecycle problems.
use std::any::TypeId;
use crate::event::EventId; // Assuming EventId is u64 or similar
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventSystemError {
    #[error("Failed to register event handler for event name '{event_name}'")]
    HandlerRegistrationFailedByName {
        event_name: String, // &'static str might be tricky with thiserror if not careful
        reason: String,
    },

    #[error("Failed to register event handler for event type ID '{type_id:?}'")]
    HandlerRegistrationFailedByType {
        type_id: TypeId,
        reason: String,
    },

    #[error("Failed to unregister event handler with ID {id}: {reason}")]
    HandlerUnregistrationFailed {
        id: EventId,
        reason: String,
    },

    #[error("Event dispatch failed for event '{event_name}': {reason}")]
    DispatchError {
        event_name: String,
        reason: String,
    },

    #[error("Event queue operation '{operation}' failed: {reason}")]
    QueueOperationFailed {
        operation: String, // e.g., "enqueue", "process_queue"
        reason: String,
    },

    #[error("Invalid event data for event '{event_name}': {details}")]
    InvalidEventData {
        event_name: String,
        details: String,
    },
    
    #[error("Attempted to operate on a poisoned event dispatcher component: {component}")]
    DispatcherPoisoned {
        component: String, // e.g., "handlers_map", "event_queue"
    },

    #[error("Internal event system error: {0}")]
    InternalError(String),
}