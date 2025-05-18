# Generic VM Setup Workflow Design

This document outlines the proposed workflow involving key plugins for environment checking, generic virtual machine (VM) hardware definition (QEMU/KVM Setup), and guest-specific setup (e.g., Linux Distro Setup).

*Note: This workflow assumes the application's initial first-run setup (handled by the kernel or early core stages) has already completed, ensuring necessary user directories and potentially default configuration files (like the VM TOML template) exist.*
## 1. Overall Workflow

The process involves sequential execution of plugins:

1.  **Plugin: Environment Check:** Gathers detailed information about the host system.
2.  **Plugin: QEMU/KVM Setup:** Acts as the **generic VM hardware definition layer**. It reads host information and user configuration (from TOML via storage) to define the VM's hardware specifications (CPU, RAM, devices, passthrough) independent of the guest OS. It may optionally generate hypervisor-specific configurations like libvirt XML.
3.  **Guest-Specific Setup Plugin(s):** These plugins consume the generic VM definition from QEMU/KVM Setup and perform actions specific to the target guest OS. The primary example for initial development is:
    *   `Plugin: Linux Distro Setup`: Prepares cloud-init data, sets hostname, or performs other initial setup steps for Linux guests.
    Other examples could include:
    *   `Plugin: OpenCore Builder` (for macOS): Generates the OpenCore EFI.
    *   `Plugin: Windows Prep` (Hypothetical): Configures specific drivers or settings needed for Windows.

## 2. Plugin Purposes & Responsibilities

*Note: Plugins can also implement the `Plugin::preflight_check` method to perform essential validations (e.g., checking host capabilities, TOML sanity) before the main stage pipeline begins.*

### Plugin: Environment Check
*   **Purpose:** To collect comprehensive data about the host system's hardware and software environment relevant to virtualization in general. This plugin *only gathers information*.
*   **Key Information Gathered:**
    *   Host OS/Distro Information
    *   Kernel Version & Features (e.g., KVM support)
    *   CPU Details (Model, Vendor, Features like VT-x/AMD-V)
    *   Memory Details (Total RAM, Huge Pages status)
    *   GPU Details (Vendor, Model for all detected GPUs)
    *   IOMMU Status (Enabled, Groupings)
    *   Relevant Kernel Modules (e.g., `kvm`, `vfio-pci`)
    *   Storage Device Information (Optional, for passthrough considerations)
    *   Network Interface Information (Optional)

### Plugin: QEMU/KVM Setup
*   **Purpose:** To act as the **generic VM hardware definition layer**. It defines the specific parameters and configuration of the target virtual machine based on user intent (from TOML configuration) and validated against the host capabilities (from Environment Check). It handles the core VM hardware definition, including resource allocation and passthrough setup, independent of the guest OS.
*   **Inputs:**
    *   Environment Check Data (Host capabilities)
    *   User Configuration (from TOML via Storage System, typically loaded from `StorageManager::user_config_path()`. The plugin can define its expected TOML structure via the `config_schema` field in its `PluginManifest`.)
*   **Key Tasks:**
    *   Load and validate user's VM configuration from TOML against host capabilities.
    *   Define VM CPU topology and RAM allocation.
    *   Define virtual storage devices (e.g., virtio-blk, NVMe).
    *   Define virtual network interfaces (e.g., virtio-net).
    *   Define basic virtual devices (Input, USB controllers).
    *   **Configure VFIO Passthrough:** If requested in TOML and supported by the host (checked using Environment Check data), configure PCI passthrough for GPUs or other devices. This includes identifying devices, IOMMU groups, and necessary kernel driver bindings.
    *   **Generate Libvirt XML (Optional):** Provide a stage to generate a `domain.xml` file compatible with libvirt/virt-manager based on the defined VM configuration.
    *   Finalize the **Defined VM Configuration** data structure.
*   **Output:** A structured representation of the fully defined VM configuration (e.g., `session.vm_definition`). Optionally, a libvirt XML file (potentially written to a subdirectory within `StorageManager::user_data_path()`).

### Example Guest-Specific Plugin: Linux Distro Setup
*   **Purpose:** As the primary example guest-specific plugin, this prepares common configurations for Linux distributions running within the VM defined by the QEMU/KVM Setup plugin.
*   **Inputs:**
    *   Defined VM Configuration (from QEMU/KVM Setup plugin - virtual hardware details)
    *   User Configuration (from TOML - e.g., desired hostname, cloud-init user-data source)
*   **Key Tasks:**
    *   Load Linux-specific configuration from TOML.
    *   Generate cloud-init meta-data and user-data files/ISO.
    *   Configure VM settings relevant to Linux guests (e.g., ensuring virtio drivers are primary).
    *   Potentially prepare installation media or scripts.
    *   Set hostname within cloud-init or via other mechanisms if applicable.
