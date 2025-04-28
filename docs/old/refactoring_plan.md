## Gini Workspace Refactoring Plan

**1. Goal:** Refactor the `gini` binary crate into a Cargo workspace containing a core library crate (`gini-core`) and a thin binary crate (`gini`) to enable building plugins as separate crates.

**2. Proposed Directory Structure:**

```
/home/xiao/Documents/source/repo/gini/  (Project Root)
├── .gitignore
├── Cargo.lock                  (Will be updated/regenerated)
├── Cargo.toml                  (NEW: Workspace definition)
├── CONTRIBUTING.md
├── docs/                       (Remains unchanged)
│   └── ...
│   └── refactoring_plan.md     (NEW: This plan)
├── plugins/                    (Remains unchanged, future plugin location)
│   └── ...
├── crates/                     (NEW: Contains workspace members)
│   ├── gini-core/              (NEW: Core library crate)
│   │   ├── Cargo.toml          (NEW: gini-core package definition)
│   │   └── src/                (NEW: Source code for gini-core)
│   │       ├── lib.rs          (NEW: Library root)
│   │       ├── event/          (MOVED from old src/)
│   │       ├── kernel/         (MOVED from old src/, including tests)
│   │       ├── plugin_system/  (MOVED from old src/, including tests)
│   │       ├── stage_manager/  (MOVED from old src/)
│   │       ├── storage/        (MOVED from old src/)
│   │       ├── ui_bridge/      (MOVED from old src/)
│   │       └── utils/          (MOVED from old src/)
│   └── gini/                   (NEW: Thin binary crate)
│       ├── Cargo.toml          (NEW: gini binary package definition)
│       └── src/                (NEW: Source code for gini binary)
│           └── main.rs         (MOVED and MODIFIED from old src/)
└── src/                        (OLD: To be removed after migration)
    └── ...
```

*   **Top-level files:** `.gitignore`, `CONTRIBUTING.md`, `docs/`, `plugins/` remain at the project root.
*   **`Cargo.lock`:** Will be managed by Cargo at the workspace level.
*   **Old `src/`:** The original `src/` directory will be removed once all its contents are successfully migrated.

**3. `Cargo.toml` Specifications:**

*   **Root `/Cargo.toml`:**
    ```toml
    [workspace]
    resolver = "2"
    members = [
        "crates/gini-core",
        "crates/gini",
        # Add future plugin crates here, e.g., "plugins/gini-plugin-example"
    ]

    # Optional: Define workspace-level dependencies if shared across many crates
    # [workspace.dependencies]
    # tokio = { version = "1", features = ["rt", "rt-multi-thread", "macros", "fs", "sync"] }
    # async-trait = "0.1"
    # ...

    # Optional: Define workspace-level metadata
    [workspace.package]
    version = "0.1.0" # Manage version centrally
    authors = ["Your Name <you@example.com>"] # Replace with actual authors
    edition = "2024"
    # license = "..."
    # repository = "..."
    ```
    *(Note: Added `workspace.package` for central version management as requested.)*

*   **`crates/gini-core/Cargo.toml`:**
    ```toml
    [package]
    name = "gini-core"
    version.workspace = true # Inherit from workspace
    edition.workspace = true # Inherit from workspace
    authors.workspace = true # Inherit from workspace
    # description = "Core library for the Gini plugin engine"
    # license.workspace = true
    # repository.workspace = true

    [lib]
    name = "gini_core"
    path = "src/lib.rs"

    [dependencies]
    # Dependencies moved from the original gini crate
    tokio = { version = "1", features = ["rt", "rt-multi-thread", "macros", "fs", "sync"] }
    # Or use workspace dependency: tokio = { workspace = true }
    async-trait = "0.1"
    # Or use workspace dependency: async-trait = { workspace = true }
    tokio-stream = "0.1"
    # Or use workspace dependency: tokio-stream = { workspace = true }
    semver = "1.0.26"
    # Or use workspace dependency: semver = { workspace = true }
    thiserror = "2.0.12" # Assuming this is used by core error types
    # Or use workspace dependency: thiserror = { workspace = true }

    [dev-dependencies]
    # If tests moved here require tempfile
    tempfile = "3.10"
    # Or use workspace dependency: tempfile = { workspace = true }
    ```

*   **`crates/gini/Cargo.toml`:**
    ```toml
    [package]
    name = "gini"
    version.workspace = true # Inherit from workspace
    edition.workspace = true # Inherit from workspace
    authors.workspace = true # Inherit from workspace

    [[bin]]
    name = "gini"
    path = "src/main.rs"

    [dependencies]
    gini-core = { path = "../gini-core", version = "0.1.0" } # Explicit version match for path dependency
    # Add any dependencies ONLY needed by main.rs itself (e.g., command-line arg parsing like clap)
    # tokio = { version = "1", features = ["rt-multi-thread", "macros"] } # Only if main needs its own runtime handle directly
    # Or use workspace dependency: tokio = { workspace = true, features = [...] }

    [dev-dependencies]
    # Keep tempfile here ONLY if integration tests specific to the binary remain/are added here.
    # tempfile = "3.10"
    # Or use workspace dependency: tempfile = { workspace = true }
    ```

**4. Code Migration Plan:**

*   **Move:** All directories and `.rs` files from the original `src/` directory **except** `src/main.rs` will be moved into the new `crates/gini-core/src/` directory.
    *   `src/event/` -> `crates/gini-core/src/event/`
    *   `src/kernel/` -> `crates/gini-core/src/kernel/`
    *   `src/plugin_system/` -> `crates/gini-core/src/plugin_system/`
    *   `src/stage_manager/` -> `crates/gini-core/src/stage_manager/`
    *   `src/storage/` -> `crates/gini-core/src/storage/`
    *   `src/ui_bridge/` -> `crates/gini-core/src/ui_bridge/`
    *   `src/utils/` -> `crates/gini-core/src/utils/`
