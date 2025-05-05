use crate::stage_manager::{Stage, StageContext, StageResult};
use crate::stage_manager::pipeline::StagePipeline;
use crate::stage_manager::registry::SharedStageRegistry; // Import SharedStageRegistry
use crate::kernel::error::{Result, Error};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::Mutex;
 // Import HashMap

// Test helper to track stage execution
struct ExecutionTracker {
    executed_stages: Mutex<Vec<String>>,
    execution_count: Arc<AtomicU32>,
}

impl ExecutionTracker {
    fn new() -> Self {
        Self {
            executed_stages: Mutex::new(Vec::new()),
            execution_count: Arc::new(AtomicU32::new(0)),
        }
    }

    async fn record_execution(&self, stage_id: &str) {
        let mut stages = self.executed_stages.lock().await;
        stages.push(stage_id.to_string());
        self.execution_count.fetch_add(1, Ordering::SeqCst);
    }

    async fn get_execution_order(&self) -> Vec<String> {
        self.executed_stages.lock().await.clone()
    }

    #[allow(dead_code)] // Allow dead code as this helper might not be used in all tests
    fn get_execution_count(&self) -> u32 {
        self.execution_count.load(Ordering::SeqCst)
    }
}

// Mock Stage implementation that uses the tracker
// Store error message string instead of Result<()>, as Error is not Clone
struct MockStage {
    id: String,
    name: String,
    description: String,
    tracker: Arc<ExecutionTracker>,
    error_message: Option<String>, // Store potential error message
}

impl MockStage {
    fn new(id: &str, tracker: Arc<ExecutionTracker>) -> Self {
        Self {
            id: id.to_string(),
            name: format!("Mock Stage {}", id),
            description: format!("Test stage with ID {}", id),
            tracker,
            error_message: None, // Default to success
        }
    }

    // Configure the mock stage to return an error
    fn with_error(mut self, error_message: &str) -> Self {
        self.error_message = Some(error_message.to_string());
        self
    }
}

#[async_trait]
impl Stage for MockStage {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn execute(&self, _context: &mut StageContext) -> Result<()> {
        // Record this stage was executed
        self.tracker.record_execution(self.id()).await;

        // Return Ok or Err based on stored error message
        if let Some(msg) = &self.error_message {
            Err(Error::Stage(msg.clone()))
        } else {
            Ok(())
        }
    }
}

// Helper to register stages into a SharedStageRegistry
async fn register_stages(shared_registry: &SharedStageRegistry, stages: Vec<Box<dyn Stage>>) {
     let registry = shared_registry.registry();
     let mut registry_guard = registry.lock().await;
     for stage in stages {
         registry_guard.register_stage(stage).unwrap();
     }
}

#[tokio::test]
async fn test_pipeline_basics() {
    let tracker = Arc::new(ExecutionTracker::new());
    let shared_registry = SharedStageRegistry::new(); // Create empty shared registry

    // Register stages within the test
    register_stages(&shared_registry, vec![
        Box::new(MockStage::new("stage.1", Arc::clone(&tracker))),
        Box::new(MockStage::new("stage.2", Arc::clone(&tracker))),
        Box::new(MockStage::new("stage.3", Arc::clone(&tracker))),
    ]).await;

    // Create a pipeline with name and description
    let mut pipeline = StagePipeline::new("Basic Pipeline", "Tests basic execution");
    let mut context = StageContext::new_live(std::env::temp_dir());

    // Add stages to pipeline
    pipeline.add_stage("stage.1").unwrap();
    pipeline.add_stage("stage.2").unwrap();
    pipeline.add_stage("stage.3").unwrap();

    // Execute pipeline, passing the shared registry
    let result_map = pipeline.execute(&mut context, &shared_registry).await;

    // Check overall result (should be Ok containing the map)
    assert!(result_map.is_ok(), "Pipeline execution should succeed");
    let results = result_map.unwrap();

    // Check individual stage results
    assert_eq!(results.len(), 3);
    assert!(matches!(results.get("stage.1"), Some(StageResult::Success)));
    assert!(matches!(results.get("stage.2"), Some(StageResult::Success)));
    assert!(matches!(results.get("stage.3"), Some(StageResult::Success)));


    // Check execution order
    let executed = tracker.get_execution_order().await;
    assert_eq!(executed.len(), 3, "All three stages should have executed");
    assert_eq!(executed[0], "stage.1");
    assert_eq!(executed[1], "stage.2");
    assert_eq!(executed[2], "stage.3");
}

