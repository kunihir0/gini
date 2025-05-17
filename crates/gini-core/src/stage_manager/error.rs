//! # Gini Core Stage Manager Errors
//!
//! Defines error types specific to the Gini Stage Management System.
//!
//! This module includes [`StageError`], the primary enum encompassing various
//! errors that can occur during stage definition, registration, dependency
//! resolution, pipeline execution, or context management.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StageSystemError {
    #[error("Stage '{stage_id}' not found")]
    StageNotFound { stage_id: String },

    #[error("Stage '{stage_id}' already exists in the registry")]
    StageAlreadyExists { stage_id: String },

    #[error("Pipeline validation failed: {reason}")]
    PipelineValidationFailed { reason: String },

    #[error("Pipeline '{pipeline_name}' validation: Stage '{stage_id}' not found in registry")]
    StageNotFoundInPipelineValidation { pipeline_name: String, stage_id: String },

    #[error("Pipeline '{pipeline_name}' validation: Dependency stage '{dependency_id}' for stage '{stage_id}' must also be part of the pipeline definition")]
    DependencyStageNotInPipeline { pipeline_name: String, stage_id: String, dependency_id: String },
    
    #[error("Dependency cycle detected in pipeline '{pipeline_name}'. Path: {cycle_path:?}")]
    DependencyCycleDetected { pipeline_name: String, cycle_path: Vec<String> },

    #[error("Stage execution failed for stage '{stage_id}': {source}")]
    StageExecutionFailed {
        stage_id: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    #[error("Invalid stage dependency for stage '{stage_id}': {reason}")]
    InvalidStageDependency { stage_id: String, reason: String },

    #[error("Missing required stages for graph/pipeline '{entity_name}': {missing_stages:?}")]
    MissingStageDependencies { entity_name: String, missing_stages: Vec<String> },

    #[error("Error accessing data from StageContext: Key '{key}' - {reason}")]
    ContextError { key: String, reason: String },

    #[error("Internal Stage Manager error: {0}")]
    InternalError(String),
}