*   **Output:** Configuration files (e.g., cloud-init data ISO) or scripts (potentially written to a subdirectory within `StorageManager::user_data_path()`) ready to be used by the Linux VM during first boot.

## 3. Key Stages & Dependencies (Examples)

*   **Environment Check Stages:**
    *   `env_check:gather_os_info`
    *   `env_check:gather_cpu_info`
    *   `env_check:gather_memory_info`
    *   `env_check:gather_gpu_info`
    *   `env_check:check_iommu`
    *   `env_check:finalize_host_report` (Outputs `session.environment_data`)

*   **QEMU/KVM Setup Stages:**
    *   `qemu_setup:load_vm_config_toml` (Inputs: TOML path; Outputs: `session.user_vm_request`)
    *   `qemu_setup:validate_vm_request` (Inputs: `session.environment_data`, `session.user_vm_request`)
    *   `qemu_setup:define_cpu_ram`
    *   `qemu_setup:define_storage`
    *   `qemu_setup:define_networking`
    *   `qemu_setup:configure_vfio` (Conditional; Inputs: `session.environment_data`, `session.user_vm_request`)
    *   `qemu_setup:finalize_vm_definition` (Outputs: `session.vm_definition`)
    *   `qemu_setup:generate_libvirt_xml` (Optional; Inputs: `session.vm_definition`)

*   **Linux Distro Setup Stages (Examples):**
    *   `linux_setup:load_config_toml` (Inputs: TOML path)
    *   `linux_setup:generate_cloud_init_meta` (Inputs: `session.vm_definition`)
    *   `linux_setup:generate_cloud_init_user` (Inputs: User config from TOML)
    *   `linux_setup:assemble_cloud_init_iso` (Outputs: Path to cloud-init ISO)
    *   `linux_setup:prepare_install_scripts` (Optional)

*   **Dependencies:**
    *   All `qemu_setup:*` stages depend on `env_check:finalize_host_report`.
    *   `qemu_setup:validate_vm_request` also depends on `qemu_setup:load_vm_config_toml`.
    *   `qemu_setup:configure_vfio` depends on `env_check:check_iommu` and `env_check:gather_gpu_info` (implicitly via `session.environment_data`).
    *   All `linux_setup:*` stages depend on `qemu_setup:finalize_vm_definition`. Some `linux_setup` stages might also depend on `linux_setup:load_config_toml`.

## 4. Data Flow

1.  **Environment Check** runs, gathers host data, and stores it (e.g., in `session.environment_data`).
2.  **QEMU/KVM Setup** runs:
    *   Loads user's desired config from TOML (`session.user_vm_request`).
    *   Reads `session.environment_data`.
    *   Validates request against host capabilities.
    *   Defines the VM hardware based on validated request.
    *   Stores the final VM definition (e.g., in `session.vm_definition`).
    *   Optionally generates libvirt XML.
3.  **Linux Distro Setup** runs:
    *   Reads `session.vm_definition` (for virtual hardware context).
    *   Reads its specific configuration from TOML.
    *   Generates cloud-init data or other setup artifacts based on both inputs.
    *   Outputs the path(s) to the generated artifacts (e.g., cloud-init ISO).

## 5. Plugin Granularity Considerations

The plugins described (Environment Check, QEMU/KVM Setup, Linux Distro Setup) represent logical units. They could be further broken down as discussed previously, adhering strictly to the "everything is a plugin" model:

*   **Environment Check:** Could split into `Plugin: Host CPU Info`, `Plugin: Host GPU Info`, `Plugin: Host IOMMU Check`, etc.
*   **QEMU/KVM Setup:** Could split into `Plugin: VM CPU/RAM Definer`, `Plugin: VM Storage Definer`, `Plugin: VFIO Configurator`, `Plugin: Libvirt XML Generator`, etc.
*   **Linux Distro Setup:** Could split into `Plugin: CloudInit Generator`, `Plugin: Hostname Setter`, `Plugin: Install Script Prep`, etc.

The trade-offs remain: increased modularity vs. potentially increased complexity in managing dependencies and data flow between many small plugins. Starting with the core logical plugins (Environment Check, QEMU/KVM Setup, and initial guest-specific ones like Linux Distro Setup) and designing the data contracts (`session.*` objects) well allows for future refactoring into more granular plugins if desired.

## 6. Integration with Core Concepts

### Dry Run Mode
The entire workflow must respect the kernel's dry run mode flag (`Kernel::is_dry_run()`).
*   **Stage Implementation:** Each stage function within the plugins must check this flag.
*   **Behavior:**
    *   In dry run mode, stages should perform all calculations, validations, and data transformations as usual.
    *   They **must not** perform actions with persistent side effects on the host system (e.g., creating/modifying files like disk images, loading/unloading kernel modules, changing system configurations, writing libvirt XML unless explicitly requested for dry-run output).
    *   They should log or report (via UI Bridge) the actions they *would* have taken.
    *   The final output data (e.g., `session.vm_definition`, path to *simulated* EFI location) should still be generated to allow subsequent dry run stages to function.

