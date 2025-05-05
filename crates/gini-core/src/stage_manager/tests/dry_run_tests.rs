use crate::stage_manager::{Stage, StageContext, StageResult};
use crate::stage_manager::pipeline::StagePipeline;
use crate::stage_manager::registry::SharedStageRegistry;
use crate::kernel::error::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;

// Mock Stage that tracks execution and supports dry run description
struct MockDryRunStage {
    id: String,
    tracker: Arc<Mutex<Vec<String>>>, // Track actual executions
    supports_dry_run: bool,
    dry_run_desc: String,
}

impl MockDryRunStage {
    fn new(id: &str, tracker: Arc<Mutex<Vec<String>>>, supports_dry_run: bool) -> Self {
        Self {
            id: id.to_string(),
            tracker,
            supports_dry_run,
            dry_run_desc: format!("Dry run: Would execute stage {}", id),
        }
    }
}

#[async_trait]
impl Stage for MockDryRunStage {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.id }
    fn description(&self) -> &str { "Mock stage for dry run tests" }
    fn supports_dry_run(&self) -> bool { self.supports_dry_run }
    fn dry_run_description(&self, _context: &StageContext) -> String { self.dry_run_desc.clone() }

    async fn execute(&self, _context: &mut StageContext) -> Result<()> {
        // Only record execution if actually run (not dry run)
        let mut tracker_lock = self.tracker.lock().await;
        tracker_lock.push(self.id.clone());
        println!("ACTUAL EXECUTION: {}", self.id); // Log actual execution
        Ok(())
    }
}

// Helper to setup registry and pipeline
async fn setup_dry_run_test(
    stages_config: Vec<(&str, bool)> // (id, supports_dry_run)
) -> (StagePipeline, SharedStageRegistry, Arc<Mutex<Vec<String>>>) {
    let tracker = Arc::new(Mutex::new(Vec::new()));
    let shared_registry = SharedStageRegistry::new();
    // Fix lifetime issue
    let registry_arc = shared_registry.registry();
    { // Scope for lock guard
        let mut registry_guard = registry_arc.lock().await;
        // Use iter() to borrow stages_config in the first loop
        for (id, supports) in stages_config.iter() {
            registry_guard.register_stage(Box::new(MockDryRunStage::new(
                id,
                Arc::clone(&tracker),
                *supports, // Dereference bool
            ))).unwrap();
        }
    } // Lock guard dropped

    let mut pipeline = StagePipeline::new("Dry Run Test Pipeline", "Tests dry run");
    // Use iter() again to borrow stages_config
    for (id, _) in stages_config.iter() {
        pipeline.add_stage(id).unwrap();
    }

    (pipeline, shared_registry, tracker)
}

#[tokio::test]
async fn test_dry_run_execution() -> Result<()> {
    let (mut pipeline, registry, tracker) = setup_dry_run_test(vec![
        ("stage_a", true),
        ("stage_b", true),
        ("stage_c", true),
    ]).await;

    // Create context in dry run mode
    let mut context = StageContext::new_dry_run(PathBuf::from("./dummy_dry_run"));

    // Execute pipeline in dry run mode
    let results = pipeline.execute(&mut context, &registry).await?;

    // Check results - all should report Success (as dry run doesn't execute)
    assert_eq!(results.len(), 3);
    assert!(matches!(results.get("stage_a"), Some(StageResult::Success)));
    assert!(matches!(results.get("stage_b"), Some(StageResult::Success)));
    assert!(matches!(results.get("stage_c"), Some(StageResult::Success)));

    // Check tracker - no stages should have actually executed
    let executed = tracker.lock().await;
    assert!(executed.is_empty(), "No stages should execute in dry run mode");

    Ok(())
}

#[tokio::test]
async fn test_dry_run_with_unsupported_stage() -> Result<()> {
    // Stage B does not support dry run (though Stage trait default is true, we override)
    // The pipeline execution itself should still succeed in dry run mode,
    // as the stage's execute method is not called. Validation might catch this
    // if we add dry run support checks there, but execute should proceed.
    let (mut pipeline, registry, tracker) = setup_dry_run_test(vec![
        ("stage_a", true),
        ("stage_b_no_dry", false), // This stage "doesn't support" dry run
        ("stage_c", true),
    ]).await;

    let mut context = StageContext::new_dry_run(PathBuf::from("./dummy_dry_run_unsupported"));

    // Execute pipeline in dry run mode
    let results = pipeline.execute(&mut context, &registry).await?;

    // Check results - all should still report Success because execute isn't called
    assert_eq!(results.len(), 3);
    assert!(matches!(results.get("stage_a"), Some(StageResult::Success)));
    assert!(matches!(results.get("stage_b_no_dry"), Some(StageResult::Success)));
    assert!(matches!(results.get("stage_c"), Some(StageResult::Success)));

    // Check tracker - no stages should have actually executed
    let executed = tracker.lock().await;
    assert!(executed.is_empty(), "No stages should execute in dry run mode, even unsupported ones");

    Ok(())
}

#[tokio::test]
async fn test_live_run_execution() -> Result<()> {
    // Verify that a live run actually executes the stages
    let (mut pipeline, registry, tracker) = setup_dry_run_test(vec![
        ("stage_a", true),
        ("stage_b", true),
    ]).await;

    // Create context in live mode
    let mut context = StageContext::new_live(PathBuf::from("./dummy_live_run"));

    // Execute pipeline in live mode
    let results = pipeline.execute(&mut context, &registry).await?;

    // Check results - all should report Success
    assert_eq!(results.len(), 2);
    assert!(matches!(results.get("stage_a"), Some(StageResult::Success)));
    assert!(matches!(results.get("stage_b"), Some(StageResult::Success)));

    // Check tracker - stages should have executed
    let executed = tracker.lock().await;
    assert_eq!(executed.len(), 2, "Both stages should have executed in live mode");
    assert_eq!(executed[0], "stage_a");
    assert_eq!(executed[1], "stage_b");

    Ok(())
}