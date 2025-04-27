use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::kernel::error::{Error, Result};
use crate::stage_manager::{Stage, StageContext, StageResult};

/// Registry for managing stages
pub struct StageRegistry {
    /// Registered stages by ID
    stages: HashMap<String, Box<dyn Stage>>,
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
    
    /// Execute a specific stage
    pub fn execute_stage(&mut self, id: &str, context: &mut StageContext) -> Result<StageResult> {
        // Find the stage
        let stage = match self.stages.get_mut(id) {
            Some(stage) => stage,
            None => return Err(Error::Stage(format!("Stage not found with ID: {}", id))),
        };
        
        // Log the stage execution
        println!("Executing stage: {} ({})", stage.name(), id);
        
        if context.is_dry_run() {
            println!("DRY RUN: {}", stage.dry_run_description(context));
            return Ok(StageResult::Success); // Always succeed in dry run mode
        }
        
        // Execute the stage
        match stage.execute(context) {
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

/// Thread-safe stage registry
pub struct SharedStageRegistry {
    registry: Arc<Mutex<StageRegistry>>,
}

impl SharedStageRegistry {
    /// Create a new shared stage registry
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Mutex::new(StageRegistry::new())),
        }
    }
    
    /// Get a cloned reference to the registry
    pub fn registry(&self) -> Arc<Mutex<StageRegistry>> {
        self.registry.clone()
    }
    
    /// Register a stage
    pub fn register_stage(&self, stage: Box<dyn Stage>) -> Result<()> {
        match self.registry.lock() {
            Ok(mut registry) => registry.register_stage(stage),
            Err(_) => Err(Error::Stage("Failed to lock stage registry".to_string())),
        }
    }
    
    /// Check if a stage exists
    pub fn has_stage(&self, id: &str) -> Result<bool> {
        match self.registry.lock() {
            Ok(registry) => Ok(registry.has_stage(id)),
            Err(_) => Err(Error::Stage("Failed to lock stage registry".to_string())),
        }
    }
    
    /// Execute a specific stage
    pub fn execute_stage(&self, id: &str, context: &mut StageContext) -> Result<StageResult> {
        match self.registry.lock() {
            Ok(mut registry) => registry.execute_stage(id, context),
            Err(_) => Err(Error::Stage("Failed to lock stage registry".to_string())),
        }
    }
    
    /// Get all registered stage IDs
    pub fn get_all_ids(&self) -> Result<Vec<String>> {
        match self.registry.lock() {
            Ok(registry) => Ok(registry.get_all_ids()),
            Err(_) => Err(Error::Stage("Failed to lock stage registry".to_string())),
        }
    }
}

impl Default for SharedStageRegistry {
    fn default() -> Self {
        Self::new()
    }
}