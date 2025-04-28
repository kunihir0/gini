# Plugin Lifecycle Stages Design

This document details the design for new core stages within the `StageManager` to support enhanced plugin lifecycle management, specifically for pre-flight checks and coordinated initialization.

## 1. Motivation

To support scenarios like compatibility checks (`compat-check` plugin) before another plugin (`vfio-setup`) runs, we need points in the application startup sequence to execute plugin-specific logic *before* the main plugin initialization occurs. Adding dedicated stages provides a structured way to manage this.

## 2. Proposed Mechanism

Instead of adding new lifecycle hooks directly to the `Plugin` trait, we will introduce new standard stages managed by the `StageManager`. This leverages the existing stage execution infrastructure and keeps lifecycle orchestration centralized.

## 3. New Core Stages

The following new stages will be defined and registered by the kernel during bootstrap. They represent distinct phases in the plugin lifecycle *after* manifests are loaded but *before* the main application logic begins.

*   **`CoreStage::PluginDependencyResolution`**
    *   **Purpose:** Ensures all plugin dependencies (declared in manifests) are met, including version constraints, and checks for circular dependencies.
    *   **Execution:** This might run implicitly within the `PluginManager` or `PluginLoader` *before* any stages are formally executed, or it could be the very first explicit stage in the plugin-related part of the startup pipeline. If explicit, it sets the foundation for subsequent stages.
    *   **Failure:** If required dependencies are not met, relevant plugins are marked as unloadable, preventing them from proceeding to later stages. Errors are reported.

*   **`CoreStage::PluginPreflightCheck`**
    *   **Purpose:** Allows plugins to perform checks *after* dependencies are confirmed but *before* their main initialization logic (`Plugin::init`) runs. Examples: hardware compatibility checks, required system library checks, configuration validation, resource availability checks.
    *   **Execution:** Runs after successful dependency resolution. The `StageManager` executes this stage. Plugins can contribute checks by:
        1.  Providing their own `Stage` implementation designed to run during this phase (e.g., by declaring a dependency on a hypothetical `PreflightAnchor` stage or similar mechanism if stage dependencies are implemented).
        2.  (Alternative, less preferred for now) A potential future enhancement could involve the `Plugin` trait providing a list of simple check functions/objects specifically for this stage.
    *   **Failure:** If a critical pre-flight check fails, the corresponding plugin might be prevented from initializing, or the entire application startup might halt, depending on the check's severity. Errors are reported.

*   **`CoreStage::PluginInitialization`**
    *   **Purpose:** Executes the main initialization logic for all plugins that have successfully passed dependency and pre-flight checks. This typically involves calling the `Plugin::init` method for each valid plugin.
    *   **Execution:** Runs after `PluginPreflightCheck`. The order of `init` calls might follow the topological sort from dependency resolution or run concurrently if safe.
    *   **Failure:** Errors during `Plugin::init` should be handled, potentially preventing the plugin from being fully active.

*   **`CoreStage::PluginPostInitialization`**
    *   **Purpose:** Provides a synchronization point after all plugins have attempted initialization. Useful for plugins that need to interact with other plugins *after* knowing they have been initialized.
    *   **Execution:** Runs after `PluginInitialization` completes for all plugins.
    *   **Failure:** Handled based on the logic within stages running in this phase.

## 4. Integration

*   **Registration:** The `kernel` or `bootstrap` module will be responsible for creating instances of these core stages and registering them with the `StageManager` early in the application lifecycle.
*   **Pipeline:** The main application startup pipeline (managed by `StageManager`) must include these stages in the correct logical order relative to other startup tasks like configuration loading, event system initialization, etc. A likely order segment:
    1.  ... (Config Load, etc.)
    2.  (`PluginDependencyResolution` - if explicit stage)
    3.  `PluginPreflightCheck`
    4.  `PluginInitialization`
    5.  `PluginPostInitialization`
    6.  ... (Main Application Stages, UI Start, etc.)
*   **`StageContext`:** The `StageContext` passed between stages may need to be augmented to carry information about the status of plugins (e.g., which passed checks, which failed) to inform the execution of subsequent stages.
*   **Error Handling:** The `StageManager` and `StagePipeline` execution logic must gracefully handle failures within these new stages. A failure in `PluginPreflightCheck` for Plugin A should prevent Plugin A from reaching `PluginInitialization`, but might not necessarily stop Plugin B from initializing if its checks passed (unless the failure is deemed critical for the whole application).

## 5. Example Flow (Mermaid)

```mermaid
graph TD
    subgraph Kernel Bootstrap
        A[Discover Plugins] --> B(Load Manifests);
        B --> C{Resolve Dependencies};
        C -- Success --> D[Register Core Stages];
        C -- Failure --> E[Report Dependency Errors];
    end

    subgraph Stage Execution (via StageManager)
        D --> F[Stage: PluginPreflightCheck];
        F -- Success --> G[Stage: PluginInitialization (Plugin::init)];
        F -- Failure --> H[Report Preflight Errors / Skip Plugin];
        G --> I[Stage: PluginPostInitialization];
        I --> J[Stage: Regular Application Stages...];
    end

    E --> Z((Stop/Error State));
    H --> Z;
```

This stage-based approach provides clear extension points for complex plugin interactions while maintaining separation of concerns between the plugin logic and the lifecycle orchestration.