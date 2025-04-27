use std::any::TypeId;
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex; // Use tokio's Mutex
use std::fmt; // Import fmt

use async_trait::async_trait;
use crate::event::{Event, AsyncEventHandler, EventHandler, EventId, EventResult};
use crate::kernel::error::{Error, Result};

// This type represents an owned future that returns EventResult
pub type BoxFuture<'a> = Pin<Box<dyn Future<Output = EventResult> + Send + 'a>>;

//--------------------------------------------------
// EventDispatcher (Internal, wrapped by SharedEventDispatcher)
//--------------------------------------------------

/// Event dispatcher for managing and dispatching events (Internal Implementation)
pub struct EventDispatcher {
    handlers: HashMap<&'static str, Vec<(EventId, Box<dyn AsyncEventHandler>)>>,
    type_handlers: HashMap<TypeId, Vec<(EventId, Box<dyn AsyncEventHandler>)>>,
    next_handler_id: EventId,
    event_queue: VecDeque<Box<dyn Event>>,
}

// Manual Debug implementation for EventDispatcher
impl fmt::Debug for EventDispatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name_handler_count: usize = self.handlers.values().map(|v| v.len()).sum();
        let type_handler_count: usize = self.type_handlers.values().map(|v| v.len()).sum();
        f.debug_struct("EventDispatcher")
         .field("name_handlers_count", &name_handler_count)
         .field("type_handlers_count", &type_handler_count)
         .field("next_handler_id", &self.next_handler_id)
         .field("event_queue_size", &self.event_queue.len())
         .finish()
    }
}

/// Simple handler for events with a specific name (Internal Helper)
struct SimpleHandler {
    handler: Box<dyn Fn(&dyn Event) -> BoxFuture<'_> + Send + Sync>,
}
impl fmt::Debug for SimpleHandler {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_struct("SimpleHandler").finish_non_exhaustive() }
}
#[async_trait]
impl AsyncEventHandler for SimpleHandler {
    async fn handle(&self, event: &dyn Event) -> EventResult { (self.handler)(event).await }
}

/// Handler for typed events that will check the type (Internal Helper)
struct TypedEventHandler<E: Event + 'static> {
    handler: Box<dyn Fn(&E) -> BoxFuture<'_> + Send + Sync>,
}
impl<E: Event + 'static> fmt::Debug for TypedEventHandler<E> {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_struct("TypedEventHandler").finish_non_exhaustive() }
}
#[async_trait]
impl<E: Event + 'static> AsyncEventHandler for TypedEventHandler<E> {
    async fn handle(&self, event: &dyn Event) -> EventResult {
        if let Some(e) = event.as_any().downcast_ref::<E>() { (self.handler)(e).await }
        else { EventResult::Continue }
    }
}

// Single implementation block for EventDispatcher
impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            type_handlers: HashMap::new(),
            next_handler_id: 1,
            event_queue: VecDeque::new(),
        }
    }

    pub fn register_handler( &mut self, event_name: &'static str, handler: Box<dyn Fn(&dyn Event) -> BoxFuture<'_> + Send + Sync> ) -> EventId {
        let id = self.next_handler_id; self.next_handler_id += 1;
        let handler = SimpleHandler { handler };
        self.handlers.entry(event_name).or_default().push((id, Box::new(handler)));
        id
    }

    pub fn register_type_handler<E: Event + 'static>( &mut self, handler: Box<dyn Fn(&E) -> BoxFuture<'_> + Send + Sync> ) -> EventId {
        let id = self.next_handler_id; self.next_handler_id += 1;
        let type_id = TypeId::of::<E>();
        let handler = TypedEventHandler { handler };
        self.type_handlers.entry(type_id).or_default().push((id, Box::new(handler)));
        id
    }

     pub fn unregister_handler(&mut self, id: EventId) -> bool {
        let mut found = false;
        self.handlers.values_mut().for_each(|handlers| {
            let len_before = handlers.len(); handlers.retain(|(h_id, _)| *h_id != id);
            if handlers.len() < len_before { found = true; }
        });
        self.type_handlers.values_mut().for_each(|handlers| {
             let len_before = handlers.len(); handlers.retain(|(h_id, _)| *h_id != id);
             if handlers.len() < len_before { found = true; }
        });
        found
    }

    pub async fn dispatch_internal(&self, event: &dyn Event) -> EventResult {
        let mut result = EventResult::Continue;
        if let Some(handlers) = self.handlers.get(event.name()) {
            for (_, handler) in handlers {
                match handler.handle(event).await {
                    EventResult::Continue => {},
                    EventResult::Stop => { result = EventResult::Stop; break; }
                }
            }
        }
        if result == EventResult::Stop { return result; }
        if let Some(handlers) = self.type_handlers.get(&event.as_any().type_id()) {
            for (_, handler) in handlers {
                match handler.handle(event).await {
                    EventResult::Continue => {},
                    EventResult::Stop => { result = EventResult::Stop; break; }
                }
            }
        }
        result
    }

    pub fn queue_event(&mut self, event: Box<dyn Event>) { self.event_queue.push_back(event); }

    pub async fn process_queue_internal(&mut self) -> usize {
        let mut count = 0;
        // Process events one by one from the queue
        while let Some(event) = self.event_queue.pop_front() {
             // Temporarily borrow self immutably to call dispatch_internal
            let dispatcher_ref = &*self;
            dispatcher_ref.dispatch_internal(&*event).await;
            count += 1;
        }
        count
    }

    pub fn queue_size(&self) -> usize { self.event_queue.len() }
}

