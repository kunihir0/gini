use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

/// Trait for operations that can be simulated in dry run mode
pub trait DryRunnable {
    /// Whether this operation supports dry run mode
    fn supports_dry_run(&self) -> bool {
        true // Most operations should support dry run by default
    }
    
    /// Generate a description of what this operation would do
    fn dry_run_description(&self) -> String;
    
    /// Estimate disk usage for this operation
    fn estimated_disk_usage(&self) -> u64 {
        0 // Default is no disk usage
    }
    
    /// Estimate duration for this operation
    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(0) // Default is instant
    }
}

/// Types of file operations for dry run
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOperationType {
    Create,
    Copy,
    Move,
    Delete,
    Modify,
    ChangePermissions,
}

/// File operation that implements DryRunnable
pub struct FileOperation {
    pub operation_type: FileOperationType,
    pub source: PathBuf,
    pub destination: Option<PathBuf>,
    pub permissions: Option<u32>,
    pub content: Option<Vec<u8>>,
}

impl DryRunnable for FileOperation {
    fn dry_run_description(&self) -> String {
        match self.operation_type {
            FileOperationType::Create => {
                format!("Would create file at {}", self.source.display())
            }
            FileOperationType::Copy => {
                if let Some(ref dest) = self.destination {
                    format!("Would copy {} to {}", self.source.display(), dest.display())
                } else {
                    format!("Would copy {}", self.source.display())
                }
            }
            FileOperationType::Move => {
                if let Some(ref dest) = self.destination {
                    format!("Would move {} to {}", self.source.display(), dest.display())
                } else {
                    format!("Would move {}", self.source.display())
                }
            }
            FileOperationType::Delete => {
                format!("Would delete {}", self.source.display())
            }
            FileOperationType::Modify => {
                format!("Would modify {}", self.source.display())
            }
            FileOperationType::ChangePermissions => {
                if let Some(perms) = self.permissions {
                    format!(
                        "Would change permissions of {} to {}",
                        self.source.display(),
                        format!("{:o}", perms)
                    )
                } else {
                    format!("Would change permissions of {}", self.source.display())
                }
            }
        }
    }
    
    fn estimated_disk_usage(&self) -> u64 {
        match self.operation_type {
            FileOperationType::Create | FileOperationType::Copy => {
                if let Some(ref content) = self.content {
                    content.len() as u64
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
}

/// Context for tracking operations in dry run mode
#[derive(Default)]
pub struct DryRunContext {
    pub planned_operations: Vec<Box<dyn DryRunnable>>,
    pub stage_operations: HashMap<String, Vec<Box<dyn DryRunnable>>>,
    pub estimated_disk_usage: u64,
    pub estimated_duration: Duration,
    pub potential_conflicts: Vec<String>,
}

impl DryRunContext {
    /// Create a new dry run context
    pub fn new() -> Self {
        Self {
            planned_operations: Vec::new(),
            stage_operations: HashMap::new(),
            estimated_disk_usage: 0,
            estimated_duration: Duration::from_secs(0),
            potential_conflicts: Vec::new(),
        }
    }
    
    /// Record an operation in the context
    pub fn record_operation<T: DryRunnable + 'static>(&mut self, stage_name: &str, operation: T) {
        // Define simple operation type for tracking
        #[derive(Clone)]
        struct SimpleOperation {
            description: String,
            disk_usage: u64,
            duration: Duration,
        }
        
        impl DryRunnable for SimpleOperation {
            fn dry_run_description(&self) -> String {
                self.description.clone()
            }
            
            fn estimated_disk_usage(&self) -> u64 {
                self.disk_usage
            }
            
            fn estimated_duration(&self) -> Duration {
                self.duration
            }
        }
        
        // Extract values from the operation
        let description = operation.dry_run_description();
        let disk_usage = operation.estimated_disk_usage();
        let duration = operation.estimated_duration();
        
        // Update estimates
        self.estimated_disk_usage += disk_usage;
        self.estimated_duration += duration;
        
        // Create a simple operation for tracking
        let simple_op = SimpleOperation {
            description: description.clone(),
            disk_usage,
            duration,
        };
        
        // Add to stage-specific operations
        self.stage_operations
            .entry(stage_name.to_string())
            .or_default()
            .push(Box::new(operation));
            
        // Add to planned operations
        self.planned_operations.push(Box::new(simple_op));
    }
    
    /// Add a potential conflict
    pub fn add_conflict(&mut self, conflict_description: &str) {
        self.potential_conflicts.push(conflict_description.to_string());
    }
    
    /// Generate a report of planned operations
    pub fn generate_report(&self) -> DryRunReport {
        DryRunReport {
            operations_count: self.planned_operations.len(),
            stages_count: self.stage_operations.len(),
            estimated_disk_usage: self.estimated_disk_usage,
            estimated_duration: self.estimated_duration,
            has_conflicts: !self.potential_conflicts.is_empty(),
        }
    }
}

/// Summary report of a dry run
pub struct DryRunReport {
    pub operations_count: usize,
    pub stages_count: usize,
    pub estimated_disk_usage: u64,
    pub estimated_duration: Duration,
    pub has_conflicts: bool,
}

impl fmt::Display for DryRunReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Dry Run Results:")?;
        writeln!(f, "================")?;
        writeln!(f, "Total operations: {}", self.operations_count)?;
        writeln!(f, "Total stages: {}", self.stages_count)?;
        writeln!(f, "Estimated disk usage: {} bytes", self.estimated_disk_usage)?;
        writeln!(f, "Estimated duration: {:?}", self.estimated_duration)?;
        
        if self.has_conflicts {
            writeln!(f, "WARNING: Potential conflicts detected!")?;
        } else {
            writeln!(f, "No potential conflicts detected")?;
        }
        
        writeln!(f, "\nTo execute these changes, run the same command without the --dry-run flag.")
    }
}