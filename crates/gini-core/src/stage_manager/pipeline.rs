use std::collections::{HashMap, HashSet};
use crate::kernel::error::{Error as KernelError, Result as KernelResult}; // Renamed Error & Result
use crate::stage_manager::{StageContext, StageResult};
use crate::stage_manager::error::StageSystemError; // Import StageSystemError
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
    pub fn add_stage(&mut self, stage_id: &str) -> KernelResult<()> { // Changed to KernelResult
        // Just add the ID, validation against registry happens elsewhere
        if !self.stages.contains(&stage_id.to_string()) {
            self.stages.push(stage_id.to_string());
        }
        Ok(())
    }

    /// Add multiple stage IDs to the pipeline
    pub fn add_stages(&mut self, stage_ids: &[&str]) -> KernelResult<()> { // Changed to KernelResult
        for stage_id in stage_ids {
            self.add_stage(stage_id)?;
        }
        Ok(())
    }

    /// Add a dependency between stages
    // Changed to return Result<(), StageSystemError> as per plan for internal errors
    pub fn add_dependency(&mut self, stage_id: &str, depends_on: &str) -> std::result::Result<(), StageSystemError> {
        // Validation against registry happens elsewhere
        // Ensure the stages are at least added to this pipeline instance
        if !self.stages.contains(&stage_id.to_string()) {
             return Err(StageSystemError::DependencyStageNotInPipeline {
                pipeline_name: self.name.clone(), // Or a generic name if not available
                stage_id: stage_id.to_string(),
                dependency_id: depends_on.to_string(), // This error is about stage_id not being in pipeline
            });
        }
         if !self.stages.contains(&depends_on.to_string()) {
             return Err(StageSystemError::DependencyStageNotInPipeline {
                pipeline_name: self.name.clone(),
                stage_id: stage_id.to_string(), // This error is about depends_on not being in pipeline
                dependency_id: depends_on.to_string(),
            });
        }
 
        self.dependencies
            .entry(stage_id.to_string())
            .or_default()
            .push(depends_on.to_string());

        Ok(())
    }

    /// Validate the pipeline structure (cycles) and stage existence against a registry
    // Changed to return Result<(), StageSystemError>
    pub async fn validate(&self, registry: &SharedStageRegistry) -> std::result::Result<(), StageSystemError> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
 
        for stage_id in &self.stages {
            // Check existence in the provided registry
            if !registry.has_stage(stage_id).await { // Removed ? as has_stage is now infallible
                 return Err(StageSystemError::StageNotFoundInPipelineValidation {
                    pipeline_name: self.name.clone(),
                    stage_id: stage_id.to_string(),
                });
            }
            // Check for cycles
            if !visited.contains(stage_id) {
                // has_cycle now returns Result<bool, StageSystemError>
                // It returns Err(DependencyCycleDetected) if a cycle is found.
                // It returns Ok(false) if no cycle is found from this node.
                // It should not return Ok(true).
                match self.has_cycle(stage_id, &mut visited, &mut stack) {
                    Ok(false) => { /* No cycle found from this node, continue */ }
                    Ok(true) => {
                        // This case should ideally not be reached if has_cycle correctly returns Err on cycle.
                        // If it does, it's an internal logic inconsistency in has_cycle.
                        // For robustness, treat Ok(true) as a cycle detection as well.
                        return Err(StageSystemError::DependencyCycleDetected {
                            pipeline_name: self.name.clone(),
                            cycle_path: vec![stage_id.to_string()], // Simplified path
                        });
                    }
                    Err(e @ StageSystemError::DependencyCycleDetected { .. }) => return Err(e), // Propagate the detailed error
                    Err(e) => return Err(e), // Should not happen if has_cycle only returns DependencyCycleDetected or Ok(false)
                }
            }
        }
        Ok(())
    }
 
    /// Check for cycles in the dependency graph using DFS (internal helper)
    // Changed to return Result<bool, StageSystemError>, Err if cycle detected.
    fn has_cycle(&self, stage_id: &str, visited: &mut HashSet<String>, stack: &mut HashSet<String>) -> std::result::Result<bool, StageSystemError> {
        visited.insert(stage_id.to_string());
        stack.insert(stage_id.to_string());

        if let Some(deps) = self.dependencies.get(stage_id) {
            for dep in deps {
                if !visited.contains(dep) {
                    if self.has_cycle(dep, visited, stack)? { // Recursive call returns Result<bool, StageSystemError>
                        // If inner call found a cycle (returned Ok(true)), propagate it as Ok(true)
                        return Ok(true);
                    }
                } else if stack.contains(dep) {
                    // Cycle detected
                    return Err(StageSystemError::DependencyCycleDetected {
                        pipeline_name: self.name.clone(), // Or pass pipeline name
                        cycle_path: stack.iter().cloned().collect(), // Provide the current stack as path
                    });
                }
            }
        }
 
        stack.remove(stage_id);
        Ok(false) // No cycle found starting from this node in this DFS path
    }
 
    /// Generate a topologically sorted execution order
    /// Validation (including cycle check) should happen before calling this.
    // Changed to return Result<Vec<String>, StageSystemError>
    fn get_execution_order(&self) -> std::result::Result<Vec<String>, StageSystemError> {
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
    ) -> std::result::Result<(), StageSystemError> { // Changed to StageSystemError
        if temp_mark.contains(stage_id) {
            // This should ideally be caught by validate(), but check again
            return Err(StageSystemError::DependencyCycleDetected {
                pipeline_name: self.name.clone(),
                cycle_path: temp_mark.iter().cloned().collect(), // Provide current path
            });
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
    ) -> KernelResult<HashMap<String, StageResult>> { // Returns KernelResult
        println!("Executing pipeline: {}", self.name);
        println!("Description: {}", self.description);

        if context.is_dry_run() {
            println!("MODE: DRY RUN");
            // Perform dry run validation if needed, or just simulate success
             self.validate(registry).await.map_err(KernelError::from)?; // Validate against registry
             println!("Dry run validation successful.");
             // Return simulated success for all stages in order
             let execution_order = self.get_execution_order().map_err(KernelError::from)?;
             let results = execution_order.into_iter().map(|id| (id, StageResult::Success)).collect();
             return Ok(results);
        }
 
        // Validate before execution
        self.validate(registry).await.map_err(KernelError::from)?;

        // --- Add SharedStageRegistry to context ---
        // Clone the Arc to store it in the context.
        // Stages like PluginInitializationStage can retrieve this.
        context.set_data("stage_registry_arc", registry.clone());
        // --- End Add SharedStageRegistry ---


        // Get the execution order
        let execution_order = self.get_execution_order().map_err(KernelError::from)?;
        let mut results = HashMap::new();
 
        // Execute each stage in order using the provided registry
        for stage_id in execution_order {
            // Use the registry passed as argument
            // registry.execute_stage now returns KernelResult<StageResult>
            // which wraps Result<StageResult, StageSystemError>
            match registry.execute_stage(&stage_id, context).await {
                Ok(stage_outcome) => {
                    results.insert(stage_id.clone(), stage_outcome.clone());
                    // The StageResult::Failure case for aborting is removed because
                    // execute_stage_internal now returns Err(StageSystemError::StageExecutionFailed)
                    // for actual errors from Stage::execute.
                    // If StageResult::Failure were to be used for other "logical" non-error failures
                    // that should still halt the pipeline, that logic would go here.
                    // For now, only an Err from execute_stage halts the pipeline.
                }
                Err(kernel_err) => {
                    // This means execute_stage_internal returned Err(StageSystemError),
                    // which was mapped to KernelError by SharedStageRegistry::execute_stage.
                    // This is a hard error from the stage execution itself (e.g. StageExecutionFailed).
                    println!("Pipeline aborted due to stage error: {} - {}", stage_id, kernel_err);
                    return Err(kernel_err); // Propagate the KernelError
                }
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