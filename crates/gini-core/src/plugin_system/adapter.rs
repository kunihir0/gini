use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::kernel::error::{Error, Result};

/// Adapter trait for providing type-safe interfaces between plugins
pub trait Adapter: Send + Sync {
    /// Get the adapter's type ID
    fn type_id(&self) -> TypeId;
    
    /// Cast to Any to allow dynamic downcasting
    fn as_any(&self) -> &dyn Any;
    
    /// Cast to mutable Any
    fn as_any_mut(&mut self) -> &mut dyn Any;
    
    /// Get the adapter's name
    fn name(&self) -> &str;
}

/// Registry for adapters
#[derive(Default)]
pub struct AdapterRegistry {
    adapters: HashMap<TypeId, Box<dyn Adapter>>,
    names: HashMap<String, TypeId>,
}

impl AdapterRegistry {
    /// Create a new adapter registry
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
            names: HashMap::new(),
        }
    }
    
    /// Register an adapter
    pub fn register<A: Adapter + 'static>(&mut self, adapter: A) -> Result<()> {
        let type_id = adapter.type_id();
        let name = adapter.name().to_string();
        
        if self.adapters.contains_key(&type_id) {
            return Err(Error::Plugin(format!("Adapter already registered for type ID: {:?}", type_id)));
        }
        
        if self.names.contains_key(&name) {
            return Err(Error::Plugin(format!("Adapter already registered with name: {}", name)));
        }
        
        self.adapters.insert(type_id, Box::new(adapter));
        self.names.insert(name, type_id);
        
        Ok(())
    }
    
    /// Get an adapter by type
    pub fn get<A: 'static>(&self) -> Option<&A> {
        let type_id = TypeId::of::<A>();
        self.adapters.get(&type_id).and_then(|adapter| {
            adapter.as_any().downcast_ref::<A>()
        })
    }
    
    /// Get a mutable adapter by type
    pub fn get_mut<A: 'static>(&mut self) -> Option<&mut A> {
        let type_id = TypeId::of::<A>();
        self.adapters.get_mut(&type_id).and_then(|adapter| {
            adapter.as_any_mut().downcast_mut::<A>()
        })
    }
    
    /// Get an adapter by name
    pub fn get_by_name<A: 'static>(&self, name: &str) -> Option<&A> {
        let type_id = self.names.get(name)?;
        let adapter = self.adapters.get(type_id)?;
        adapter.as_any().downcast_ref::<A>()
    }
    
    /// Get a mutable adapter by name
    pub fn get_by_name_mut<A: 'static>(&mut self, name: &str) -> Option<&mut A> {
        // Get the type ID first
        let type_id = match self.names.get(name) {
            Some(id) => *id,
            None => return None,
        };
        
        // Then get and downcast the adapter
        if let Some(adapter) = self.adapters.get_mut(&type_id) {
            adapter.as_any_mut().downcast_mut::<A>()
        } else {
            None
        }
    }
    
    /// Check if an adapter type is registered
    pub fn has<A: 'static>(&self) -> bool {
        self.adapters.contains_key(&TypeId::of::<A>())
    }
    
    /// Check if an adapter name is registered
    pub fn has_name(&self, name: &str) -> bool {
        self.names.contains_key(name)
    }
    
    /// Remove an adapter by type
    pub fn remove<A: 'static>(&mut self) -> Option<Box<dyn Adapter>> {
        let type_id = TypeId::of::<A>();
        if let Some(adapter) = self.adapters.remove(&type_id) {
            // Also remove from names map
            let name = adapter.name().to_string();
            self.names.remove(&name);
            Some(adapter)
        } else {
            None
        }
    }
    
    /// Remove an adapter by name
    pub fn remove_by_name(&mut self, name: &str) -> Option<Box<dyn Adapter>> {
        let type_id = self.names.get(name).cloned()?;
        self.names.remove(name);
        self.adapters.remove(&type_id)
    }
    
    /// Get the number of registered adapters
    pub fn count(&self) -> usize {
        self.adapters.len()
    }
    
    /// Get all adapter names
    pub fn names(&self) -> Vec<&str> {
        self.names.keys().map(|s| s.as_str()).collect()
    }
}

/// Macro to create an adapter implementation
#[macro_export]
macro_rules! define_adapter {
    // Add $impl_type:ident as the second argument
    ($adapter_name:ident, $impl_type:ident, $trait_name:ident) => {
        // The struct name is $adapter_name, generic over the implementation type $impl_type
        // Use $impl_type as the generic parameter name as well
        pub struct $adapter_name<$impl_type: $trait_name + Send + Sync + 'static> {
            name: String,
            // The implementation field holds the concrete type $impl_type
            implementation: $impl_type, // Use $impl_type here
        }
        
        // Implement methods for the wrapper struct
        impl $adapter_name<$impl_type> { // Specify $impl_type here
            // new takes the concrete implementation type
            pub fn new(name: &str, implementation: $impl_type) -> Self {
                Self {
                    name: name.to_string(),
                    implementation, // Store the concrete type
                }
            }
            
            // Return a reference to the concrete implementation
            pub fn implementation(&self) -> &$impl_type {
                &self.implementation
            }

            // Return a mutable reference to the concrete implementation
            pub fn implementation_mut(&mut self) -> &mut $impl_type {
                &mut self.implementation
            }
        }

        // Implement the Adapter trait for the wrapper struct
        impl crate::plugin_system::adapter::Adapter for $adapter_name<$impl_type> { // Specify $impl_type
            fn type_id(&self) -> std::any::TypeId {
                // Use the TypeId of the wrapper struct itself
                std::any::TypeId::of::<$adapter_name<$impl_type>>()
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
            
            fn name(&self) -> &str {
                &self.name
            }
        }
    };
}