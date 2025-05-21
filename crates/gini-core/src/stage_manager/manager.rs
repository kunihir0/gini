use std::fmt::Debug;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc; // Added for Arc<dyn EventManager>
// use tokio::sync::Mutex; // Added for Mutex type - now unused as type is fully qualified
 
use crate::kernel::component::KernelComponent;
use crate::kernel::error::{Result, Error as KernelError}; // Import KernelError for specific error creation
use crate::stage_manager::{Stage, StageContext, StageResult};
use crate::stage_manager::pipeline::{StagePipeline, PipelineBuilder};
use crate::stage_manager::error::StageSystemError; // Import StageSystemError
use crate::stage_manager::registry::SharedStageRegistry;
use crate::event::EventManager; // Added for EventManager
use crate::event::types::PipelineExecutionCompletedEvent; // Added for the event
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

    /// Retrieve a predefined pipeline by its name, constructing it from its definition.
    async fn get_pipeline_by_name(&self, name: &str) -> Result<Option<StagePipeline>>;

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
    event_manager: Arc<dyn EventManager>, // Added EventManager
}
 
impl DefaultStageManager {
    /// Create a new default stage manager
    pub fn new(event_manager: Arc<dyn EventManager>) -> Self {
        Self {
            name: "DefaultStageManager",
            shared_registry: SharedStageRegistry::new(), // Initialize SharedStageRegistry
            event_manager, // Store EventManager
        }
    }
 
    /// Get access to the underlying stage registry Arc<tokio::sync::Mutex<StageRegistry>>
    pub fn registry(&self) -> Arc<tokio::sync::Mutex<crate::stage_manager::registry::StageRegistry>> {
        self.shared_registry.registry()
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
        Ok(self.shared_registry.has_stage(id).await)
    }
 
    async fn get_stage_ids(&self) -> Result<Vec<String>> {
        Ok(self.shared_registry.get_all_ids().await)
    }
 
    async fn create_pipeline(&self, name: &str, description: &str, stage_ids: Vec<String>) -> Result<StagePipeline> {
        let mut builder = PipelineBuilder::new(name, description);
        for id in &stage_ids {
            // Check existence before adding to builder
            if !self.shared_registry.has_stage(id).await {
                return Err(KernelError::from(StageSystemError::StageNotFound {
                    stage_id: id.to_string()
                }));
            }
            builder = builder.add_stage(id);
        }
        // Build the pipeline struct (doesn't contain registry anymore)
        // Build the pipeline struct (doesn't contain registry anymore)
        Ok(builder.build())
    }

    async fn get_pipeline_by_name(&self, name: &str) -> Result<Option<StagePipeline>> {
        let registry_guard = self.shared_registry.registry.lock().await;
        if let Some(pipeline_def) = registry_guard.get_pipeline_definition(name) {
            // Found the definition, now construct a StagePipeline
            let mut builder = PipelineBuilder::new(pipeline_def.name, pipeline_def.description.unwrap_or(""));
            for stage_id_ref in pipeline_def.stages {
                let stage_id = stage_id_ref.to_string();
                // Ensure stage still exists in the registry using the public method
                if !registry_guard.has_stage(&stage_id) {
                    return Err(KernelError::from(StageSystemError::StageNotFoundInPipelineDefinition {
                        pipeline_name: name.to_string(), // Use the pipeline name from the function argument
                        stage_id: stage_id.clone(),
                    }));
                }
                builder = builder.add_stage(&stage_id);
            }
            Ok(Some(builder.build()))
        } else {
            Ok(None) // Pipeline definition not found
        }
    }

    async fn execute_pipeline(&self, pipeline: &mut StagePipeline, context: &mut StageContext) -> Result<HashMap<String, StageResult>> {
        let pipeline_name = pipeline.name().to_string();
        let execution_result = pipeline.execute(context, &self.shared_registry).await;
        
        let success = execution_result.is_ok();
        
        let event = PipelineExecutionCompletedEvent {
            pipeline_name,
            success,
            timestamp: std::time::SystemTime::now(),
        };
        
        // Use queue_event as per typical event manager patterns for async emission
        // The original instruction mentioned `emit_event(...).await` which implies an async operation.
        // `queue_event` is async on the EventManager trait.
        self.event_manager.queue_event(Box::new(event)).await;
        // Note: The EventManager's queue_event doesn't return a Result, so direct error handling
        // for emission itself isn't done here unless queue_event's signature changes.
        // If logging for emission failure is needed, it would depend on EventManager's implementation details.
        // For now, we assume queue_event handles its errors internally or is infallible.
        // If `emit_event` was a specific method that returned a Result, we'd handle it:
        // if let Err(e) = self.event_manager.emit_event(Box::new(event)).await {
        //     log::error!("Failed to emit PipelineExecutionCompletedEvent: {:?}", e);
        // }
        
        execution_result
    }
 
    async fn execute_stage(&self, stage_id: &str, context: &mut StageContext) -> Result<StageResult> {
        self.shared_registry.execute_stage(stage_id, context).await
    }

    async fn validate_pipeline(&self, pipeline: &StagePipeline) -> Result<()> { // Make async
        // Pass the shared registry to the pipeline's validate method
        pipeline.validate(&self.shared_registry).await.map_err(KernelError::from)
    }
 
    async fn create_dry_run_pipeline(&self, name: &str, description: &str, stage_ids: Vec<String>) -> Result<StagePipeline> {
        // Create a regular pipeline
        self.create_pipeline(name, description, stage_ids).await
    }
}

// Default implementation is no longer straightforward as it requires an EventManager.
// If a Default is strictly needed, it might require a way to get a default EventManager,
// or this impl should be removed if DefaultStageManager is always constructed with dependencies.
// For now, commenting out as it will cause a compile error.
// impl Default for DefaultStageManager {
//     fn default() -> Self {
//         // This would require a default EventManager instance, which is complex.
//         // Consider if Default is truly necessary or if instantiation should always be explicit.
//         panic!("DefaultStageManager cannot be created with default(); it requires an EventManager.");
//     }
// }