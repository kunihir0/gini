# Test Plan: Plugin System Coverage (`crates/gini-core/src/plugin_system/`)

**Objective:** Increase code coverage for the low-coverage files identified in the `crates/gini-core/src/plugin_system/` module: `manifest.rs`, `conflict.rs`, `adapter.rs`, `loader.rs`, `dependency.rs`, and `traits.rs`.

**Methodology:** Implement a combination of unit tests (within the respective modules or their `tests` submodules) and integration tests (primarily in `crates/gini-core/src/tests/integration/plugin_tests.rs`) to cover the logic paths, functions, and error conditions within these files.

---

## 1. `manifest.rs`

*   **Focus:** Structuring plugin metadata (`PluginManifest`, `DependencyInfo`) and the builder pattern (`ManifestBuilder`).
*   **Tests:** Unit Tests (likely in `crates/gini-core/src/plugin_system/tests/manifest_tests.rs` - create if needed)

| Test Name/Purpose                     | Type/Location | Scenario                                                                 | Expected Outcome                                                                 |
| :------------------------------------ | :------------ | :----------------------------------------------------------------------- | :------------------------------------------------------------------------------- |
| `test_manifest_new_defaults`          | Unit          | Call `PluginManifest::new()`                                             | Verify default fields, especially `entry_point` format (`lib{id}.so`).           |
| `test_manifest_builder_methods`       | Unit          | Use `add_api_version`, `add_dependency`, `set_core`, etc.                | Verify each method correctly modifies the corresponding field in the manifest.   |
| `test_manifest_get_priority`          | Unit          | Call `get_priority()` when priority is `None`, Some valid, Some invalid. | Return `None`, `Some(PluginPriority)`, `None` respectively.                      |
| `test_manifest_builder_chaining`      | Unit          | Chain multiple `ManifestBuilder` methods (`description`, `author`, etc.) | Verify the final `build()` result contains all chained modifications correctly. |
| `test_manifest_builder_defaults`      | Unit          | Use `ManifestBuilder::new()` without setting description/author.         | Verify the default description and author are set in the built manifest.       |
| `test_dependency_info_creation`       | Unit          | Create `DependencyInfo` instances.                                       | Verify fields are correctly initialized.                                         |
| `test_manifest_add_multiple_items`    | Unit          | Add multiple dependencies, API versions, tags using builder/methods.     | Verify all items are present in the final manifest lists.                        |
| `test_manifest_priority_parsing_edge` | Unit          | Call `get_priority` after setting priority with edge-case strings.       | Verify correct handling based on `PluginPriority::from_str` logic.             |

---

## 2. `conflict.rs`

*   **Focus:** Conflict types (`ConflictType`), conflict representation (`PluginConflict`), resolution strategies (`ResolutionStrategy`), and management (`ConflictManager`). Note: `detect_conflicts` is a stub.
*   **Tests:** Unit Tests (likely in `crates/gini-core/src/plugin_system/tests/conflict_tests.rs` - create if needed)

| Test Name/Purpose                               | Type/Location | Scenario                                                                                                | Expected Outcome                                                                                             |
| :---------------------------------------------- | :------------ | :------------------------------------------------------------------------------------------------------ | :----------------------------------------------------------------------------------------------------------- |
| `test_conflict_type_is_critical`                | Unit          | Check `is_critical()` for all `ConflictType` variants.                                                  | Returns `true` for critical types, `false` otherwise.                                                        |
| `test_conflict_type_description`                | Unit          | Check `description()` for all `ConflictType` variants.                                                  | Returns the expected descriptive string for each type.                                                       |
| `test_plugin_conflict_new`                      | Unit          | Create a `PluginConflict` using `new()`.                                                                | Verify fields (`first_plugin`, `second_plugin`, `conflict_type`, `description`, `resolved`, `resolution`). |
| `test_plugin_conflict_resolve`                  | Unit          | Call `resolve()` on a `PluginConflict`.                                                                 | `resolved` becomes `true`, `resolution` is set to the provided strategy.                                     |
| `test_plugin_conflict_is_critical`              | Unit          | Call `is_critical()` on conflicts with critical and non-critical types.                                 | Returns the correct boolean based on the underlying `ConflictType`.                                          |
| `test_conflict_manager_new_default`             | Unit          | Create `ConflictManager` via `new()` and `default()`.                                                   | Manager is initialized with an empty `conflicts` vector.                                                     |
| `test_conflict_manager_add_conflict`            | Unit          | Add multiple conflicts to the manager.                                                                  | `get_conflicts()` returns all added conflicts.                                                               |
| `test_conflict_manager_get_unresolved`          | Unit          | Add resolved and unresolved conflicts. Call `get_unresolved_conflicts()`.                               | Returns only the unresolved conflicts.                                                                       |
| `test_conflict_manager_get_critical_unresolved` | Unit          | Add various conflicts (resolved/unresolved, critical/non-critical). Call `get_critical_unresolved...`. | Returns only conflicts that are both critical AND unresolved.                                                |
| `test_conflict_manager_resolve_conflict`        | Unit          | Resolve a conflict by index. Test valid and invalid indices.                                            | Valid index: Conflict is resolved, returns `Ok(())`. Invalid index: Returns `Err`.                           |
| `test_conflict_manager_all_critical_resolved`   | Unit          | Test with no critical conflicts, unresolved critical, resolved critical.                                | Returns `true`, `false`, `true` respectively.                                                                |
| `test_conflict_manager_get_plugins_to_disable`  | Unit          | Add resolved conflicts with `DisableFirst`, `DisableSecond`, other strategies. Test duplicates.         | Returns a sorted, deduplicated list containing only the IDs specified by `DisableFirst`/`DisableSecond`.     |
| `test_conflict_manager_detect_conflicts_stub`   | Unit          | Call the stub `detect_conflicts()` method.                                                              | Returns `Ok(())` without panicking or modifying state significantly.                                         |

