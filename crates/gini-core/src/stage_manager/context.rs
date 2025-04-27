use std::collections::HashMap;
use std::path::PathBuf;
use crate::kernel::error::Result;

/// Execution mode for stages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Live mode - actually execute operations
    Live,
    /// Dry run mode - simulate operations without executing them
    DryRun,
}

impl ExecutionMode {
    /// Check if this is dry run mode
    pub fn is_dry_run(&self) -> bool {
        matches!(self, ExecutionMode::DryRun)
    }
    
    /// Check if this is live mode
    pub fn is_live(&self) -> bool {
        matches!(self, ExecutionMode::Live)
    }
}

/// Context provided to stages during execution
pub struct StageContext {
    /// The execution mode
    pub mode: ExecutionMode,
    
    /// Configuration directory
    config_dir: PathBuf,
    
    /// Shared data between stages
    shared_data: HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
    
    /// Passed CLI arguments
    cli_args: HashMap<String, String>,
}

impl StageContext {
    /// Create a new context in live mode
    pub fn new_live(config_dir: PathBuf) -> Self {
        Self {
            mode: ExecutionMode::Live,
            config_dir,
            shared_data: HashMap::new(),
            cli_args: HashMap::new(),
        }
    }
    
    /// Create a new context in dry run mode
    pub fn new_dry_run(config_dir: PathBuf) -> Self {
        Self {
            mode: ExecutionMode::DryRun,
            config_dir,
            shared_data: HashMap::new(),
            cli_args: HashMap::new(),
        }
    }
    
    /// Set a CLI argument
    pub fn set_cli_arg(&mut self, key: &str, value: &str) {
        self.cli_args.insert(key.to_string(), value.to_string());
    }
    
    /// Get a CLI argument
    pub fn get_cli_arg(&self, key: &str) -> Option<&str> {
        self.cli_args.get(key).map(|s| s.as_str())
    }
    
    /// Get the configuration directory
    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }
    
    /// Set a shared data value
    pub fn set_data<T: 'static + Send + Sync>(&mut self, key: &str, value: T) {
        self.shared_data.insert(key.to_string(), Box::new(value));
    }
    
    /// Get a shared data value
    pub fn get_data<T: 'static + Send + Sync>(&self, key: &str) -> Option<&T> {
        self.shared_data.get(key).and_then(|data| data.downcast_ref::<T>())
    }
    
    /// Get a mutable reference to a shared data value
    pub fn get_data_mut<T: 'static + Send + Sync>(&mut self, key: &str) -> Option<&mut T> {
        self.shared_data.get_mut(key).and_then(|data| data.downcast_mut::<T>())
    }
    
    /// Check if dry run mode is active
    pub fn is_dry_run(&self) -> bool {
        self.mode.is_dry_run()
    }
    
    /// Execute a function only in live mode
    pub fn execute_live<F>(&self, f: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        if self.mode.is_live() {
            f()
        } else {
            Ok(())
        }
    }
}