impl Default for EventDispatcher { fn default() -> Self { Self::new() } }


//--------------------------------------------------
// SharedEventDispatcher (Public API)
//--------------------------------------------------

/// Thread-safe shared event dispatcher using Tokio Mutex
#[derive(Clone)] // Only Clone
pub struct SharedEventDispatcher {
    dispatcher: Arc<Mutex<EventDispatcher>>
}

// Manual Debug impl for SharedEventDispatcher
impl fmt::Debug for SharedEventDispatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedEventDispatcher").finish_non_exhaustive()
    }
}

// Single implementation block for SharedEventDispatcher
impl SharedEventDispatcher {
    pub fn new() -> Self { Self { dispatcher: Arc::new(Mutex::new(EventDispatcher::new())) } }

    pub fn clone_dispatcher(&self) -> Arc<Mutex<EventDispatcher>> { self.dispatcher.clone() }

    pub async fn dispatch(&self, event: &dyn Event) -> Result<EventResult> {
        let dispatcher = self.dispatcher.lock().await;
        Ok(dispatcher.dispatch_internal(event).await)
    }

    pub async fn queue_event(&self, event: Box<dyn Event>) -> Result<()> {
        let mut dispatcher = self.dispatcher.lock().await;
        dispatcher.queue_event(event); Ok(())
    }

    pub async fn process_queue(&self) -> Result<usize> {
        let mut dispatcher = self.dispatcher.lock().await;
        Ok(dispatcher.process_queue_internal().await)
    }

    pub async fn register_handler( &self, event_name: &'static str, handler: Box<dyn Fn(&dyn Event) -> BoxFuture<'_> + Send + Sync> ) -> Result<EventId> {
        let mut dispatcher = self.dispatcher.lock().await;
        Ok(dispatcher.register_handler(event_name, handler))
    }

    pub async fn register_type_handler<E: Event + 'static>( &self, handler: Box<dyn Fn(&E) -> BoxFuture<'_> + Send + Sync> ) -> Result<EventId> {
        let mut dispatcher = self.dispatcher.lock().await;
        Ok(dispatcher.register_type_handler::<E>(handler))
    }

    pub async fn unregister_handler(&self, id: EventId) -> Result<bool> {
        let mut dispatcher = self.dispatcher.lock().await;
        Ok(dispatcher.unregister_handler(id))
    }
}

impl Default for SharedEventDispatcher { fn default() -> Self { Self::new() } }

//--------------------------------------------------
// Helper Functions
//--------------------------------------------------

/// Create a new event dispatcher instance
pub fn create_dispatcher() -> SharedEventDispatcher { SharedEventDispatcher::new() }

/// Helper function to create synchronous handlers that are compatible with async system
pub fn sync_event_handler<F>(f: F) -> Box<dyn Fn(&dyn Event) -> BoxFuture<'_> + Send + Sync>
where F: Fn(&dyn Event) -> EventResult + Send + Sync + 'static {
    Box::new(move |event| { let result = f(event); Box::pin(async move { result }) })
}

/// Helper function to create typed synchronous handlers
pub fn sync_typed_handler<E, F>(f: F) -> Box<dyn Fn(&E) -> BoxFuture<'_> + Send + Sync>
where E: Event + 'static, F: Fn(&E) -> EventResult + Send + Sync + 'static {
    Box::new(move |event| { let result = f(event); Box::pin(async move { result }) })
}