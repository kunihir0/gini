use async_trait::async_trait;
use crate::kernel::error::Result;
use crate::stage_manager::{Stage, StageContext, StageResult};

// --- Core Stage Definitions ---

// TODO: Define PluginDependencyResolution stage if needed as an explicit stage

/// Stage for running plugin pre-flight checks.
#[derive(Debug)]
pub struct PluginPreflightCheckStage;

#[async_trait]
impl Stage for PluginPreflightCheckStage {
    fn id(&self) -> &str { "core::plugin_preflight_check" }
    fn name(&self) -> &str { "Plugin Pre-flight Checks" }
    fn description(&self) -> &str { "Executes pre-initialization checks for all loaded plugins." }

    async fn execute(&self, context: &mut StageContext) -> Result<()> {
        println!("Executing Stage: {}", self.name());
        // TODO: Implement logic to iterate through plugins and run their checks.
        // This will likely involve accessing plugin manager/registry via context.
        // Need to decide how plugins register their checks (e.g., specific trait method, dedicated check objects).
        Ok(())
    }

    fn dry_run_description(&self, _context: &StageContext) -> String {
        format!("Would execute pre-flight checks for loaded plugins.")
    }
}

/// Stage for initializing plugins (calling Plugin::init).
#[derive(Debug)]
pub struct PluginInitializationStage;

#[async_trait]
impl Stage for PluginInitializationStage {
    fn id(&self) -> &str { "core::plugin_initialization" }
    fn name(&self) -> &str { "Plugin Initialization" }
    fn description(&self) -> &str { "Initializes all plugins that passed previous checks." }

    async fn execute(&self, context: &mut StageContext) -> Result<()> {
        println!("Executing Stage: {}", self.name());
        // TODO: Implement logic to iterate through valid plugins and call their init() method.
        // This requires access to the PluginManager/Registry and potentially the Application object.
        // Handle potential errors during init.
        Ok(())
    }

     fn dry_run_description(&self, _context: &StageContext) -> String {
        format!("Would initialize plugins.")
    }
}

/// Stage for running post-initialization logic.
#[derive(Debug)]
pub struct PluginPostInitializationStage;

#[async_trait]
impl Stage for PluginPostInitializationStage {
    fn id(&self) -> &str { "core::plugin_post_initialization" }
    fn name(&self) -> &str { "Plugin Post-Initialization" }
    fn description(&self) -> &str { "Executes logic after all plugins have been initialized." }

    async fn execute(&self, context: &mut StageContext) -> Result<()> {
        println!("Executing Stage: {}", self.name());
        // TODO: Implement logic if any core post-init actions are needed,
        // or allow plugins to hook into this stage if necessary.
        Ok(())
    }

     fn dry_run_description(&self, _context: &StageContext) -> String {
        format!("Would run post-initialization hooks.")
    }
}