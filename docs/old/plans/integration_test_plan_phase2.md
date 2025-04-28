# Integration Test Plan - Phase 2

## Objective
Increase integration test coverage for `gini-core`, focusing on component interactions and areas identified as low-coverage by the LCOV report.

## Planned Tests

### 1. `event_tests.rs`
*   **Target Modules:** `event/manager.rs`, `event/dispatcher.rs`, `stage_manager/manager.rs`, `plugin_system/manager.rs`
*   **Tests:**
    *   **`test_event_handler_unregistration`**:
        *   **Purpose:** Verify that event handlers can be successfully unregistered.
        *   **Scenario:** Register a handler, dispatch an event (verify call), unregister the handler, dispatch the same event again.
        *   **Expected Outcome:** Handler is called the first time, but not the second time.
        *   **Coverage Aim:** `EventManager::unregister_handler`, `EventDispatcher` internal handler removal logic.
    *   **`test_event_queueing_and_processing`**:
        *   **Purpose:** Verify event queueing behavior and processing order.
        *   **Scenario:** Queue multiple events using `queue_event`, then call `process_queue`. Use handlers to track the order of execution.
        *   **Expected Outcome:** Events are processed in the order they were queued (or based on priority if implemented later). Handlers are called sequentially.
        *   **Coverage Aim:** `EventManager::queue_event`, `EventManager::process_queue`, `EventDispatcher` queue logic.
    *   **`test_stage_dispatches_event`**:
        *   **Purpose:** Verify a stage can dispatch an event during its execution.
        *   **Scenario:** Create a stage that accesses the `EventManager` via `StageContext` and dispatches a custom event. Register a handler for this event. Execute the stage within a pipeline.
        *   **Expected Outcome:** The event handler is called with the correct event data after the stage executes.
        *   **Coverage Aim:** `StageContext` access to `EventManager`, `EventManager::dispatch` usage within a stage.
    *   **`test_plugin_dispatches_event`**:
        *   **Purpose:** Verify a plugin's method (e.g., `init`) can dispatch an event.
        *   **Scenario:** Create a plugin whose `init` method accesses the `EventManager` (likely via `Application` passed to `init`) and dispatches an event. Register a handler. Initialize the plugin.
        *   **Expected Outcome:** The event handler is called when the plugin is initialized.
        *   **Coverage Aim:** Plugin access to `EventManager`, `EventManager::dispatch` usage within a plugin.

### 2. `plugin_tests.rs`
*   **Target Modules:** `plugin_system/manager.rs`, `plugin_system/registry.rs`, `plugin_system/loader.rs`, `plugin_system/conflict.rs`, `storage/manager.rs`, `stage_manager/core_stages.rs`
*   **Tests:**
    *   **`test_plugin_preflight_check_failure_handling`**:
        *   **Purpose:** Verify the system correctly handles plugins failing their preflight checks.
        *   **Scenario:** Create a plugin configured to fail its `preflight_check`. Register it. Execute the preflight check stage (or trigger initialization which includes it).
        *   **Expected Outcome:** The preflight check stage reports an error or the plugin initialization fails. The plugin should not be marked as fully initialized or active.
        *   **Coverage Aim:** `Plugin::preflight_check` integration, `PluginPreflightCheckStage` execution, error handling in `PluginRegistry` or `PluginManager`.
    *   **`test_plugin_conflict_detection_and_resolution`**:
        *   **Purpose:** Verify detection of conflicting plugins and potential resolution (if applicable).
        *   **Scenario:** Create two plugins that conflict (e.g., provide the same resource or have incompatible requirements). Register both. Trigger conflict detection (e.g., during dependency checks or initialization).
        *   **Expected Outcome:** Conflict is detected. If resolution mechanisms exist, test them; otherwise, verify appropriate errors are reported.
        *   **Coverage Aim:** `plugin_system/conflict.rs`, conflict detection logic within `PluginManager` or `PluginRegistry`.
    *   **`test_plugin_loading_from_directory`**:
        *   **Purpose:** Verify loading of shared object (`.so`/`.dylib`/`.dll`) plugins from a directory.
        *   **Scenario:** (Requires test setup with a compiled dummy plugin artifact) Configure `PluginManager` with a directory containing the dummy plugin. Call `load_plugins_from_directory`.
        *   **Expected Outcome:** The plugin is loaded, registered, and appears in the plugin list. Its manifest data is correctly parsed.
        *   **Coverage Aim:** `plugin_system/loader.rs`, `PluginManager::load_plugins_from_directory`, manifest parsing from loaded plugins.
    *   **`test_plugin_interaction_with_storage`**:
        *   **Purpose:** Verify a plugin can interact with the storage system via the context or dependency injection.
        *   **Scenario:** Create a plugin whose `init` or a stage method accesses the `StorageManager`. Have it write a test file and then read it back.
        *   **Expected Outcome:** The plugin successfully writes and reads data using the `StorageManager`.
        *   **Coverage Aim:** Plugin access to `StorageManager`, integration between `PluginManager` and `StorageManager`.

