use std::collections::{HashMap, HashSet, VecDeque};

use crate::kernel::error::{Error, Result};
use crate::stage_manager::{StageRegistry, StageContext, StageResult};
use crate::stage_manager::requirement::StageRequirement;

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
    /// Registry to look up stages
    registry: StageRegistry,
}

impl StagePipeline {
    /// Create a new stage pipeline
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            stages: Vec::new(),
            dependencies: HashMap::new(),
            registry: StageRegistry::new(),
        }
    }
    
    /// Set the stage registry
    pub fn with_registry(mut self, registry: StageRegistry) -> Self {
        self.registry = registry;
        self
    }
    
    /// Add a stage to the pipeline
    pub fn add_stage(&mut self, stage_id: &str) -> Result<()> {
        // Ensure the stage exists in the registry
        if !self.registry.has_stage(stage_id) {
            return Err(Error::Stage(format!(
                "Cannot add stage '{}' to pipeline: Stage not found in registry",
                stage_id
            )));
        }
        
        // Add to the pipeline if not already present
        if !self.stages.contains(&stage_id.to_string()) {
            self.stages.push(stage_id.to_string());
        }
        
        Ok(())
    }
    
    /// Add multiple stages to the pipeline
    pub fn add_stages(&mut self, stage_ids: &[&str]) -> Result<()> {
        for stage_id in stage_ids {
            self.add_stage(stage_id)?;
        }
        Ok(())
    }
    
    /// Add a dependency between stages
    pub fn add_dependency(&mut self, stage_id: &str, depends_on: &str) -> Result<()> {
        // Ensure both stages exist
        if !self.registry.has_stage(stage_id) {
            return Err(Error::Stage(format!("Stage not found: {}", stage_id)));
        }
        
        if !self.registry.has_stage(depends_on) {
            return Err(Error::Stage(format!("Dependency stage not found: {}", depends_on)));
        }
        
        // Add the dependency
        self.dependencies
            .entry(stage_id.to_string())
            .or_default()
            .push(depends_on.to_string());
        
        Ok(())
    }
    
    /// Validate the pipeline for cyclic dependencies
    pub fn validate(&self) -> Result<()> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        
        for stage_id in &self.stages {
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
    
    /// Check for cycles in the dependency graph using DFS
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
    fn get_execution_order(&self) -> Result<Vec<String>> {
        // First validate to ensure there are no cycles
        self.validate()?;
        
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new();
        
        // Visit all stages for topological sort
        for stage_id in &self.stages {
            if !visited.contains(stage_id) {
                self.visit_for_topsort(stage_id, &mut visited, &mut temp_mark, &mut result)?;
            }
        }
        
        Ok(result)
    }
    
    /// Visit nodes for topological sort
    fn visit_for_topsort(
        &self,
        stage_id: &str,
        visited: &mut HashSet<String>,
        temp_mark: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<()> {
        if temp_mark.contains(stage_id) {
            return Err(Error::Stage(
                format!("Cannot sort stages: Cyclic dependency found at stage {}", stage_id)
            ));
        }
        
        if !visited.contains(stage_id) {
            temp_mark.insert(stage_id.to_string());
            
            if let Some(deps) = self.dependencies.get(stage_id) {
                for dep in deps {
                    self.visit_for_topsort(dep, visited, temp_mark, result)?;
                }
            }
            
            temp_mark.remove(stage_id);
            visited.insert(stage_id.to_string());
            result.push(stage_id.to_string());
        }
        
        Ok(())
    }
    
    /// Execute the pipeline
    pub fn execute(&mut self, context: &mut StageContext) -> Result<HashMap<String, StageResult>> {
        println!("Executing pipeline: {}", self.name);
        println!("Description: {}", self.description);
        
        if context.is_dry_run() {
            println!("MODE: DRY RUN");
        }
        
        // Get the execution order
        let execution_order = self.get_execution_order()?;
        let mut results = HashMap::new();
        
        // Execute each stage in order
        for stage_id in execution_order {
            let result = self.registry.execute_stage(&stage_id, context)?;
            
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
    /// Flag to indicate whether the pipeline is valid
    valid: bool,
}

impl PipelineBuilder {
    /// Start building a new pipeline
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            pipeline: StagePipeline::new(name, description),
            valid: true,
        }
    }
    
    /// Set the registry to use for the pipeline
    pub fn with_registry(mut self, registry: StageRegistry) -> Self {
        self.pipeline.registry = registry;
        self
    }
    
    /// Add a stage to the pipeline
    pub fn add_stage(mut self, stage_id: &str) -> Self {
        if let Err(e) = self.pipeline.add_stage(stage_id) {
            eprintln!("Error adding stage: {}", e);
            self.valid = false;
        }
        self
    }
    
    /// Add multiple stages to the pipeline
    pub fn add_stages(mut self, stage_ids: &[&str]) -> Self {
        for stage_id in stage_ids {
            if let Err(e) = self.pipeline.add_stage(stage_id) {
                eprintln!("Error adding stage {}: {}", stage_id, e);
                self.valid = false;
            }
        }
        self
    }
    
    /// Add a dependency between stages
    pub fn add_dependency(mut self, stage_id: &str, depends_on: &str) -> Self {
        if let Err(e) = self.pipeline.add_dependency(stage_id, depends_on) {
            eprintln!("Error adding dependency: {} -> {}: {}", stage_id, depends_on, e);
            self.valid = false;
        }
        self
    }
    
    /// Build the pipeline
    pub fn build(self) -> Result<StagePipeline> {
        if !self.valid {
            return Err(Error::Stage("Pipeline build failed due to previous errors".to_string()));
        }
        
        if let Err(e) = self.pipeline.validate() {
            return Err(Error::Stage(format!("Pipeline validation failed: {}", e)));
        }
        
        Ok(self.pipeline)
    }
}