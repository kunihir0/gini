use crate::kernel::error::Result;
use crate::stage_manager::{Stage, StageContext};
use crate::stage_manager::registry::StageRegistry;
use async_trait::async_trait;

// Mock Stage for testing
struct MockStage {
    id: String,
    name: String,
    description: String,
    supports_dry_run: bool,
}

impl MockStage {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: format!("Mock Stage {}", id),
            description: format!("Test stage with ID {}", id),
            supports_dry_run: true,
        }
    }

    #[allow(dead_code)] // Allow dead code as this helper might not be used in all tests
    fn with_dry_run(mut self, supports: bool) -> Self {
        self.supports_dry_run = supports;
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

    fn supports_dry_run(&self) -> bool {
        self.supports_dry_run
    }

    async fn execute(&self, _context: &mut StageContext) -> Result<()> {
        // Simple mock implementation that does nothing
        Ok(())
    }
}

#[test]
fn test_registry_initialization() {
    let registry = StageRegistry::new();

    assert_eq!(registry.count(), 0, "New registry should be empty");
    assert!(registry.get_all_ids().is_empty(), "New registry should have empty stage ID list");
}

#[test]
fn test_stage_registration() {
    let mut registry = StageRegistry::new();
    let stage = MockStage::new("test.stage");

    // Register stage using the correct method name
    let result = registry.register_stage(Box::new(stage));
    assert!(result.is_ok(), "Registration should succeed");

    // Check registry state using correct methods
    assert_eq!(registry.count(), 1, "Registry should have one stage");
    assert!(registry.has_stage("test.stage"), "Registry should contain stage by ID");

    let stage_ids = registry.get_all_ids();
    assert_eq!(stage_ids.len(), 1, "get_all_ids should return one ID");
    assert_eq!(stage_ids[0], "test.stage");
}

#[test]
fn test_duplicate_stage_registration() {
    let mut registry = StageRegistry::new();

    // Register first stage
    let stage1 = MockStage::new("test.stage");
    let result1 = registry.register_stage(Box::new(stage1));
    assert!(result1.is_ok(), "First registration should succeed");

    // Try to register duplicate stage
    let stage2 = MockStage::new("test.stage");
    let result2 = registry.register_stage(Box::new(stage2));
    assert!(result2.is_err(), "Duplicate registration should fail");

    // Check registry state
    assert_eq!(registry.count(), 1, "Registry should still have only one stage");
}

#[test]
fn test_has_stage() { // Renamed from test_get_stage
    let mut registry = StageRegistry::new();

    // Register a stage
    let stage = MockStage::new("test.stage");
    registry.register_stage(Box::new(stage)).unwrap();

    // Check if stage exists
    assert!(registry.has_stage("test.stage"), "Should confirm registered stage exists");

    // Check for non-existent stage
    assert!(!registry.has_stage("nonexistent"), "Nonexistent stage should return false");
}

#[test]
fn test_register_multiple_stages() {
    let mut registry = StageRegistry::new();

    // Register multiple stages
    for i in 1..=5 {
        let stage = MockStage::new(&format!("stage.{}", i));
        let result = registry.register_stage(Box::new(stage));
        assert!(result.is_ok(), "Registration of stage {} should succeed", i);
    }

    // Check registry state
    assert_eq!(registry.count(), 5, "Registry should have 5 stages");
    for i in 1..=5 {
        let id = format!("stage.{}", i);
        assert!(registry.has_stage(&id), "Registry should contain stage {}", id);
    }
}

#[test]
fn test_get_all_stage_ids() { // Renamed from test_get_all_stages
    let mut registry = StageRegistry::new();

    // Register multiple stages with specific IDs
    let stages_to_register = vec![
        "c.stage",
        "a.stage",
        "b.stage",
        "d.stage",
    ];

    for &id in &stages_to_register {
        registry.register_stage(Box::new(MockStage::new(id))).unwrap();
    }

    // Get all stage IDs
    let all_stage_ids = registry.get_all_ids();
    assert_eq!(all_stage_ids.len(), stages_to_register.len(), "get_all_ids should return all IDs");

    // Check all registered IDs are present (order not guaranteed by HashMap keys)
    let mut found_ids = Vec::new();
    for id in all_stage_ids {
        found_ids.push(id);
    }
    found_ids.sort(); // Sort for consistent comparison

    let mut expected_ids = stages_to_register.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    expected_ids.sort();

    assert_eq!(found_ids, expected_ids, "All registered IDs should be returned");
}

#[test]
fn test_remove_stage() {
    let mut registry = StageRegistry::new();

    // Register stages
    registry.register_stage(Box::new(MockStage::new("stage.1"))).unwrap();
    registry.register_stage(Box::new(MockStage::new("stage.2"))).unwrap();
    registry.register_stage(Box::new(MockStage::new("stage.3"))).unwrap();

    assert_eq!(registry.count(), 3, "Registry should have 3 stages initially");

    // Remove a stage using the correct method
    let removed_stage = registry.remove_stage("stage.2");
    assert!(removed_stage.is_some(), "Removing existing stage should return Some");
    assert_eq!(removed_stage.unwrap().id(), "stage.2"); // Verify correct stage was removed

    // Check registry state after removal
    assert_eq!(registry.count(), 2, "Registry should have 2 stages after removal");
    assert!(registry.has_stage("stage.1"), "Stage 1 should still exist");
    assert!(!registry.has_stage("stage.2"), "Stage 2 should be removed");
    assert!(registry.has_stage("stage.3"), "Stage 3 should still exist");

    // Try to remove non-existent stage
    let non_existent_removed = registry.remove_stage("nonexistent");
    assert!(non_existent_removed.is_none(), "Removing non-existent stage should return None");
}

#[test]
fn test_clear_registry() {
    let mut registry = StageRegistry::new();

    // Register multiple stages
    for i in 1..=3 {
        registry.register_stage(Box::new(MockStage::new(&format!("stage.{}", i)))).unwrap();
    }

    assert_eq!(registry.count(), 3, "Registry should have 3 stages initially");

    // Clear registry
    registry.clear();

    // Check registry state
    assert_eq!(registry.count(), 0, "Registry should be empty after clear");
    assert!(registry.get_all_ids().is_empty(), "get_all_ids should return empty list after clear");
    assert!(!registry.has_stage("stage.1"), "Registry should not contain any stages after clear");
}