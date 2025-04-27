pub mod dispatcher;
pub mod manager;
pub mod types;

use std::fmt;
use std::any::Any;

use async_trait::async_trait; // Import async_trait
use std::future::Future; // For async trait return type
use std::pin::Pin; // For async trait return type

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

// Test module declaration
#[cfg(test)]
mod tests;