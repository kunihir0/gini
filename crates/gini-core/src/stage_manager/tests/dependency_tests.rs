use crate::stage_manager::pipeline::StagePipeline;
use crate::stage_manager::registry::SharedStageRegistry;
use crate::stage_manager::{Stage, StageContext}; // Added StageResult
use crate::kernel::error::{Result, Error};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering}; // Added Ordering
use tokio::sync::Mutex;
use std::path::PathBuf; // Added PathBuf

// Re-use ExecutionTracker from pipeline_tests or define locally if needed
// Assuming it's accessible or redefined here for clarity
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
}


// Mock Stage for dependency testing - now includes tracker
struct MockStage {
    id: String,
    tracker: Arc<ExecutionTracker>, // Added tracker field
}

#[async_trait]
impl Stage for MockStage {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.id } // Simple name for testing
    fn description(&self) -> &str { "Mock stage for dependency tests" }
    async fn execute(&self, _context: &mut StageContext) -> Result<()> {
        self.tracker.record_execution(self.id()).await; // Record execution
        Ok(())
    }
}

// Helper to setup registry and pipeline, now returns tracker
async fn setup_pipeline_with_deps(
    stages: Vec<&str>,
    dependencies: Vec<(&str, &str)>
) -> (StagePipeline, SharedStageRegistry, Arc<ExecutionTracker>) { // Return tracker
    let tracker = Arc::new(ExecutionTracker::new()); // Create tracker
    let shared_registry = SharedStageRegistry::new();
    // Fix lifetime issue by binding Arc first
    let registry_arc = shared_registry.registry();
    { // Scope for the lock guard
        let mut registry_guard = registry_arc.lock().await;
        for stage_id in &stages {
            // Pass tracker to MockStage when creating
            registry_guard.register_stage(Box::new(MockStage {
                id: stage_id.to_string(),
                tracker: Arc::clone(&tracker) // Clone tracker Arc for each stage
            })).unwrap();
        }
    } // Lock guard dropped here

    let mut pipeline = StagePipeline::new("Dep Test Pipeline", "Tests dependencies");
    for stage_id in stages {
        pipeline.add_stage(stage_id).unwrap();
    }
    for (stage, dep) in dependencies {
        pipeline.add_dependency(stage, dep).unwrap();
    }
    (pipeline, shared_registry, tracker) // Return tracker
}

#[tokio::test]
async fn test_simple_dependency() -> Result<()> {
    // Capture tracker from the setup function's return tuple
    let (mut pipeline, registry, tracker) = setup_pipeline_with_deps(
        vec!["stage_a", "stage_b"],
        vec![("stage_b", "stage_a")] // b depends on a
    ).await;

    pipeline.validate(&registry).await?; // Should validate successfully

    // Execute pipeline to check order via tracker
    let mut context = StageContext::new_live(PathBuf::from("./dummy_dep_test"));
    let _results = pipeline.execute(&mut context, &registry).await?;

    let order = tracker.get_execution_order().await; // Get order from tracker
    assert_eq!(order, vec!["stage_a", "stage_b"]);
    Ok(())
}

#[tokio::test]
async fn test_multiple_dependencies() -> Result<()> {
    // Capture tracker
    let (mut pipeline, registry, tracker) = setup_pipeline_with_deps(
        vec!["a", "b", "c", "d"],
        vec![("d", "b"), ("d", "c"), ("b", "a"), ("c", "a")] // d->b, d->c, b->a, c->a
    ).await;

    pipeline.validate(&registry).await?;

    // Execute pipeline
    let mut context = StageContext::new_live(PathBuf::from("./dummy_multi_dep"));
    let _results = pipeline.execute(&mut context, &registry).await?;

    let order = tracker.get_execution_order().await; // Get order from tracker
    // Expected order: a, then b and c (order between b/c undefined), then d
    assert_eq!(order.len(), 4);
    assert_eq!(order[0], "a");
    assert!( (order[1] == "b" && order[2] == "c") || (order[1] == "c" && order[2] == "b") );
    assert_eq!(order[3], "d");
    Ok(())
}

#[tokio::test]
async fn test_no_dependencies() -> Result<()> {
    // Capture tracker
    let (mut pipeline, registry, tracker) = setup_pipeline_with_deps(
        vec!["x", "y", "z"],
        vec![] // No dependencies
    ).await;

    pipeline.validate(&registry).await?;

    // Execute pipeline
    let mut context = StageContext::new_live(PathBuf::from("./dummy_no_dep"));
    let _results = pipeline.execute(&mut context, &registry).await?;

    let order = tracker.get_execution_order().await; // Get order from tracker
    // Order is determined by add_stage order when no dependencies exist
    assert_eq!(order, vec!["x", "y", "z"]);
    Ok(())
}

#[tokio::test]
async fn test_dependency_cycle_detection() {
    // Don't need tracker here as execution shouldn't happen
    let (mut pipeline, registry, _) = setup_pipeline_with_deps(
        vec!["a", "b", "c"],
        vec![("a", "b"), ("b", "c"), ("c", "a")] // Cycle: a -> b -> c -> a
    ).await;

    // Validate should detect the cycle
    let validation_result = pipeline.validate(&registry).await;
    assert!(validation_result.is_err());
    if let Err(Error::Stage(msg)) = validation_result {
        assert!(msg.contains("cyclic dependencies"));
    } else {
        panic!("Expected cyclic dependency error from validate");
    }

    // Execute should also fail due to internal validation before running
    let mut context = StageContext::new_live(PathBuf::from("./dummy_cycle"));
    let exec_result = pipeline.execute(&mut context, &registry).await;
    assert!(exec_result.is_err());
     if let Err(Error::Stage(msg)) = exec_result {
         assert!(msg.contains("cyclic dependencies")); // execute calls validate internally
     } else {
         panic!("Expected cyclic dependency error from execute");
     }
}

#[tokio::test]
async fn test_add_dependency_on_non_added_stage() {
     let mut pipeline = StagePipeline::new("Test", "Test");
     pipeline.add_stage("stage_a").unwrap();
     // Try to add dependency on "stage_b" which isn't added to the pipeline yet
     let result = pipeline.add_dependency("stage_a", "stage_b");
     assert!(result.is_err());
     if let Err(Error::Stage(msg)) = result {
         assert!(msg.contains("Dependency stage 'stage_b' must be added"));
     } else {
         panic!("Expected specific error for missing dependency stage");
     }
}

#[tokio::test]
async fn test_add_dependency_for_non_added_stage() {
     let mut pipeline = StagePipeline::new("Test", "Test");
     pipeline.add_stage("stage_b").unwrap();
     // Try to add dependency *for* "stage_a" which isn't added to the pipeline yet
     let result = pipeline.add_dependency("stage_a", "stage_b");
     assert!(result.is_err());
     if let Err(Error::Stage(msg)) = result {
         assert!(msg.contains("Stage 'stage_a' must be added"));
     } else {
         panic!("Expected specific error for missing stage");
     }
}