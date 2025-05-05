use std::fmt::Debug;
use async_trait::async_trait;
use std::collections::HashMap;

use crate::kernel::component::KernelComponent;
use crate::kernel::error::Result;
use crate::stage_manager::{Stage, StageContext, StageResult};
use crate::stage_manager::pipeline::{StagePipeline, PipelineBuilder};
use crate::stage_manager::registry::SharedStageRegistry;
use crate::stage_manager::core_stages::{ // Import core stages
    PluginPreflightCheckStage,
    PluginInitializationStage,
    PluginPostInitializationStage,
};

/// Interface for the stage management component
#[async_trait]
pub trait StageManager: KernelComponent {
    /// Register a new stage
    async fn register_stage(&self, stage: Box<dyn Stage>) -> Result<()>;

    /// Check if a stage exists
    async fn has_stage(&self, id: &str) -> Result<bool>;

    /// Get all registered stage IDs
    async fn get_stage_ids(&self) -> Result<Vec<String>>;

    /// Create a new pipeline with a set of stage IDs
    async fn create_pipeline(&self, name: &str, description: &str, stage_ids: Vec<String>) -> Result<StagePipeline>;

    /// Execute a pipeline with the given context
    async fn execute_pipeline(&self, pipeline: &mut StagePipeline, context: &mut StageContext) -> Result<HashMap<String, StageResult>>;

    /// Execute a single stage
    async fn execute_stage(&self, stage_id: &str, context: &mut StageContext) -> Result<StageResult>;

    /// Check if a pipeline is valid against the manager's registry
    async fn validate_pipeline(&self, pipeline: &StagePipeline) -> Result<()>; // Make async

    /// Create a dry run pipeline
    async fn create_dry_run_pipeline(&self, name: &str, description: &str, stage_ids: Vec<String>) -> Result<StagePipeline>;
}

/// Default implementation of StageManager using SharedStageRegistry
#[derive(Clone, Debug)] // Add Clone and Debug derives
pub struct DefaultStageManager {
    name: &'static str,
    shared_registry: SharedStageRegistry, // Use SharedStageRegistry
}

impl DefaultStageManager {
    /// Create a new default stage manager
    pub fn new() -> Self {
        Self {
            name: "DefaultStageManager",
            shared_registry: SharedStageRegistry::new(), // Initialize SharedStageRegistry
        }
    }

    /// Get access to the underlying shared registry
    pub fn registry(&self) -> &SharedStageRegistry {
        &self.shared_registry
    }
}

#[async_trait]
impl KernelComponent for DefaultStageManager {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn initialize(&self) -> Result<()> {
        println!("Initializing DefaultStageManager and registering core stages...");
        // Register core lifecycle stages
        self.register_stage(Box::new(PluginPreflightCheckStage)).await?;
        println!("Registered stage: {}", PluginPreflightCheckStage.id());
        self.register_stage(Box::new(PluginInitializationStage)).await?;
        println!("Registered stage: {}", PluginInitializationStage.id());
        self.register_stage(Box::new(PluginPostInitializationStage)).await?;
        println!("Registered stage: {}", PluginPostInitializationStage.id());
        println!("Core stages registered.");
        Ok(())
    }
    async fn start(&self) -> Result<()> { Ok(()) }
    async fn stop(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
impl StageManager for DefaultStageManager {
    async fn register_stage(&self, stage: Box<dyn Stage>) -> Result<()> {
        self.shared_registry.register_stage(stage).await
    }

    async fn has_stage(&self, id: &str) -> Result<bool> {
        self.shared_registry.has_stage(id).await
    }

    async fn get_stage_ids(&self) -> Result<Vec<String>> {
        self.shared_registry.get_all_ids().await
    }

    async fn create_pipeline(&self, name: &str, description: &str, stage_ids: Vec<String>) -> Result<StagePipeline> {
        let mut builder = PipelineBuilder::new(name, description);
        for id in &stage_ids {
            // Check existence before adding to builder
            if !self.shared_registry.has_stage(id).await? {
                return Err(crate::kernel::error::Error::Stage(
                    format!("Stage '{}' not found in registry during pipeline creation", id)
                ));
            }
            builder = builder.add_stage(id);
        }
        // Build the pipeline struct (doesn't contain registry anymore)
        // Build the pipeline struct (doesn't contain registry anymore)
        Ok(builder.build())
    }

    async fn execute_pipeline(&self, pipeline: &mut StagePipeline, context: &mut StageContext) -> Result<HashMap<String, StageResult>> {
        // Pass the shared registry to the pipeline's execute method
        pipeline.execute(context, &self.shared_registry).await
    }

    async fn execute_stage(&self, stage_id: &str, context: &mut StageContext) -> Result<StageResult> {
        self.shared_registry.execute_stage(stage_id, context).await
    }

    async fn validate_pipeline(&self, pipeline: &StagePipeline) -> Result<()> { // Make async
        // Pass the shared registry to the pipeline's validate method
        pipeline.validate(&self.shared_registry).await
    }

    async fn create_dry_run_pipeline(&self, name: &str, description: &str, stage_ids: Vec<String>) -> Result<StagePipeline> {
        // Create a regular pipeline
        self.create_pipeline(name, description, stage_ids).await
    }
}

impl Default for DefaultStageManager {
    fn default() -> Self {
        Self::new()
    }
}