use std::any::TypeId;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use crate::event::{Event, EventHandler, EventId, EventResult};
use crate::kernel::error::{Error, Result};

/// Event dispatcher for managing and dispatching events
pub struct EventDispatcher {
    /// Handlers registered for events
    handlers: HashMap<&'static str, Vec<(EventId, EventHandler)>>,
    /// TypeId-based handlers for specific event types
    type_handlers: HashMap<TypeId, Vec<(EventId, EventHandler)>>,
    /// Next available handler ID
    next_handler_id: EventId,
    /// Event queue for asynchronous processing
    event_queue: VecDeque<Box<dyn Event>>,
}

impl EventDispatcher {
    /// Create a new event dispatcher
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            type_handlers: HashMap::new(),
            next_handler_id: 1,
            event_queue: VecDeque::new(),
        }
    }
    
    /// Register a handler for a specific event name
    pub fn register_handler<F>(&mut self, event_name: &'static str, handler: F) -> EventId
    where
        F: Fn(&dyn Event) -> EventResult + Send + Sync + 'static,
    {
        let id = self.next_handler_id;
        self.next_handler_id += 1;
        
        let handler_box = Box::new(handler) as EventHandler;
        
        self.handlers
            .entry(event_name)
            .or_default()
            .push((id, handler_box));
        
        id
    }
    
    /// Register a handler for a specific event type
    pub fn register_type_handler<E, F>(&mut self, handler: F) -> EventId
    where
        E: Event + 'static,
        F: Fn(&E) -> EventResult + Send + Sync + 'static,
    {
        let id = self.next_handler_id;
        self.next_handler_id += 1;
        
        let type_id = TypeId::of::<E>();
        let wrapped_handler: EventHandler = Box::new(move |event| {
            if let Some(specific_event) = event.as_any().downcast_ref::<E>() {
                handler(specific_event)
            } else {
                EventResult::Continue
            }
        });
        
        self.type_handlers
            .entry(type_id)
            .or_default()
            .push((id, wrapped_handler));
        
        id
    }
    
    /// Unregister a handler by its ID
    pub fn unregister_handler(&mut self, id: EventId) -> bool {
        let mut found = false;
        
        // Remove from name-based handlers
        for (_, handlers) in self.handlers.iter_mut() {
            let len_before = handlers.len();
            handlers.retain(|(handler_id, _)| *handler_id != id);
            if handlers.len() < len_before {
                found = true;
            }
        }
        
        // Remove from type-based handlers
        for (_, handlers) in self.type_handlers.iter_mut() {
            let len_before = handlers.len();
            handlers.retain(|(handler_id, _)| *handler_id != id);
            if handlers.len() < len_before {
                found = true;
            }
        }
        
        found
    }
    
    /// Dispatch an event synchronously
    pub fn dispatch(&self, event: &dyn Event) -> EventResult {
        let mut result = EventResult::Continue;
        
        // Handlers for this specific event name
        if let Some(handlers) = self.handlers.get(event.name()) {
            for (_, handler) in handlers {
                match handler(event) {
                    EventResult::Continue => {},
                    EventResult::Stop => {
                        result = EventResult::Stop;
                        break;
                    }
                }
            }
        }
        
        // If we should stop propagation, don't call type handlers
        if result == EventResult::Stop {
            return result;
        }
        
        // Type-based handlers
        if let Some(type_handlers) = self.type_handlers.get(&TypeId::of::<dyn Event>()) {
            for (_, handler) in type_handlers {
                match handler(event) {
                    EventResult::Continue => {},
                    EventResult::Stop => {
                        result = EventResult::Stop;
                        break;
                    }
                }
            }
        }
        
        result
    }
    
    /// Queue an event for asynchronous processing
    pub fn queue_event(&mut self, event: Box<dyn Event>) {
        self.event_queue.push_back(event);
    }
    
    /// Process all queued events
    pub fn process_queue(&mut self) -> usize {
        let mut count = 0;
        
        while let Some(event) = self.event_queue.pop_front() {
            self.dispatch(&*event);
            count += 1;
        }
        
        count
    }
    
    /// Get the number of queued events
    pub fn queue_size(&self) -> usize {
        self.event_queue.len()
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// Thread-safe shared event dispatcher for application-wide events
pub struct SharedEventDispatcher {
    dispatcher: Arc<Mutex<EventDispatcher>>
}

impl SharedEventDispatcher {
    /// Create a new shared event dispatcher
    pub fn new() -> Self {
        Self {
            dispatcher: Arc::new(Mutex::new(EventDispatcher::new()))
        }
    }
    
    /// Get a clone of the dispatcher Arc for sharing
    pub fn clone_dispatcher(&self) -> Arc<Mutex<EventDispatcher>> {
        self.dispatcher.clone()
    }
    
    /// Dispatch an event
    pub fn dispatch(&self, event: &dyn Event) -> Result<EventResult> {
        match self.dispatcher.lock() {
            Ok(dispatcher) => Ok(dispatcher.dispatch(event)),
            Err(_) => Err(Error::Event("Failed to lock dispatcher".into()))
        }
    }
    
    /// Queue an event for asynchronous processing
    pub fn queue_event(&self, event: Box<dyn Event>) -> Result<()> {
        match self.dispatcher.lock() {
            Ok(mut dispatcher) => {
                dispatcher.queue_event(event);
                Ok(())
            },
            Err(_) => Err(Error::Event("Failed to lock dispatcher".into()))
        }
    }
    
    /// Process all queued events
    pub fn process_queue(&self) -> Result<usize> {
        match self.dispatcher.lock() {
            Ok(mut dispatcher) => Ok(dispatcher.process_queue()),
            Err(_) => Err(Error::Event("Failed to lock dispatcher".into()))
        }
    }
    
    /// Register a handler for a specific event name
    pub fn register_handler<F>(&self, event_name: &'static str, handler: F) -> Result<EventId>
    where
        F: Fn(&dyn Event) -> EventResult + Send + Sync + 'static,
    {
        match self.dispatcher.lock() {
            Ok(mut dispatcher) => Ok(dispatcher.register_handler(event_name, handler)),
            Err(_) => Err(Error::Event("Failed to lock dispatcher".into()))
        }
    }
    
    /// Register a handler for a specific event type
    pub fn register_type_handler<E, F>(&self, handler: F) -> Result<EventId>
    where
        E: Event + 'static,
        F: Fn(&E) -> EventResult + Send + Sync + 'static,
    {
        match self.dispatcher.lock() {
            Ok(mut dispatcher) => Ok(dispatcher.register_type_handler(handler)),
            Err(_) => Err(Error::Event("Failed to lock dispatcher".into()))
        }
    }
}

impl Default for SharedEventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// Create a new event dispatcher instance
pub fn create_dispatcher() -> SharedEventDispatcher {
    SharedEventDispatcher::new()
}

// Global functions for handling events
// These are convenience methods for common event operations
// Note: In a real application, you would typically pass the EventDispatcher
// as a dependency to components that need it, rather than using globals

/// Create a simple event handler function for common event types
pub fn create_handler<F>(handler_fn: F) -> Box<dyn Fn(&dyn Event) -> EventResult + Send + Sync>
where
    F: Fn(&dyn Event) -> EventResult + Send + Sync + 'static,
{
    Box::new(handler_fn)
}

/// Create a typed event handler function
pub fn create_typed_handler<E, F>(handler_fn: F) -> Box<dyn Fn(&E) -> EventResult + Send + Sync>
where
    E: Event + 'static,
    F: Fn(&E) -> EventResult + Send + Sync + 'static,
{
    Box::new(handler_fn)
}