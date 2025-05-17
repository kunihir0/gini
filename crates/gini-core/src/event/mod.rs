//! # Gini Core Event System
//!
//! This module provides the core infrastructure for event handling within the Gini framework.
//! It defines fundamental traits like [`Event`] and [`AsyncEventHandler`], common event-related
//! types such as [`EventPriority`] and [`EventResult`], and re-exports key components from
//! its submodules: [`dispatcher`], [`manager`], and [`types`].
//!
//! ## Key Components:
//!
//! - **[`Event`] Trait**: The base trait that all system events must implement. It defines
//!   methods for event identification, priority, cancelability, and cloning.
//! - **[`AsyncEventHandler`] Trait**: A trait for defining asynchronous handlers that can
//!   process events.
//! - **[`EventPriority`] Enum**: Defines different priority levels for events, influencing
//!   their processing order.
//! - **[`EventResult`] Enum**: Represents the outcome of an event handler's execution.
//! - **[`EventId`] Type**: A unique identifier for event types.
//! - **Submodules**:
//!     - `dispatcher`: Contains the [`EventDispatcher`](dispatcher::EventDispatcher)
//!       responsible for low-level event dispatch and handler registration.
//!     - `manager`: Provides the [`EventManager`](manager::EventManager), a higher-level
//!       component for managing the overall event flow and lifecycle.
//!     - `types`: Includes concrete event type definitions used within `gini-core`.
//!     - `error`: Defines error types specific to the event system.
//!
//! The event system is designed to facilitate decoupled communication between different
//! parts of the application, allowing modules to react to occurrences without direct
//! dependencies on the event producers.
pub mod dispatcher;
pub mod error; // New submodule
pub mod manager;
pub mod types;

use std::fmt;
use std::any::Any;

use async_trait::async_trait; // Import async_trait

/// Type for event identifiers
pub type EventId = u64;

/// Event priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    /// Lowest priority, processed last
    Low = 0,
    /// Normal priority, processed in the middle
    Normal = 1,
    /// High priority, processed first
    High = 2,
    /// Critical priority, processed immediately
    Critical = 3,
}

impl Default for EventPriority {
    fn default() -> Self {
        EventPriority::Normal
    }
}

/// Result of event processing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    /// Event was processed successfully and propagation should continue
    Continue,
    /// Event was processed and propagation should stop
    Stop,
}

/// Core event trait
pub trait Event: Any + fmt::Debug + Send + Sync {
    /// Get the name of this event
    fn name(&self) -> &'static str;
    
    /// Get event priority
    fn priority(&self) -> EventPriority {
        EventPriority::Normal
    }
    
    /// Check if this event can be cancelled
    fn is_cancelable(&self) -> bool {
        false
    }
    
    /// Clone this event
    fn clone_event(&self) -> Box<dyn Event>;
    
    /// Cast to Any for downcasting
    fn as_any(&self) -> &dyn Any;
    
    /// Cast to mutable Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Asynchronous event handler trait
#[async_trait]
pub trait AsyncEventHandler: Send + Sync {
    async fn handle(&self, event: &dyn Event) -> EventResult;
}

/// Event handler type alias (now using the async trait)
pub type EventHandler = Box<dyn AsyncEventHandler>;

/// Re-export important types
pub use dispatcher::{EventDispatcher, SharedEventDispatcher, create_dispatcher};
pub use manager::{EventManager, DefaultEventManager, BoxedEvent};
pub use types::{SystemEvent, PluginEvent, StageEvent};
pub use error::EventSystemError; // Re-export EventSystemError

// Test module declaration
#[cfg(test)]
mod tests;