#[tokio::test]
async fn test_pipeline_error_handling() {
    let tracker = Arc::new(ExecutionTracker::new());
    let shared_registry = SharedStageRegistry::new(); // Create empty shared registry

    // Register stages within the test
    register_stages(&shared_registry, vec![
        Box::new(MockStage::new("stage.1", Arc::clone(&tracker))),
        Box::new(MockStage::new("stage.2", Arc::clone(&tracker))
            .with_error("Test error in stage 2")),
        Box::new(MockStage::new("stage.3", Arc::clone(&tracker))),
    ]).await;

    // Create a pipeline
    let mut pipeline = StagePipeline::new("Error Pipeline", "Tests error handling");
    let mut context = StageContext::new_live(std::env::temp_dir());

    // Add stages to pipeline
    pipeline.add_stage("stage.1").unwrap();
    pipeline.add_stage("stage.2").unwrap();
    pipeline.add_stage("stage.3").unwrap();

    // Execute pipeline
    let result_map = pipeline.execute(&mut context, &shared_registry).await;

    // Check overall result (should be Ok containing the map, but map contains failure)
    assert!(result_map.is_ok(), "Pipeline execution should return Ok result map even on stage failure");
    let results = result_map.unwrap();

    // Check individual stage results
    assert_eq!(results.len(), 2, "Should contain results for executed stages only");
    assert!(matches!(results.get("stage.1"), Some(StageResult::Success)));
    // Adjust assertion to match the full error string from e.to_string()
    assert!(matches!(results.get("stage.2"), Some(StageResult::Failure(msg)) if msg == "Stage error: Test error in stage 2"));
    assert!(results.get("stage.3").is_none(), "Stage 3 should not have a result");


    // Check execution (only stages up to and including error should execute)
    let executed = tracker.get_execution_order().await;
    assert_eq!(executed.len(), 2, "Only two stages should have executed");
    assert_eq!(executed[0], "stage.1", "First stage should execute");
    assert_eq!(executed[1], "stage.2", "Second stage should execute and fail");
}

#[tokio::test]
async fn test_pipeline_add_stage_validation() {
    // Registry is empty initially
    let shared_registry = SharedStageRegistry::new();
    let mut pipeline = StagePipeline::new("Validation Pipeline", "Tests validation");

    // Add a stage ID that doesn't exist in the registry yet
    pipeline.add_stage("nonexistent").unwrap(); // Adding ID itself is fine

    // Validation should fail because the stage isn't in the registry
    let validation_result = pipeline.validate(&shared_registry).await;
    assert!(validation_result.is_err(), "Validation should fail for nonexistent stage");
    if let Err(Error::Stage(msg)) = validation_result {
         assert!(msg.contains("nonexistent") && msg.contains("not found in registry"));
    } else {
        panic!("Expected Stage error, got: {:?}", validation_result);
    }
}


#[tokio::test]
async fn test_empty_pipeline() {
    let shared_registry = SharedStageRegistry::new(); // Empty registry
    let mut pipeline = StagePipeline::new("Empty Pipeline", "Tests empty execution");
    let mut context = StageContext::new_live(std::env::temp_dir());

    // Execute empty pipeline
    let result_map = pipeline.execute(&mut context, &shared_registry).await;

    // Should succeed and return an empty map
    assert!(result_map.is_ok(), "Empty pipeline should succeed");
    assert!(result_map.unwrap().is_empty(), "Result map should be empty");
}

#[tokio::test]
async fn test_pipeline_get_stages() {
    let _tracker = Arc::new(ExecutionTracker::new()); // Keep tracker even if unused, for consistency
    // Registry is not needed for just getting stage IDs from the pipeline definition
    let mut pipeline = StagePipeline::new("Get Stages Pipeline", "Tests getting stage list");

    // Add stages to pipeline
    pipeline.add_stage("stage.1").unwrap();
    pipeline.add_stage("stage.2").unwrap();

    // Get stage IDs
    let stage_ids = pipeline.stages(); // This returns &[String]

    // Check stage IDs
    assert_eq!(stage_ids.len(), 2, "Pipeline should have two stage IDs");
    assert_eq!(stage_ids[0], "stage.1", "First stage ID should be stage.1");
    assert_eq!(stage_ids[1], "stage.2", "Second stage ID should be stage.2");
}

// Note: The `clear` method test was removed as StagePipeline doesn't have `clear`.
// To achieve clearing, simply create a new StagePipeline instance.