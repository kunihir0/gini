use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex; // Use tokio's Mutex
use std::fmt; // Import fmt

use crate::kernel::error::{Error, Result};
use crate::stage_manager::{Stage, StageContext, StageResult};

/// Registry for managing stages
// Removed Clone derive as Box<dyn Stage> is not Clone
// Implement Debug manually
pub struct StageRegistry {
    /// Registered stages by ID
    stages: HashMap<String, Box<dyn Stage>>,
}

// Manual Debug implementation
impl fmt::Debug for StageRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format the map keys for a concise debug output
        let stage_ids: Vec<&String> = self.stages.keys().collect();
        f.debug_struct("StageRegistry")
         .field("stages", &stage_ids) // Show registered stage IDs
         .finish()
    }
}


impl StageRegistry {
    /// Create a new stage registry
    pub fn new() -> Self {
        Self {
            stages: HashMap::new(),
        }
    }

    /// Register a stage
    pub fn register_stage(&mut self, stage: Box<dyn Stage>) -> Result<()> {
        let id = stage.id().to_string();

        if self.stages.contains_key(&id) {
            return Err(Error::Stage(format!("Stage already exists with ID: {}", id)));
        }

        self.stages.insert(id, stage);
        Ok(())
    }

    /// Check if a stage with the given ID exists
    pub fn has_stage(&self, id: &str) -> bool {
        self.stages.contains_key(id)
    }

    /// Remove a stage by ID
    pub fn remove_stage(&mut self, id: &str) -> Option<Box<dyn Stage>> {
        self.stages.remove(id)
    }

    /// Get all registered stage IDs
    pub fn get_all_ids(&self) -> Vec<String> {
        self.stages.keys().cloned().collect()
    }

    /// Get the number of registered stages
    pub fn count(&self) -> usize {
        self.stages.len()
    }

    /// Clear all stages
    pub fn clear(&mut self) {
        self.stages.clear();
    }

    /// Execute a specific stage asynchronously (internal method)
    /// Takes &self because Stage::execute takes &self.
    pub async fn execute_stage_internal(&self, id: &str, context: &mut StageContext) -> Result<StageResult> {
        let stage = match self.stages.get(id) {
            Some(stage) => stage,
            None => return Err(Error::Stage(format!("Stage not found with ID: {}", id))),
        };

        println!("Executing stage: {} ({})", stage.name(), id);

        if context.is_dry_run() {
            println!("DRY RUN: {}", stage.dry_run_description(context));
            return Ok(StageResult::Success);
        }

        match stage.execute(context).await {
            Ok(()) => {
                println!("Stage completed successfully: {}", id);
                Ok(StageResult::Success)
            },
            Err(e) => {
                println!("Stage failed: {} - {}", id, e);
                Ok(StageResult::Failure(e.to_string()))
            }
        }
    }
}

impl Default for StageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe stage registry using Tokio's Mutex
#[derive(Clone, Debug)] // Shared can be Clone and Debug as it holds an Arc
pub struct SharedStageRegistry {
    registry: Arc<Mutex<StageRegistry>>, // Use tokio::sync::Mutex
}

impl SharedStageRegistry {
    /// Create a new shared stage registry
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Mutex::new(StageRegistry::new())),
        }
    }

    /// Get a cloned reference to the registry Arc<Mutex>
    pub fn registry(&self) -> Arc<Mutex<StageRegistry>> {
        self.registry.clone()
    }

    /// Register a stage
    pub async fn register_stage(&self, stage: Box<dyn Stage>) -> Result<()> {
        let mut registry = self.registry.lock().await;
        registry.register_stage(stage)
    }

    /// Check if a stage exists
    pub async fn has_stage(&self, id: &str) -> Result<bool> {
        let registry = self.registry.lock().await;
        Ok(registry.has_stage(id))
    }

    /// Execute a specific stage asynchronously
    pub async fn execute_stage(&self, id: &str, context: &mut StageContext) -> Result<StageResult> {
        // Lock the registry immutably first to get the stage reference if needed,
        // or lock mutably if the internal execute needs mutable access.
        // Since execute_stage_internal now takes &self, we lock immutably.
        let registry = self.registry.lock().await;
        registry.execute_stage_internal(id, context).await
    }

    /// Get all registered stage IDs
    pub async fn get_all_ids(&self) -> Result<Vec<String>> {
        let registry = self.registry.lock().await;
        Ok(registry.get_all_ids())
    }
}

impl Default for SharedStageRegistry {
    fn default() -> Self {
        Self::new()
    }
}