### UI Integration (CLI/TUI) - Direct Communication
This section describes the **primary mechanism** for workflow stages to communicate directly with the active user interface (CLI, TUI, etc.) via the `UIManager` and `UIBridge`.
*   **Purpose:** To provide real-time status, progress, errors, and results from the executing stages directly to the user.
*   **Mechanism:** Stages access the `UIManager` (likely via `StageContext` or dependency injection) and use `UIManager::send_message()` with appropriate `UiMessage` types.
*   **Progress & Status:** Stages send `UiMessage::Info` for general status text. For potentially long-running operations (e.g., assembling EFI, configuring VFIO), stages **must** use `UiMessage::ProgressStart`, `UiMessage::ProgressUpdate`, and `UiMessage::ProgressComplete` to provide granular feedback.
*   **Validation & Errors:** Configuration validation errors (e.g., invalid TOML settings, host incompatibilities) identified during stages are reported directly using `UiMessage::Error`.
*   **Results:** Final outcomes (e.g., path to generated files) are reported using `UiMessage::Info` or a dedicated success message type.

### Event System Integration - Decoupled Notifications
Plugins within this workflow can *also* dispatch `Event` types via the `EventManager` at key milestones. This mechanism is primarily intended for **decoupled, asynchronous notification between core components**, not for direct, real-time UI updates during the workflow execution.
*   **Purpose:** To allow other system parts (e.g., logging plugins, monitoring tools, **long-running services like a Discord RPC plugin**, future automation plugins) to react to significant milestones without being directly coupled to the VM setup stages.
*   **Example Use Case (Discord RPC):** A Discord RPC plugin would register handlers for events like `VMDefinitionFinalized` or `GuestSpecificSetupCompleted`. When a VM setup stage dispatches such an event, the `EventManager` notifies the Discord plugin's handler asynchronously, allowing it to update the user's Discord status based on the workflow progress.
*   **Mechanism:** Stages access the `EventManager` (likely via `StageContext`) and dispatch specific `Event` structs.
*   **Example Events:** (These signal completion *after* the relevant work is done)
    *   `EnvironmentCheckCompleted { environment_data: ... }`
    *   `VMDefinitionFinalized { vm_definition: ... }`
    *   `LibvirtXMLGenerated { path: ... }`
    *   `GuestSpecificSetupCompleted { guest_type: "linux", output_path: ... }` (e.g., after cloud-init ISO generation)
*   **UI Interaction:** While a UI *could* be designed to listen for these system events, it's generally more appropriate for it to receive direct updates via `UiMessage` from the stages as they execute. Relying solely on events for UI updates would make the UI less responsive to the ongoing process.

### Stage Hook Interaction - Cross-Cutting Concerns
Stage Hooks provide a synchronous way for other components (like core logging or potentially UI plugins) to inject logic *immediately before and after* each stage in the VM setup pipeline executes.
*   **Purpose:** To handle cross-cutting concerns tied directly to stage execution boundaries.
*   **Mechanism:** Other plugins register `StageHook` implementations with the `StageManager`.
*   **Use Cases:**
    *   Detailed logging of stage entry/exit, duration, and context changes.
    *   Updating UI elements that reflect the *currently running* stage (distinct from progress messages sent *by* the stage via `UiMessage`).
    *   Performance monitoring or resource tracking per stage.
    *   Transactional operations or checkpointing around stages.
### Recovery Points / Checkpointing
The ability to resume a failed workflow requires creating checkpoints or recovery points at key intervals. The core architecture facilitates this primarily through Stage Hooks:
*   **Mechanism:** A dedicated Recovery/Checkpoint plugin (not part of the core VM setup plugins themselves) would register a post-stage hook (`StageManager::register_post_stage_hook`) to run after critical stages complete successfully.
*   **Implementation:** Inside the hook, the Recovery plugin would:
    1.  Access the `StageContext` to retrieve the current state (e.g., `session.environment_data`, `session.vm_definition`, paths to generated artifacts like cloud-init ISOs or libvirt XML).
    2.  Use the `StorageManager` to serialize and save this state, along with an indicator of the last successfully completed stage, to a designated recovery location (e.g., within `StorageManager::user_data_path()`).
*   **Resumption:** On a subsequent run, the application bootstrap or an early stage could check for recovery data. If found, it could potentially restore the `StageContext` and instruct the `StageManager` to resume the pipeline from the stage *after* the last checkpointed one.
*   **Responsibility:** The VM setup plugins (Environment Check, QEMU/KVM Setup, Linux Distro Setup) are responsible for performing their tasks and updating the `StageContext` correctly. The separate Recovery/Checkpoint plugin is responsible for observing the pipeline via hooks and persisting the state.