use std::fmt::Debug;
use std::sync::Arc;
use async_trait::async_trait;

use crate::event::{Event, EventId, EventResult};
// Ensure BoxFuture is correctly imported or defined if it's local
use crate::event::dispatcher::{self, BoxFuture};
use crate::kernel::component::KernelComponent;
use crate::kernel::error::Result; // Keep for KernelComponent trait methods
// Specific event manager methods will no longer use kernel::error::Result

/// Type alias for boxed event
pub type BoxedEvent = Box<dyn Event>;

/// Event manager interface - simplified for component architecture
#[async_trait]
pub trait EventManager: KernelComponent + Send + Sync { // Add Send + Sync bounds here
    /// Register a handler for events with a specific name
    /// Note: This method uses generics (implicitly via BoxFuture<'a>) and might
    /// need adjustment if full dyn safety is required without this specific signature.
    /// However, let's keep it for now as the dispatcher expects this signature.
    async fn register_handler( // Is async
        &self,
        event_name: &'static str,
        handler: Box<dyn for<'a> Fn(&'a dyn Event) -> BoxFuture<'a> + Send + Sync>
    ) -> EventId;

    // Removed register_type_handler (generic)
    // Removed register_sync_handler (generic wrapper)
    // Removed register_sync_type_handler (generic wrapper)

    /// Unregister a handler by its ID
    async fn unregister_handler(&self, id: EventId) -> bool; // Is async

    /// Dispatch an event
    async fn dispatch(&self, event: &dyn Event) -> EventResult; // Is async

    /// Queue an event for asynchronous processing
    async fn queue_event(&self, event: BoxedEvent); // Is async

    /// Process all queued events
    async fn process_queue(&self) -> usize; // Is async
}

/// Default implementation of EventManager
#[derive(Clone, Debug)]
pub struct DefaultEventManager {
    name: &'static str,
    dispatcher: Arc<dispatcher::SharedEventDispatcher>, // Use Arc directly
}

impl DefaultEventManager {
    /// Create a new default event manager with a shared dispatcher
    pub fn new() -> Self {
        Self {
            name: "DefaultEventManager",
            dispatcher: Arc::new(dispatcher::create_dispatcher()),
        }
    }

    /// Get a reference to the underlying dispatcher Arc
    pub fn dispatcher(&self) -> &Arc<dispatcher::SharedEventDispatcher> {
        &self.dispatcher
    }

    // Add back sync handler registration methods directly on the concrete type
    // if they are needed, as they can't be on the dyn trait.

    /// Register a synchronous handler for events with a specific name (Concrete Impl)
    pub async fn register_sync_handler<F>(
        &self,
        event_name: &'static str,
        handler: F
    ) -> EventId
    where
        F: Fn(&dyn Event) -> EventResult + Send + Sync + 'static,
    {
        let async_handler = dispatcher::sync_event_handler(handler);
        self.register_handler(event_name, async_handler).await
    }

    /// Register a synchronous handler for events of a specific type (Concrete Impl)
    pub async fn register_sync_type_handler<E, F>(
        &self,
        handler: F
    ) -> EventId
    where
        E: Event + 'static,
        F: Fn(&E) -> EventResult + Send + Sync + 'static,
    {
        let async_handler = dispatcher::sync_typed_handler(handler);
        // We need register_type_handler on the dispatcher for this
        self.dispatcher.register_type_handler::<E>(async_handler).await
    }
}

#[async_trait]
impl KernelComponent for DefaultEventManager {
    fn name(&self) -> &'static str { self.name }
    async fn initialize(&self) -> Result<()> { Ok(()) }
    async fn start(&self) -> Result<()> { Ok(()) }
    async fn stop(&self) -> Result<()> { self.process_queue().await; Ok(()) } // No '?' as process_queue is infallible
    // Removed as_any and as_any_mut as they are no longer part of KernelComponent trait
}

#[async_trait]
impl EventManager for DefaultEventManager {
    async fn register_handler( // Is async
        &self,
        event_name: &'static str,
        handler: Box<dyn for<'a> Fn(&'a dyn Event) -> BoxFuture<'a> + Send + Sync>
    ) -> EventId {
        self.dispatcher.register_handler(event_name, handler).await
    }

    // Removed register_type_handler impl
    // Removed register_sync_handler impl (moved to concrete struct)
    // Removed register_sync_type_handler impl (moved to concrete struct)

    async fn unregister_handler(&self, id: EventId) -> bool { // Is async
        self.dispatcher.unregister_handler(id).await
    }

    async fn dispatch(&self, event: &dyn Event) -> EventResult { // Is async
        self.dispatcher.dispatch(event).await
    }

    async fn queue_event(&self, event: BoxedEvent) { // Is async
        self.dispatcher.queue_event(event).await
    }

    async fn process_queue(&self) -> usize { // Is async
        self.dispatcher.process_queue().await
    }
}

impl Default for DefaultEventManager {
    fn default() -> Self {
        Self::new()
    }
}