#![cfg(test)]

use std::collections::HashSet;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;
use async_trait::async_trait;

use crate::kernel::bootstrap::Application;
use crate::kernel::component::KernelComponent;
use crate::kernel::error::{Error, Result as KernelResult};
use crate::plugin_system::dependency::PluginDependency;
use crate::plugin_system::traits::{Plugin, PluginPriority, PluginError as TraitsPluginError};
use crate::plugin_system::version::VersionRange;
use crate::stage_manager::{Stage, StageContext, StageResult};
use crate::stage_manager::requirement::StageRequirement;
use crate::storage::manager::DefaultStorageManager;
use crate::plugin_system::manager::DefaultPluginManager;
use std::path::PathBuf;

use super::super::common::{setup_test_environment, TestPlugin, DependentPlugin, ShutdownBehavior, PreflightBehavior, VersionedPlugin};

#[tokio::test]
async fn test_common_plugin_helpers_coverage() {
    // Instantiate common plugins just to call uncovered methods for coverage
    let (plugin_manager, _, _, stages_executed, execution_order, shutdown_order) = setup_test_environment().await;

    let test_plugin = TestPlugin::new("CoverageTestPlugin", stages_executed.clone());
    assert!(!test_plugin.is_core()); // Cover is_core
    assert_eq!(test_plugin.priority(), PluginPriority::ThirdParty(150)); // Cover priority
    assert!(!test_plugin.compatible_api_versions().is_empty()); // Cover compatible_api_versions
    assert!(test_plugin.dependencies().is_empty()); // Cover dependencies
    assert!(test_plugin.required_stages().is_empty()); // Cover required_stages
    assert!(test_plugin.shutdown().is_ok()); // Cover shutdown

    let dep_plugin = DependentPlugin::new(
        "CoverageDependentPlugin", "1.0.0", vec![],
        ShutdownBehavior::Success, PreflightBehavior::Success,
        stages_executed.clone(), execution_order.clone(), shutdown_order.clone()
    );
    assert!(!dep_plugin.is_core()); // Cover is_core
    assert_eq!(dep_plugin.priority(), PluginPriority::ThirdParty(100)); // Cover priority
    assert!(dep_plugin.required_stages().is_empty()); // Cover required_stages

    // Instantiate and check VersionedPlugin
    let versioned_plugin = VersionedPlugin::new( // Use imported path
        "CoverageVersionedPlugin",
        "1.1.0",
        vec!["^1.0".parse().unwrap()],
        stages_executed.clone()
    );
    assert!(!versioned_plugin.is_core());
    assert_eq!(versioned_plugin.priority(), PluginPriority::ThirdParty(100));
    assert!(!versioned_plugin.compatible_api_versions().is_empty());
    assert!(versioned_plugin.dependencies().is_empty());
    assert!(versioned_plugin.required_stages().is_empty());
    let mut app_dummy = Application::new(None).unwrap(); // Need a dummy app for init
    assert!(versioned_plugin.init(&mut app_dummy).is_ok());
    let ctx_dummy = StageContext::new_dry_run(PathBuf::new()); // Dummy context
    assert!(versioned_plugin.preflight_check(&ctx_dummy).await.is_ok());
    assert!(!versioned_plugin.stages().is_empty());
    assert!(versioned_plugin.shutdown().is_ok());

}