### 3. `stage_tests.rs`
*   **Target Modules:** `stage_manager/context.rs`, `stage_manager/pipeline.rs`, `stage_manager/manager.rs`, `storage/manager.rs`, `event/manager.rs`, `stage_manager/dependency.rs`
*   **Tests:**
    *   **`test_stage_context_complex_data_types`**:
        *   **Purpose:** Verify passing of non-primitive data types (structs, Vecs, HashMaps) via context.
        *   **Scenario:** Stage A sets a custom struct in context. Stage B retrieves and verifies the struct's content.
        *   **Expected Outcome:** Stage B successfully retrieves and validates the complex data structure.
        *   **Coverage Aim:** `StageContext::set_data`, `StageContext::get_data` with complex types, `Any` type handling.
    *   **`test_stage_context_data_modification`**:
        *   **Purpose:** Verify that data set by one stage can be modified by a subsequent stage.
        *   **Scenario:** Stage A sets `context_data["value"] = 1`. Stage B retrieves the data, increments it, and sets `context_data["value"] = 2`. Stage C retrieves and verifies the value is 2.
        *   **Expected Outcome:** Stage C reads the modified value (2).
        *   **Coverage Aim:** `StageContext::get_data_mut`, modification of existing context data.
    *   **`test_stage_interaction_with_storage`**:
        *   **Purpose:** Verify a stage can interact with the storage system via the context.
        *   **Scenario:** Create a stage that accesses the `StorageManager` via `StageContext`. Have it write a test file during execution.
        *   **Expected Outcome:** The stage successfully writes data using the `StorageManager`. The file exists after pipeline execution.
        *   **Coverage Aim:** `StageContext` access to `StorageManager`, `StorageManager` usage within a stage.
    *   **`test_stage_interaction_with_event_manager`**:
        *   **Purpose:** Verify a stage can dispatch events via the context.
        *   **Scenario:** Create a stage that accesses the `EventManager` via `StageContext` and dispatches an event. Register a handler for the event. Execute the stage.
        *   **Expected Outcome:** The event handler is called.
        *   **Coverage Aim:** `StageContext` access to `EventManager`, `EventManager::dispatch` usage within a stage.
    *   **`test_pipeline_optional_dependency_handling`**:
        *   **Purpose:** Verify correct pipeline execution when optional stage dependencies are involved.
        *   **Scenario:** Define stages A, B, C. Pipeline requires A and C. C has an *optional* dependency on B. Run the pipeline once with only A and C registered, then again with A, B, and C registered.
        *   **Expected Outcome:** Pipeline executes successfully in both cases. Execution order reflects the optional dependency when B is present.
        *   **Coverage Aim:** `PipelineBuilder::add_dependency` (if it supports optional), dependency resolution logic in `StageManager` or `Pipeline`, `stage_manager/dependency.rs`.

