use std::fmt;

/// Represents a stage requirement for a plugin
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageRequirement {
    /// The unique identifier of the required stage
    pub stage_id: String,
    
    /// Whether this stage is required or optional
    pub required: bool,
    
    /// Whether this stage is provided by this plugin
    pub provided: bool,
}

impl StageRequirement {
    /// Create a new required stage dependency
    pub fn require(stage_id: &str) -> Self {
        Self {
            stage_id: stage_id.to_string(),
            required: true,
            provided: false,
        }
    }
    
    /// Create a new optional stage dependency
    pub fn optional(stage_id: &str) -> Self {
        Self {
            stage_id: stage_id.to_string(),
            required: false,
            provided: false,
        }
    }
    
    /// Create a new provided stage
    pub fn provide(stage_id: &str) -> Self {
        Self {
            stage_id: stage_id.to_string(),
            required: false,
            provided: true,
        }
    }
    
    /// Check if this requirement is satisfied by the given stage
    pub fn is_satisfied_by(&self, stage_id: &str) -> bool {
        self.stage_id == stage_id
    }
}

impl fmt::Display for StageRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let requirement_type = if self.provided {
            "Provides"
        } else if self.required {
            "Requires"
        } else {
            "Optional"
        };
        
        write!(f, "{} stage: {}", requirement_type, self.stage_id)
    }
}

/// A collection of stage requirements
#[derive(Debug, Default)]
pub struct StageRequirements {
    requirements: Vec<StageRequirement>,
}

impl StageRequirements {
    /// Create a new empty requirements collection
    pub fn new() -> Self {
        Self {
            requirements: Vec::new(),
        }
    }
    
    /// Add a required stage
    pub fn require(&mut self, stage_id: &str) -> &mut Self {
        self.requirements.push(StageRequirement::require(stage_id));
        self
    }
    
    /// Add an optional stage
    pub fn add_optional(&mut self, stage_id: &str) -> &mut Self {
        self.requirements.push(StageRequirement::optional(stage_id));
        self
    }
    
    /// Add a provided stage
    pub fn provide(&mut self, stage_id: &str) -> &mut Self {
        self.requirements.push(StageRequirement::provide(stage_id));
        self
    }
    
    /// Get all requirements
    pub fn all(&self) -> &[StageRequirement] {
        &self.requirements
    }
    
    /// Get required stages
    pub fn get_required(&self) -> Vec<&StageRequirement> {
        self.requirements.iter()
            .filter(|req| req.required && !req.provided)
            .collect()
    }
    
    /// Get optional stages
    pub fn get_optional(&self) -> Vec<&StageRequirement> {
        self.requirements.iter()
            .filter(|req| !req.required && !req.provided)
            .collect()
    }
    
    /// Get provided stages
    pub fn provided(&self) -> Vec<&StageRequirement> {
        self.requirements.iter()
            .filter(|req| req.provided)
            .collect()
    }
}