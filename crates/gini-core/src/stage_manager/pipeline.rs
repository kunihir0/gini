use std::collections::{HashMap, HashSet};
use crate::kernel::error::{Error, Result};
use crate::stage_manager::{StageContext, StageResult};
// Import SharedStageRegistry for execute method
use crate::stage_manager::registry::SharedStageRegistry;

/// Represents a static definition of a pipeline.
/// Used for defining constant pipelines that can be easily referenced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineDefinition {
    /// The unique identifier name for the pipeline.
    pub name: &'static str,
    /// An ordered slice of stage IDs included in this pipeline.
    pub stages: &'static [&'static str],
    /// An optional description of the pipeline's purpose.
    pub description: Option<&'static str>,
}
/// Stage execution pipeline
pub struct StagePipeline {
    /// Name of the pipeline
    name: String,
    /// Description of what this pipeline does
    description: String,
    /// Ordered list of stage IDs to execute
    stages: Vec<String>,
    /// Optional dependencies between stages
    dependencies: HashMap<String, Vec<String>>,
    // Removed registry: StageRegistry field
}

impl StagePipeline {
    /// Create a new stage pipeline
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            stages: Vec::new(),
            dependencies: HashMap::new(),
            // No registry initialization here
        }
    }

    // Removed with_registry method

    /// Add a stage ID to the pipeline (validation happens later or during execution)
    pub fn add_stage(&mut self, stage_id: &str) -> Result<()> {
        // Just add the ID, validation against registry happens elsewhere
        if !self.stages.contains(&stage_id.to_string()) {
            self.stages.push(stage_id.to_string());
        }
        Ok(())
    }

    /// Add multiple stage IDs to the pipeline
    pub fn add_stages(&mut self, stage_ids: &[&str]) -> Result<()> {
        for stage_id in stage_ids {
            self.add_stage(stage_id)?;
        }
        Ok(())
    }

    /// Add a dependency between stages
    pub fn add_dependency(&mut self, stage_id: &str, depends_on: &str) -> Result<()> {
        // Validation against registry happens elsewhere
        // Ensure the stages are at least added to this pipeline instance
        if !self.stages.contains(&stage_id.to_string()) {
             return Err(Error::Stage(format!("Stage '{}' must be added to pipeline before adding dependency", stage_id)));
        }
         if !self.stages.contains(&depends_on.to_string()) {
             return Err(Error::Stage(format!("Dependency stage '{}' must be added to pipeline before adding dependency", depends_on)));
        }

        self.dependencies
            .entry(stage_id.to_string())
            .or_default()
            .push(depends_on.to_string());

        Ok(())
    }

    /// Validate the pipeline structure (cycles) and stage existence against a registry
    pub async fn validate(&self, registry: &SharedStageRegistry) -> Result<()> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        for stage_id in &self.stages {
            // Check existence in the provided registry
            if !registry.has_stage(stage_id).await? {
                 return Err(Error::Stage(format!("Stage '{}' defined in pipeline but not found in registry", stage_id)));
            }
            // Check for cycles
            if !visited.contains(stage_id) {
                if self.has_cycle(stage_id, &mut visited, &mut stack)? {
                    return Err(Error::Stage(
                        format!("Pipeline has cyclic dependencies starting from stage: {}", stage_id)
                    ));
                }
            }
        }
        Ok(())
    }

    /// Check for cycles in the dependency graph using DFS (internal helper)
    fn has_cycle(&self, stage_id: &str, visited: &mut HashSet<String>, stack: &mut HashSet<String>) -> Result<bool> {
        visited.insert(stage_id.to_string());
        stack.insert(stage_id.to_string());

        if let Some(deps) = self.dependencies.get(stage_id) {
            for dep in deps {
                if !visited.contains(dep) {
                    if self.has_cycle(dep, visited, stack)? {
                        return Ok(true);
                    }
                } else if stack.contains(dep) {
                    return Ok(true);
                }
            }
        }

        stack.remove(stage_id);
        Ok(false)
    }

    /// Generate a topologically sorted execution order
    /// Validation (including cycle check) should happen before calling this.
    fn get_execution_order(&self) -> Result<Vec<String>> {
        // Assume validate() was called externally and succeeded
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new(); // For cycle detection during sort

        for stage_id in &self.stages {
            if !visited.contains(stage_id) {
                self.visit_for_topsort(stage_id, &mut visited, &mut temp_mark, &mut result)?;
            }
        }
        Ok(result)
    }

    /// Visit nodes for topological sort (internal helper)
    fn visit_for_topsort(
        &self,
        stage_id: &str,
        visited: &mut HashSet<String>,
        temp_mark: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<()> {
        if temp_mark.contains(stage_id) {
            // This should ideally be caught by validate(), but check again
            return Err(Error::Stage(format!("Cyclic dependency found at stage {}", stage_id)));
        }
        if visited.contains(stage_id) {
            return Ok(()); // Already visited and added
        }

        temp_mark.insert(stage_id.to_string());

        if let Some(deps) = self.dependencies.get(stage_id) {
            for dep in deps {
                self.visit_for_topsort(dep, visited, temp_mark, result)?;
            }
        }

        temp_mark.remove(stage_id);
        visited.insert(stage_id.to_string());
        result.push(stage_id.to_string()); // Add after visiting dependencies

        Ok(())
    }

    /// Execute the pipeline asynchronously using the provided shared registry
    pub async fn execute(
        &mut self,
        context: &mut StageContext,
        registry: &SharedStageRegistry // Accept registry here
    ) -> Result<HashMap<String, StageResult>> {
        println!("Executing pipeline: {}", self.name);
        println!("Description: {}", self.description);

        if context.is_dry_run() {
            println!("MODE: DRY RUN");
            // Perform dry run validation if needed, or just simulate success
             self.validate(registry).await?; // Validate against registry even in dry run
             println!("Dry run validation successful.");
             // Return simulated success for all stages in order
             let execution_order = self.get_execution_order()?;
             let results = execution_order.into_iter().map(|id| (id, StageResult::Success)).collect();
             return Ok(results);
        }

        // Validate before execution
        self.validate(registry).await?;

        // --- Add SharedStageRegistry to context ---
        // Clone the Arc to store it in the context.
        // Stages like PluginInitializationStage can retrieve this.
        context.set_data("stage_registry_arc", registry.clone());
        // --- End Add SharedStageRegistry ---


        // Get the execution order
        let execution_order = self.get_execution_order()?;
        let mut results = HashMap::new();

        // Execute each stage in order using the provided registry
        for stage_id in execution_order {
            // Use the registry passed as argument
            let result = registry.execute_stage(&stage_id, context).await?;

            results.insert(stage_id.clone(), result.clone());

            // Check if the stage failed and we should abort
            if let StageResult::Failure(_) = result {
                println!("Pipeline aborted due to stage failure: {}", stage_id);
                break;
            }
        }

        Ok(results)
    }

    /// Get the name of the pipeline
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the description of the pipeline
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the stages in the pipeline
    pub fn stages(&self) -> &[String] {
        &self.stages
    }
}