### 4. `storage_tests.rs`
*   **Target Modules:** `storage/manager.rs`, `storage/local.rs` (and potentially other providers if added)
*   **Tests:**
    *   **`test_storage_directory_operations`**:
        *   **Purpose:** Verify creation, reading, and deletion of directories.
        *   **Scenario:** Use `create_dir`, check existence, create files inside, use `read_dir` to list contents, use `remove_dir` (for empty) and `remove_dir_all` (for non-empty).
        *   **Expected Outcome:** Directory operations succeed/fail as expected according to filesystem rules. `read_dir` returns correct entries.
        *   **Coverage Aim:** `StorageProvider::create_dir`, `read_dir`, `remove_dir`, `remove_dir_all`.
    *   **`test_storage_copy_and_rename`**:
        *   **Purpose:** Verify file copying and renaming functionality.
        *   **Scenario:** Create file A. Copy A to B. Verify B exists with correct content. Rename B to C. Verify C exists and B does not.
        *   **Expected Outcome:** Files are copied and renamed correctly. Contents are preserved. Original files are removed after rename.
        *   **Coverage Aim:** `StorageProvider::copy`, `StorageProvider::rename`.
    *   **`test_storage_metadata_retrieval`**:
        *   **Purpose:** Verify retrieval of file metadata.
        *   **Scenario:** Create a file. Use `metadata` to get its properties (size, modification time, type).
        *   **Expected Outcome:** Metadata is retrieved successfully and reflects the file's properties.
        *   **Coverage Aim:** `StorageProvider::metadata`.
    *   **`test_storage_byte_level_operations`**:
        *   **Purpose:** Verify reading and writing raw byte data.
        *   **Scenario:** Use `write_bytes` to save byte array. Use `read_to_bytes` to retrieve it.
        *   **Expected Outcome:** The retrieved byte array matches the original saved array.
        *   **Coverage Aim:** `StorageProvider::write_bytes`, `StorageProvider::read_to_bytes`.
    *   **`test_storage_append_operations`**:
        *   **Purpose:** Verify appending data to existing files.
        *   **Scenario:** Create a file with initial content. Use `open_append` and write additional data. Read the entire file content back.
        *   **Expected Outcome:** The final file content contains both the initial and appended data.
        *   **Coverage Aim:** `StorageProvider::open_append`.

### 5. `ui_tests.rs`
*   **Target Modules:** `ui_bridge/mod.rs`, `ui_bridge/messages.rs`
*   **Tests:**
    *   **`test_ui_provider_registration_and_default`**:
        *   **Purpose:** Verify registration of UI providers and setting the default.
        *   **Scenario:** Create a `UiBridge`. Register a mock `ConsoleUiProvider` and a custom mock provider. Set the custom provider as default. Verify `get_default_provider_name` returns the correct name.
        *   **Expected Outcome:** Providers are registered. Default provider can be set and retrieved.
        *   **Coverage Aim:** `UiBridge::register_provider`, `UiBridge::set_default_provider`, `UiBridge::get_default_provider_name`.
    *   **`test_ui_bridge_message_dispatch`**:
        *   **Purpose:** Verify that messages sent via the `UiBridge` reach the registered provider.
        *   **Scenario:** Register a mock provider with methods to track received messages. Use `UiBridge::log`, `status`, `progress`, `dialog` helper methods.
        *   **Expected Outcome:** The mock provider's `handle_message` method is called with the correctly formatted `UiMessage` for each helper method used.
        *   **Coverage Aim:** `UiBridge::send_message`, `UiBridge` helper methods (`log`, `status`, etc.), `UiProvider::handle_message`.
    *   **`test_ui_provider_lifecycle_calls`**:
        *   **Purpose:** Verify that the `initialize` and `finalize` methods on providers are called by the bridge.
        *   **Scenario:** Register a mock provider that tracks calls to `initialize` and `finalize`. Call `UiBridge::initialize` and `UiBridge::finalize`.
        *   **Expected Outcome:** The mock provider's `initialize` and `finalize` methods are called exactly once.
        *   **Coverage Aim:** `UiBridge::initialize`, `UiBridge::finalize`, `UiProvider::initialize`, `UiProvider::finalize`.