---

## 3. `adapter.rs`

*   **Focus:** `Adapter` trait, `AdapterRegistry` for managing adapters, and the `define_adapter!` macro.
*   **Tests:** Unit Tests (likely in `crates/gini-core/src/plugin_system/tests/adapter_tests.rs` - create if needed)
*   **Note:** Requires defining a mock trait and mock adapter implementation for testing.

| Test Name/Purpose                         | Type/Location | Scenario                                                                                             | Expected Outcome                                                                                                |
| :---------------------------------------- | :------------ | :--------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------- |
| `test_adapter_registry_new_default`       | Unit          | Create `AdapterRegistry` via `new()` and `default()`.                                                | Registry is initialized with empty `adapters` and `names` maps.                                                 |
| `test_adapter_registry_register_success`  | Unit          | Register a valid mock adapter.                                                                       | Returns `Ok(())`. Adapter is present in `adapters` and `names` maps. `count()` increments.                      |
| `test_adapter_registry_register_duplicate`| Unit          | Attempt to register adapters with duplicate `TypeId` or duplicate name.                              | Returns `Err` with appropriate message. State remains unchanged from before the failed registration attempt. |
| `test_adapter_registry_get_by_type`       | Unit          | Get existing adapter by type (`get`, `get_mut`). Get non-existent type. Get existing type with wrong cast. | Returns `Some(&Adapter)`, `Some(&mut Adapter)`, `None`, `None` respectively.                                    |
| `test_adapter_registry_get_by_name`       | Unit          | Get existing adapter by name (`get_by_name`, `get_by_name_mut`). Get non-existent name. Get wrong type. | Returns `Some(&Adapter)`, `Some(&mut Adapter)`, `None`, `None` respectively.                                    |
| `test_adapter_registry_has`               | Unit          | Check for existing/non-existent adapters using `has()` and `has_name()`.                             | Returns `true` / `false` accordingly.                                                                           |
| `test_adapter_registry_remove`            | Unit          | Remove existing adapter by type/name. Attempt to remove non-existent adapter.                        | Returns `Some(Box<dyn Adapter>)` for existing, `None` for non-existent. Verify removal from maps. `count()` decrements. |
| `test_adapter_registry_count`             | Unit          | Check `count()` after various register/remove operations.                                            | Returns the correct number of registered adapters.                                                              |
| `test_adapter_registry_names`             | Unit          | Check `names()` after registering/removing adapters.                                                 | Returns a vector containing the names of currently registered adapters.                                         |
| `test_define_adapter_macro`               | Unit          | Define a simple trait, use `define_adapter!`, instantiate, call adapter methods.                     | Macro compiles. Instantiated adapter implements `Adapter` trait correctly (`type_id`, `as_any`, `name`).        |

---

## 4. `loader.rs`

*   **Focus:** Asynchronous plugin scanning (`scan_for_manifests`), manifest loading (stubbed), dependency resolution (`resolve_dependencies`), API compatibility, and plugin registration (`register_all_plugins`).
*   **Tests:** Unit Tests (in `loader.rs` module or `tests/loader_tests.rs`) & Integration Tests (`tests/integration/plugin_tests.rs`)

**Unit Tests:**