/// Pipeline builder for simplified pipeline creation
pub struct PipelineBuilder {
    /// The pipeline being built
    pipeline: StagePipeline,
    // No registry needed here
    // No valid flag needed, build() will perform final validation if desired
}

impl PipelineBuilder {
    /// Start building a new pipeline
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            pipeline: StagePipeline::new(name, description),
        }
    }

    // Removed with_registry method

    /// Add a stage to the pipeline
    pub fn add_stage(mut self, stage_id: &str) -> Self {
        // Error handling can be deferred to build() or validate()
        let _ = self.pipeline.add_stage(stage_id); // Ignore result here
        self
    }

    /// Add multiple stages to the pipeline
    pub fn add_stages(mut self, stage_ids: &[&str]) -> Self {
        for stage_id in stage_ids {
            let _ = self.pipeline.add_stage(stage_id); // Ignore result here
        }
        self
    }

    /// Add a dependency between stages
    pub fn add_dependency(mut self, stage_id: &str, depends_on: &str) -> Self {
        // Error handling can be deferred to build() or validate()
        let _ = self.pipeline.add_dependency(stage_id, depends_on); // Ignore result here
        self
    }

    /// Build the pipeline. Validation against a registry must be done separately.
    pub fn build(self) -> StagePipeline {
        // Basic structural validation (cycles) can be done here if desired,
        // but validation against a registry requires the registry instance.
        // For now, just return the constructed pipeline.
        // Example internal cycle check (optional):
        // if let Err(e) = self.pipeline.validate_structure_only() { /* handle */ }
        self.pipeline
    }
}