*   **Create `crates/gini-core/src/lib.rs`:**
    ```rust
    // Declare modules moved from the old src/
    pub mod event;
    pub mod kernel;
    pub mod plugin_system;
    pub mod stage_manager;
    pub mod storage;
    pub mod ui_bridge;
    pub mod utils;
    // Potentially declare top-level error types if they exist
    // pub mod error;

    // Re-export key public types/traits for easier use by the binary and plugins
    // Example:
    pub use kernel::{Kernel, KernelError}; // Assuming KernelError exists
    pub use plugin_system::{Plugin, PluginError, PluginManifest}; // Assuming these exist
    pub use stage_manager::{StageManager, Stage, StageContext}; // Assuming these exist
    pub use event::{Event, EventDispatcher}; // Assuming these exist
    // ... add other necessary pub use statements for the public API ...

    // Example of how tests within modules are declared (no change needed if already like this)
    // Inside kernel/mod.rs:
    // #[cfg(test)]
    // mod tests;

    // Inside plugin_system/mod.rs:
    // #[cfg(test)]
    // mod tests;
    ```
*   **Update `crates/gini/src/main.rs`:**
    *   Move the original `src/main.rs` to `crates/gini/src/main.rs`.
    *   Modify all `use` statements that previously referenced `crate::module` to now reference `gini_core::module`.
    *   Example: Change `use crate::kernel::Kernel;` to `use gini_core::kernel::Kernel;` or simply `use gini_core::Kernel;` if re-exported in `gini_core/src/lib.rs`.
    *   Ensure `main` function signature and logic remain appropriate for the application entry point.

**5. Test Migration Plan:**

*   **Move Module Tests:** The test modules (`tests/`) located within the moved source directories (`src/kernel/tests/`, `src/plugin_system/tests/`) should be moved along with their parent modules into `crates/gini-core/src/`.
    *   `src/kernel/tests/` -> `crates/gini-core/src/kernel/tests/`
    *   `src/plugin_system/tests/` -> `crates/gini-core/src/plugin_system/tests/`
*   **Adjust Test Paths:** `use` paths within these tests might need adjustment. If they used `super::` or `crate::`, they might need to change depending on the new structure and `pub use` statements in `gini_core/src/lib.rs`. Paths like `crate::some_module` will likely become `gini_core::some_module` or just reference items via `super::` if testing private module internals.
*   **`tempfile` Dependency:** Since `tempfile` is listed under `[dev-dependencies]` in the original `Cargo.toml`, it's likely used by tests. These tests are probably within the modules being moved to `gini-core`. Therefore, `tempfile` should be added to the `[dev-dependencies]` section of `crates/gini-core/Cargo.toml`. If any integration tests remain specifically for the binary in `crates/gini/tests/` (a directory that would need to be created), they might also need `tempfile` in `crates/gini/Cargo.toml`'s `[dev-dependencies]`. For now, assume it's needed in `gini-core`.

**6. Step-by-Step Refactoring Sequence:**

1.  **Create Directories:**
    *   `mkdir -p crates/gini-core/src`
    *   `mkdir -p crates/gini/src`
2.  **Create `Cargo.toml` Files:**
    *   Create `/Cargo.toml` (workspace root) with the content specified in section 3.
    *   Create `crates/gini-core/Cargo.toml` with the content specified in section 3.
    *   Create `crates/gini/Cargo.toml` with the content specified in section 3.
3.  **Move Source Code:**
    *   `mv src/event crates/gini-core/src/`
    *   `mv src/kernel crates/gini-core/src/`
    *   `mv src/plugin_system crates/gini-core/src/`
    *   `mv src/stage_manager crates/gini-core/src/`
    *   `mv src/storage crates/gini-core/src/`
    *   `mv src/ui_bridge crates/gini-core/src/`
    *   `mv src/utils crates/gini-core/src/`
    *   `mv src/main.rs crates/gini/src/`
4.  **Create/Update Core Files:**
    *   Create `crates/gini-core/src/lib.rs` with the content specified in section 4.
    *   Modify `crates/gini/src/main.rs` as specified in section 4 (update `use` paths).
5.  **Adjust Internal Paths:**
    *   Recursively search and replace `use crate::` with `use gini_core::` within all `.rs` files inside `crates/gini-core/src/` and `crates/gini/src/main.rs`. Be mindful of context; some `crate::` might refer to the *current* crate (`gini-core` or `gini`) and should remain, while others refer to what *was* the top-level crate and now need `gini_core::`. Careful review is needed. Paths like `super::` should generally still work within modules.
    *   Review paths within tests (`crates/gini-core/src/*/tests/*.rs`) carefully.
6.  **Iterative Checks and Testing:**
    *   Run `cargo check --workspace` from the project root. Address any compilation errors (path issues, missing dependencies, visibility problems).
    *   Run `cargo test --workspace` from the project root. Address any test failures. This might involve fixing test code paths or ensuring test setup works correctly in the new structure.
    *   Repeat checks and tests until all errors and failures are resolved.
7.  **Cleanup:** Once confident, delete the now-empty original `src/` directory: `rm -rf src/`.
8.  **Commit:** Commit the changes as a single, logical refactoring step.

**7. Verification:**

*   The primary verification method is ensuring `cargo check --workspace` and `cargo test --workspace` pass without errors or failures from the project root directory (`/home/xiao/Documents/source/repo/gini/`).
*   Additionally, run `cargo build --workspace` to confirm both the library and binary build successfully.
*   Run the binary (`cargo run --package gini` or `./target/debug/gini`) to perform basic smoke tests and ensure it still functions as expected.