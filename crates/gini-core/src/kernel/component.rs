use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use async_trait::async_trait;
use crate::kernel::error::{Error, Result};

/// Core component lifecycle trait for all kernel components
#[async_trait]
pub trait KernelComponent: Any + Send + Sync + Debug { // Add Any bound here
    fn name(&self) -> &'static str;
    async fn initialize(&self) -> Result<()>;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}

/// Simplified registry storing components as Arc<dyn KernelComponent>
#[derive(Default, Debug)] // Can derive Debug now
pub struct DependencyRegistry {
    // Store Arc<dyn KernelComponent> keyed by the *concrete* type's TypeId
    instances: HashMap<TypeId, Arc<dyn KernelComponent>>,
}

impl DependencyRegistry {
    /// Create a new empty dependency registry
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }

    /// Register a component instance. Stores it as Arc<dyn KernelComponent>.
    /// Keyed by the TypeId of the concrete type V.
    pub fn register_instance<V>(&mut self, instance: Arc<V>)
    where
        V: KernelComponent + 'static, // V must impl KernelComponent and be 'static
    {
        let type_id = TypeId::of::<V>();
        // The instance Arc<V> is cast to Arc<dyn KernelComponent> for storage
        self.instances.insert(type_id, instance);
    }

    /// Get a component instance by the TypeId of its concrete type.
    /// Returns Arc<dyn KernelComponent>.
    pub fn get_component_by_id(&self, type_id: &TypeId) -> Option<Arc<dyn KernelComponent>> {
        self.instances.get(type_id).cloned()
    }

    /// Get a component instance by concrete type T.
    /// Returns Arc<T> if found and downcast is successful.
    pub fn get_concrete<T: KernelComponent + 'static>(&self) -> Option<Arc<T>> {
        let type_id = TypeId::of::<T>();
        self.instances
            .get(&type_id)
            // arc_kc is Arc<dyn KernelComponent>. Since KernelComponent: Any, we can downcast.
            .and_then(|arc_kc| {
                // Clone the Arc<dyn KernelComponent>
                let cloned_arc = arc_kc.clone();
                // Treat it as Arc<dyn Any + Send + Sync> to use downcast
                let arc_any: Arc<dyn Any + Send + Sync> = cloned_arc;
                // Attempt downcast using static method call
                Arc::downcast::<T>(arc_any).ok()
            })
    }


    /// Get all registered component trait objects.
    pub fn get_all_components(&self) -> Vec<Arc<dyn KernelComponent>> {
        self.instances.values().cloned().collect()
    }

     /// Get TypeIds of all registered components.
    pub fn get_registered_ids(&self) -> Vec<TypeId> {
        self.instances.keys().cloned().collect()
    }

    /// Clear all instances.
    pub fn clear(&mut self) {
        self.instances.clear();
    }
}