| Test Name/Purpose                             | Type/Location | Scenario                                                                                                                            | Expected Outcome                                                                                                                               |
| :-------------------------------------------- | :------------ | :---------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_resolution_error_display`               | Unit          | Format various `ResolutionError` variants using `format!()`.                                                                        | Verify the output strings match the expected format.                                                                                           |
| `test_plugin_loader_new_default`              | Unit          | Create `PluginLoader` via `new()` and `default()`.                                                                                  | Loader is initialized with empty `plugin_dirs` and `manifests`.                                                                                |
| `test_plugin_loader_add_dir`                  | Unit          | Add directories using `add_plugin_dir()`.                                                                                           | The `plugin_dirs` vector contains the added paths.                                                                                             |
| `test_plugin_loader_get_manifests`            | Unit          | Call `get_manifest()` / `get_all_manifests()` on a loader with pre-populated `manifests` map.                                       | Returns the expected `Some(&Manifest)` / `None` or `Vec<&Manifest>`.                                                                          |
| `test_resolve_dependencies_success`           | Unit          | Setup mock manifests with valid, compatible dependencies (required, optional). Call `resolve_dependencies()`.                       | Returns `Ok(())`.                                                                                                                              |
| `test_resolve_dependencies_missing`           | Unit          | Setup mock manifests where a required dependency is missing. Call `resolve_dependencies()`.                                         | Returns `Err(ResolutionError::MissingDependency)`.                                                                                             |
| `test_resolve_dependencies_version_mismatch`  | Unit          | Setup mock manifests where a required dependency has an incompatible version. Call `resolve_dependencies()`.                        | Returns `Err(ResolutionError::VersionMismatch)`.                                                                                               |
| `test_resolve_dependencies_version_parse_err` | Unit          | Setup mock manifests where a plugin or dependency has an invalid version string. Call `resolve_dependencies()`.                     | Returns `Err(ResolutionError::VersionParseError)`.                                                                                             |
| `test_resolve_dependencies_cycle_simple`      | Unit          | Setup mock manifests with a direct cycle (A -> B -> A). Call `resolve_dependencies()`.                                              | Returns `Err(ResolutionError::CycleDetected)` with the correct cycle path.                                                                     |
| `test_resolve_dependencies_cycle_long`        | Unit          | Setup mock manifests with a longer cycle (A -> B -> C -> A). Call `resolve_dependencies()`.                                         | Returns `Err(ResolutionError::CycleDetected)` with the correct cycle path.                                                                     |
| `test_resolve_dependencies_optional_ignored`  | Unit          | Setup mock manifests with missing/incompatible *optional* dependencies. Call `resolve_dependencies()`.                              | Returns `Ok(())` (optional dependencies shouldn't cause resolution failure).                                                                   |

**Integration Tests (`tests/integration/plugin_tests.rs`):**

| Test Name/Purpose                               | Type/Location | Scenario                                                                                                                                                           | Expected Outcome                                                                                                                                                                |
| :---------------------------------------------- | :------------ | :----------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `test_scan_manifests_empty_dirs`                | Integration   | Call `scan_for_manifests()` with empty or non-existent directories configured.                                                                                     | Returns `Ok(Vec::new())`. No errors logged for non-existent dirs.                                                                                                               |
| `test_scan_manifests_valid`                     | Integration   | Setup mock filesystem (using `tempfile`) with valid `manifest.json` files in nested dirs. Call `scan_for_manifests()`.                                              | Returns `Ok` containing the parsed manifests. `manifests` cache in loader is populated. Verify stubbed manifest details (ID from filename).                                     |
| `test_scan_manifests_invalid_json_or_io_error`  | Integration   | Setup mock filesystem with invalid JSON manifests or directories causing I/O errors (e.g., permissions). Call `scan_for_manifests()`.                                | Returns `Ok` containing only valid manifests. Errors are logged to stderr, but the scan continues for other valid manifests.                                                    |
| `test_register_all_plugins_success`             | Integration   | Setup loader with compatible mock manifests (valid deps, compatible API). Setup mock `PluginRegistry`. Call `register_all_plugins()`.                               | Returns `Ok(count)`. `registry.register_plugin` called for each compatible plugin (mocked). (Note: `load_plugin` stub currently returns Err, so count will likely be 0 for now). |
| `test_register_all_plugins_api_incompatible`    | Integration   | Setup loader with some manifests having incompatible `api_versions`. Setup mock `PluginRegistry`. Call `register_all_plugins()`.                                     | Returns `Ok(count)`. Incompatible plugins are skipped. `registry.register_plugin` only called for compatible ones.                                                              |
| `test_register_all_plugins_dep_resolution_fail` | Integration   | Setup loader with manifests causing a dependency resolution error (missing, cycle, etc.). Setup mock `PluginRegistry`. Call `register_all_plugins()`.                 | Returns `Err(KernelError::Plugin)` indicating dependency resolution failure. `registry.register_plugin` is not called.                                                          |
| `test_register_all_plugins_load_fail`           | Integration   | Setup loader with compatible manifests. Setup mock `PluginRegistry`. Call `register_all_plugins()`. (Relies on `load_plugin` stub returning Err).                   | Returns `Ok(0)`. Errors are logged for each failed load attempt. `registry.register_plugin` is not called.                                                                       |

---

## 5. `dependency.rs`

*   **Focus:** Representing individual `PluginDependency` requirements and checking version compatibility.
*   **Tests:** Unit Tests (likely in `crates/gini-core/src/plugin_system/tests/dependency_tests.rs` - create if needed)

| Test Name/Purpose                         | Type/Location | Scenario                                                                                                                            | Expected Outcome                                                                                             |
| :---------------------------------------- | :------------ | :---------------------------------------------------------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------------------------------- |
| `test_dependency_constructors`            | Unit          | Create dependencies using `required`, `required_any`, `optional`, `optional_any`.                                                   | Verify `plugin_name`, `version_range`, and `required` fields are set correctly in the resulting struct.      |
| `test_dependency_is_compatible_no_range`  | Unit          | Call `is_compatible_with()` on a dependency with `version_range = None`.                                                            | Always returns `true`.                                                                                       |
| `test_dependency_is_compatible_with_range`| Unit          | Call `is_compatible_with()` with a valid `VersionRange` and various compatible/incompatible version strings.                        | Returns `true` for compatible versions, `false` for incompatible ones based on `VersionRange::includes()`. |
| `test_dependency_is_compatible_invalid_version` | Unit      | Call `is_compatible_with()` with a valid `VersionRange` but an invalid `version_str` (e.g., "abc").                               | Returns `false`. An error message should be logged to stderr.                                                |
| `test_dependency_display_format`          | Unit          | Format `PluginDependency` instances (required/optional, with/without range) using `format!()`.                                      | Verify the output strings match the expected format (e.g., "Requires plugin: core (version: ^1.0)").       |
| `test_dependency_error_display_format`    | Unit          | Format various `DependencyError` variants using `format!()`.                                                                        | Verify the output strings match the expected format.                                                         |

---

## 6. `traits.rs`

*   **Focus:** Core `Plugin` trait definition, `PluginPriority` enum (parsing, comparison, display), and `PluginError` enum.
*   **Tests:** Unit Tests (likely in `crates/gini-core/src/plugin_system/tests/traits_tests.rs` - create if needed)

| Test Name/Purpose                         | Type/Location | Scenario                                                                                                                            | Expected Outcome                                                                                                                               |
| :---------------------------------------- | :------------ | :---------------------------------------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------- |
| `test_priority_value`                     | Unit          | Get `value()` for each `PluginPriority` variant.                                                                                    | Returns the correct underlying `u8` value.                                                                                                     |
| `test_priority_from_str_valid`            | Unit          | Parse valid priority strings (e.g., "kernel:5", "core:70", "third_party_high:120").                                                 | Returns `Some(PluginPriority)` with the correct variant and value.                                                                             |
| `test_priority_from_str_invalid`          | Unit          | Parse invalid strings (wrong format, unknown type, out-of-range value).                                                             | Returns `None`.                                                                                                                                |
| `test_priority_display_format`            | Unit          | Format `PluginPriority` variants using `format!()`.                                                                                 | Verify the output strings match the expected format (e.g., "core_critical:30").                                                                |
| `test_priority_ordering`                  | Unit          | Compare different `PluginPriority` instances using `<`, `>`, `==`. Test different types and different values within the same type. | Comparisons follow the defined `Ord` implementation (Kernel < CoreCritical < ... < ThirdPartyLow; lower value = higher priority within type). |
| `test_plugin_error_display_format`        | Unit          | Format various `PluginError` variants using `format!()`.                                                                            | Verify the output strings match the expected format.                                                                                           |
| `test_plugin_trait_default_preflight`     | Unit          | Create a minimal mock struct implementing `Plugin`. Call the default `preflight_check()` async method.                              | Returns `Ok(())`.                                                                                                                              |

---

This plan provides a clear roadmap for creating tests to cover the essential logic within the specified `plugin_system` files.