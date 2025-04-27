pub mod dispatcher;
pub mod types;

use std::fmt;
use std::any::Any;

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

/// Event handler function type
pub type EventHandler = Box<dyn Fn(&dyn Event) -> EventResult + Send + Sync>;

/// Re-export important types
pub use dispatcher::{EventDispatcher, SharedEventDispatcher, create_dispatcher};
pub use types::{SystemEvent, PluginEvent, StageEvent};