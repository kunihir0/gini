// Import necessary items based on the Plugin trait definition and core exports
use async_trait::async_trait;
use gini_core::kernel::{
    bootstrap::Application,
    error::Result as KernelResult, // Use kernel's Result
                                   // KernelError will be used via its full path if needed, or aliased if many uses.
};
use gini_core::plugin_system::{
    dependency::PluginDependency,
    // plugin_impl, // Keep commented out until location is confirmed
    error::PluginSystemError,     // Import PluginSystemError
    traits::{Plugin, PluginPriority}, // Import Plugin, PluginPriority
    version::VersionRange,        // Import VersionRange
};
use gini_core::stage_manager::{
    context::StageContext,       // Import StageContext
    requirement::StageRequirement, // Import StageRequirement
    registry::StageRegistry,     // Import StageRegistry
    pipeline::PipelineDefinition, // Import PipelineDefinition
    Stage,                       // Import Stage trait (defined in stage_manager/mod.rs)
};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind}; // Added ErrorKind
use std::path::Path; // Added Path, removed unused PathBuf
// use std::future::Future; // Removed unused import
// use std::pin::Pin; // Removed unused import
use std::str::FromStr; // For parsing VersionRange
use std::error::Error as StdError; // For boxing

// --- Data Structures ---

#[derive(Serialize, Deserialize, Debug, Clone, Default)] // Added Default
pub struct OsInfo {
    pub id: Option<String>,          // e.g., "ubuntu", "fedora"
    pub name: Option<String>,        // e.g., "Ubuntu", "Fedora Linux"
    pub version_id: Option<String>,  // e.g., "22.04"
    pub pretty_name: Option<String>, // e.g., "Ubuntu 22.04.3 LTS"
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CpuInfo {
    pub vendor_id: Option<String>,
    pub brand: Option<String>, // Often called "model name" in /proc/cpuinfo
    pub physical_core_count: Option<usize>, // May be harder to get reliably from /proc/cpuinfo
    pub logical_core_count: usize, // Count of "processor" entries
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RamInfo {
    pub total_kb: Option<u64>,
    pub available_kb: Option<u64>, // "MemAvailable" is generally preferred over "MemFree"
                                   // Add other fields like MemFree, Buffers, Cached if needed
}


#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GpuInfo {
    pub pci_id: String, // The directory name, e.g., "0000:01:00.0"
    pub vendor_id: Option<String>,
    pub device_id: Option<String>,
    pub class_code: Option<String>, // Store the full class code read
    pub audio_pci_id: Option<String>, // Associated HD Audio device, e.g., "0000:01:00.1"
                                    // Optional fields to add later if needed/possible:
                                    // pub driver: Option<String>,
                                    // pub iommu_group: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct IommuInfo {
    pub enabled: bool,
    // Map of Group ID (String) to Vec of PCI IDs (String) in that group
    pub groups: HashMap<String, Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ZramDeviceInfo {
    pub device_name: String, // e.g., zram0
    pub disk_size_bytes: Option<u64>,
    pub compression_algo: Option<String>,
    pub original_data_size_bytes: Option<u64>,
    pub compressed_data_size_bytes: Option<u64>,
    pub mem_used_total_bytes: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SwapInfo {
    pub total_swap_kb: Option<u64>,
    pub free_swap_kb: Option<u64>,
    pub swappiness: Option<u8>,
    pub zram_devices: Vec<ZramDeviceInfo>,
    pub active_swap_files: Vec<String>, // For file-based swap
    pub active_swap_partitions: Vec<String>, // For partition-based swap
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PhysicalVolumeInfo {
    pub pv_name: String, // e.g., /dev/sda1
    pub vg_name: Option<String>,
    pub pv_size_bytes: Option<u64>, // Might be hard to get accurately without LVM tools
    pub pv_free_bytes: Option<u64>, // Might be hard to get accurately without LVM tools
    pub pv_uuid: Option<String>,    // Might be hard to get accurately without LVM tools
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct VolumeGroupInfo {
    pub vg_name: String,
    pub pv_count: usize, // Count of PVs associated (best effort)
    pub lv_count: usize, // Count of LVs associated (best effort)
    pub vg_size_bytes: Option<u64>, // Might be hard to get accurately without LVM tools
    pub vg_free_bytes: Option<u64>, // Might be hard to get accurately without LVM tools
    pub vg_uuid: Option<String>,    // Might be hard to get accurately without LVM tools
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LvmCacheInfo {
    pub cache_pool_lv: String,
    pub cache_mode: String, // e.g., writethrough, writeback
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LvmRaidInfo {
    pub raid_type: String, // e.g., raid1, raid5
    pub sync_percentage: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LogicalVolumeInfo {
    pub lv_name: String, // The LV name part, e.g., homevol
    pub vg_name: String, // The VG name part, e.g., MyVolGroup
    pub lv_path: String, // Full path, e.g., /dev/MyVolGroup/homevol or /dev/mapper/MyVolGroup-homevol
    pub lv_size_bytes: Option<u64>, // From /sys/class/block/dm-X/size (sectors * 512)
    pub lv_uuid: Option<String>,    // Might be hard to get accurately without LVM tools
    pub filesystem_type: Option<String>, // Best effort from /proc/mounts
    pub mount_point: Option<String>,     // From /proc/mounts
    pub is_thin_pool: bool,       // Difficult to determine reliably without LVM tools
    pub is_thin_volume: bool,     // Difficult to determine reliably without LVM tools
    pub is_snapshot: bool,        // Difficult to determine reliably without LVM tools
    pub cache_info: Option<LvmCacheInfo>, // Difficult
    pub raid_info: Option<LvmRaidInfo>,   // Difficult
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LvmInfo {
    pub physical_volumes: Vec<PhysicalVolumeInfo>, // Likely to be sparsely populated
    pub volume_groups: Vec<VolumeGroupInfo>,       // Likely to be sparsely populated
    pub logical_volumes: Vec<LogicalVolumeInfo>,   // Main focus for file-based check
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BridgeInfo {
    pub bridge_name: String,
    pub interfaces: Vec<String>,
    pub mac_address: Option<String>,
    // pub ip_addresses: Vec<String>, // Hard without `ip` command or netlink
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TapDeviceInfo {
    pub tap_name: String,
    pub mac_address: Option<String>,
    pub owner_uid: Option<u32>,
    pub flags: Option<String>, // Raw flags string for now
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NetworkVirtInfo {
    pub ip_forwarding_enabled: Option<bool>,
    pub bridges: Vec<BridgeInfo>,
    pub tap_devices: Vec<TapDeviceInfo>,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PackageManagerInfo {
    pub identified_manager: Option<String>, // "dpkg/apt", "rpm/dnf/yum", "pacman", "unknown"
    pub detection_method: Option<String>, // e.g., "/var/lib/dpkg/status found"
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PackageCheckInfo {
    pub package_name: String,
    pub is_installed: Option<bool>, // true, false, or None if status unknown
    pub check_method: String, // How was it checked? e.g. "dpkg status file", "binary path"
    pub version: Option<String>, // If obtainable
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SystemPackagesInfo {
    pub package_manager: PackageManagerInfo,
    pub checked_packages: Vec<PackageCheckInfo>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KernelVirtualizationParamsInfo {
    pub raw_cmdline: String,
    pub iommu_status: String,
    pub iommu_verification_method: String,
    pub nested_virtualization_status: String,
    pub nested_virtualization_verification_method: String,
    pub other_relevant_params: std::collections::HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BootloaderScanInfo {
    pub identified_bootloader: Option<String>,
    pub config_files_checked: Vec<String>,
    pub kernel_params_in_config: Option<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
}


// --- Plugin Implementation ---

#[derive(Default)]
#[allow(dead_code)] // This struct is instantiated by the plugin loader
pub struct EnvironmentCheckPlugin;

#[async_trait]
impl Plugin for EnvironmentCheckPlugin {
    fn name(&self) -> &'static str {
        "core-environment-check"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        const COMPATIBLE_API_REQ: &str = "^0.1"; // Adjust if core API is different
        match VersionRange::from_str(COMPATIBLE_API_REQ) {
            Ok(vr) => vec![vr],
            Err(e) => {
                log::error!(
                    "Failed to parse API version requirement ('{}'): {}",
                    COMPATIBLE_API_REQ,
                    e
                );
                vec![]
            }
        }
    }

    fn init(&self, _app: &mut Application) -> Result<(), PluginSystemError> {
        info!("Initializing Core Environment Check Plugin v{}", self.version());
        Ok(())
    }

    fn is_core(&self) -> bool {
        true
    }

    fn priority(&self) -> PluginPriority {
        PluginPriority::Core(51)
    }

    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![
            PluginDependency {
                plugin_name: "core-logging".to_string(),
                version_range: Some(VersionRange::from_str("^0.1.0").expect("Failed to parse version range for core-logging dependency")),
                required: true, // Assuming core-logging is a required dependency
            }
        ]
    }

    fn required_stages(&self) -> Vec<StageRequirement> {
        vec![]
    }

    fn conflicts_with(&self) -> Vec<String> {
        vec![]
    }

    fn incompatible_with(&self) -> Vec<PluginDependency> {
        vec![]
    }

    async fn preflight_check(&self, _context: &StageContext) -> Result<(), PluginSystemError> {
        Ok(())
    }

    // Use PluginSystemError as the return type
    fn register_stages(&self, registry: &mut StageRegistry) -> Result<(), PluginSystemError> {
        info!("Registering stages for {}", self.name());
        // Register the stages defined in this plugin
        registry.register_stage(Box::new(GatherOsInfoStage)).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
        registry.register_stage(Box::new(GatherCpuInfoStage)).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
        registry.register_stage(Box::new(GatherRamInfoStage)).map_err(|e| PluginSystemError::InternalError(e.to_string()))?; // Register the new RAM stage
        registry.register_stage(Box::new(GatherGpuInfoStage)).map_err(|e| PluginSystemError::InternalError(e.to_string()))?; // Register the new GPU stage
        registry.register_stage(Box::new(CheckIommuStage)).map_err(|e| PluginSystemError::InternalError(e.to_string()))?; // Register the new IOMMU stage
        registry.register_stage(Box::new(VirtualizationKernelParamsStage)).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
        registry.register_stage(Box::new(CheckSwapStage)).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
        registry.register_stage(Box::new(CheckLvmStage)).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
        registry.register_stage(Box::new(CheckNetworkVirtStage)).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;
        registry.register_stage(Box::new(CheckSystemPackagesStage)).map_err(|e| PluginSystemError::InternalError(e.to_string()))?;

        // Define and register the startup pipeline
        const STARTUP_PIPELINE_STAGES: &[&'static str] = &[
            "env_check:gather_os_info",
            "env_check:gather_cpu_info",
            "env_check:gather_ram_info",
            "env_check:gather_gpu_info",
            "env_check:check_iommu",
            "env_check:virtualization_kernel_params",
            "env_check:check_swap",
            "env_check:check_lvm",
            "env_check:check_network_virt",
            "env_check:check_system_packages",
        ];

        let startup_pipeline_def = PipelineDefinition {
            name: "startup_environment_check",
            stages: STARTUP_PIPELINE_STAGES,
            description: Some("Core environment checks provided by the core-environment-check plugin."),
        };

        registry.register_pipeline(startup_pipeline_def)
            .map_err(|e| PluginSystemError::InternalError(format!("Failed to register startup pipeline: {}", e)))?;

        Ok(())
    }

    // startup_check_stages is removed as the plugin now registers its pipeline directly.
    // fn startup_check_stages(&self) -> Vec<String> {
    //     vec![]
    // }

    fn shutdown(&self) -> Result<(), PluginSystemError> {
        info!("Shutting down Core Environment Check Plugin");
        Ok(())
    }
}

// --- Concrete Stage Implementation ---

#[allow(dead_code)] // This struct is instantiated by the plugin loader
struct GatherOsInfoStage;

#[async_trait]
impl Stage for GatherOsInfoStage {
    fn id(&self) -> &str {
        "env_check:gather_os_info"
    }

    fn name(&self) -> &str {
        "Gather OS Info"
    }

    fn description(&self) -> &str {
        "Gathers OS and distribution information from /etc/os-release."
    }

    // Implement the async execute method
    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        // Call the wrapper function that uses the default path
        gather_os_info_stage_wrapper(context).await.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync + 'static>)
    }
}

#[allow(dead_code)] // This struct is instantiated by the plugin loader
struct GatherCpuInfoStage;

#[async_trait]
impl Stage for GatherCpuInfoStage {
    fn id(&self) -> &str {
        "env_check:gather_cpu_info"
    }

    fn name(&self) -> &str {
        "Gather CPU Info"
    }

    fn description(&self) -> &str {
        "Gathers CPU vendor, brand, and core count from /proc/cpuinfo."
    }

    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        gather_cpu_info_stage(context).await.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync + 'static>)
    }
}

#[allow(dead_code)] // This struct is instantiated by the plugin loader
struct GatherRamInfoStage;

#[async_trait]
impl Stage for GatherRamInfoStage {
    fn id(&self) -> &str {
        "env_check:gather_ram_info"
    }

    fn name(&self) -> &str {
        "Gather RAM Info"
    }

    fn description(&self) -> &str {
        "Gathers RAM total and available memory from /proc/meminfo."
    }

    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        gather_ram_info_stage(context).await.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync + 'static>)
    }
}

#[allow(dead_code)] // This struct is instantiated by the plugin loader
struct GatherGpuInfoStage;

#[async_trait]
impl Stage for GatherGpuInfoStage {
    fn id(&self) -> &str {
        "env_check:gather_gpu_info"
    }

    fn name(&self) -> &str {
        "Gather GPU Info"
    }

    fn description(&self) -> &str {
        "Gathers GPU information by parsing /sys/bus/pci/devices/."
    }

    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        gather_gpu_info_stage(context).await.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync + 'static>)
    }
}

#[allow(dead_code)] // This struct is instantiated by the plugin loader
struct CheckIommuStage;

#[async_trait]
impl Stage for CheckIommuStage {
    fn id(&self) -> &str {
        "env_check:check_iommu"
    }

    fn name(&self) -> &str {
        "Check IOMMU Status"
    }

    fn description(&self) -> &str {
        "Checks IOMMU status via /proc/cmdline and /sys/class/iommu."
    }

    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        check_iommu_stage(context).await.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync + 'static>)
    }
}

#[allow(dead_code)] // This struct is instantiated by the plugin loader
struct VirtualizationKernelParamsStage;

#[async_trait]
impl Stage for VirtualizationKernelParamsStage {
    fn id(&self) -> &str {
        "env_check:virtualization_kernel_params"
    }

    fn name(&self) -> &str {
        "Virtualization Kernel & Bootloader Parameter Check"
    }

    fn description(&self) -> &str {
        "Checks kernel parameters and bootloader settings for virtualization based on core-env documentation."
    }

    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        // Call helper functions
        check_kernel_virtualization_params(context).await.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync + 'static>)?;
        scan_bootloader_configuration(context).await.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync + 'static>)
    }
}

#[allow(dead_code)] // This struct is instantiated by the plugin loader
struct CheckSwapStage;

#[async_trait]
impl Stage for CheckSwapStage {
    fn id(&self) -> &str { "env_check:check_swap" }
    fn name(&self) -> &str { "Check Swap Configuration (ZRAM, Swappiness)" }
    fn description(&self) -> &str { "Checks swap usage, ZRAM devices, and swappiness." }
    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        check_swap_config(context).await.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync + 'static>)
    }
}

#[allow(dead_code)] // This struct is instantiated by the plugin loader
struct CheckLvmStage;

#[async_trait]
impl Stage for CheckLvmStage {
    fn id(&self) -> &str { "env_check:check_lvm" }
    fn name(&self) -> &str { "Check LVM Configuration" }
    fn description(&self) -> &str { "Performs a best-effort check of LVM setup using /dev/mapper and /sys." }
    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        check_lvm_config(context).await.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync + 'static>)
    }
}

#[allow(dead_code)] // This struct is instantiated by the plugin loader
struct CheckNetworkVirtStage;

#[async_trait]
impl Stage for CheckNetworkVirtStage {
    fn id(&self) -> &str { "env_check:check_network_virt" }
    fn name(&self) -> &str { "Check Network Virtualization Settings" }
    fn description(&self) -> &str { "Checks IP forwarding, bridges, and TAP devices." }
    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        check_network_virt_config(context).await.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync + 'static>)
    }
}

// Added missing stage definition
#[allow(dead_code)] // This struct is instantiated by the plugin loader
struct CheckSystemPackagesStage;

#[async_trait]
impl Stage for CheckSystemPackagesStage {
    fn id(&self) -> &str { "env_check:check_system_packages" }
    fn name(&self) -> &str { "Check System Packages" }
    fn description(&self) -> &str { "Identifies package manager and checks for essential virtualization packages." }
    async fn execute(&self, context: &mut StageContext) -> std::result::Result<(), Box<dyn StdError + Send + Sync + 'static>> {
        check_system_packages(context).await.map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync + 'static>)
    }
}


// --- Stage Function Logic ---

// Wrapper function called by the Stage trait, uses the default path
#[allow(dead_code)] // This function is called by the Stage trait implementation
async fn gather_os_info_stage_wrapper(ctx: &mut StageContext) -> KernelResult<()> {
    use std::path::Path;
    const OS_RELEASE_PATH: &str = "/etc/os-release";
    gather_os_info_from_file(ctx, Path::new(OS_RELEASE_PATH)).await
}

/// Gathers OS and distribution information from a specified file path.
#[allow(dead_code)] // This function is called by gather_os_info_stage_wrapper
async fn gather_os_info_from_file(ctx: &mut StageContext, file_path: &std::path::Path) -> KernelResult<()> {
    info!("Stage: Gathering OS/Distribution info from {}...", file_path.display());
    const OS_INFO_KEY: &str = "env_check:os_info";
    let mut os_info = OsInfo::default(); // Start with default

    match fs::File::open(file_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut vars = HashMap::new();

            for line_result in reader.lines() {
                match line_result {
                    Ok(line) => {
                        if line.trim().is_empty() || line.starts_with('#') {
                            continue;
                        }
                        if let Some((key, value)) = line.split_once('=') {
                            // Trim quotes from value if present
                            let value = value.trim().trim_matches('"').to_string();
                            vars.insert(key.trim().to_string(), value);
                        } else {
                            log::warn!("Ignoring malformed line in {}: {}", file_path.display(), line);
                        }
                    }
                    Err(e) => {
                        // Log error reading line but continue
                        log::warn!("Failed to read line from {}: {}", file_path.display(), e);
                    }
                }
            }

            os_info = OsInfo {
                id: vars.get("ID").cloned(),
                name: vars.get("NAME").cloned(),
                version_id: vars.get("VERSION_ID").cloned(),
                pretty_name: vars.get("PRETTY_NAME").cloned(),
            };
        }
        Err(e) => {
            // Log error opening file but proceed with default OsInfo
            // Depending on requirements, this could be a hard error by returning Err(...)
            log::warn!(
                "Could not open {}: {}. Proceeding with default OS info.",
                file_path.display(),
                e
            );
            // No need to return Err here based on instructions to proceed
        }
    }

    info!("Detected OS: {:?}", os_info);
    // Store the potentially default OsInfo.
    // Note: The original ctx.set_data signature doesn't return Result,
    // but the instructions example included '?', implying it might.
    // Assuming the current signature is correct and doesn't return Result.
    // If it *did* return Result<_, KernelError>, we'd use:
    // ctx.set_data(OS_INFO_KEY, os_info).map_err(|e| KernelError::Stage(format!("Failed to set OS info in context: {}", e)))?;
    ctx.set_data(OS_INFO_KEY, os_info);

    Ok(())
}

/// Gathers CPU information by parsing /proc/cpuinfo.
#[allow(dead_code)] // This function is called by the Stage trait implementation
async fn gather_cpu_info_stage(ctx: &mut StageContext) -> KernelResult<()> {
    use log::warn; // Ensure warn is in scope

    info!("Stage: Gathering CPU info from /proc/cpuinfo...");
    const CPUINFO_PATH: &str = "/proc/cpuinfo";
    const CPU_INFO_KEY: &str = "env_check:cpu_info";
    let mut cpu_info = CpuInfo::default();
    let mut processor_count = 0;
    let mut physical_cores_per_socket: Option<usize> = None; // Tracks 'cpu cores' value

    match fs::File::open(CPUINFO_PATH) {
        Ok(file) => {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                match line {
                    Ok(line_content) => {
                        if let Some((key_raw, value_raw)) = line_content.split_once(':') {
                            let key = key_raw.trim();
                            let value = value_raw.trim();

                            match key {
                                "processor" => processor_count += 1,
                                "vendor_id" if cpu_info.vendor_id.is_none() => {
                                    cpu_info.vendor_id = Some(value.to_string());
                                }
                                "model name" if cpu_info.brand.is_none() => {
                                    cpu_info.brand = Some(value.to_string());
                                }
                                // Note: 'cpu cores' usually refers to cores per physical package/socket.
                                // If there are multiple sockets, this might not be the total physical core count.
                                // A more robust solution might involve counting unique physical IDs and core IDs,
                                // but that adds complexity. We'll stick to the simpler approach for now.
                                "cpu cores" if physical_cores_per_socket.is_none() => {
                                    match value.parse::<usize>() {
                                        Ok(cores) => physical_cores_per_socket = Some(cores),
                                        Err(e) => warn!("Failed to parse 'cpu cores' value '{}': {}", value, e),
                                    }
                                }
                                _ => {} // Ignore other keys
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error reading line from {}: {}", CPUINFO_PATH, e);
                        // Decide whether to continue or bail out. Continuing seems reasonable.
                    }
                }
            }
            // Assign logical cores (total processors listed)
            cpu_info.logical_core_count = processor_count;
            // Assign physical cores based on the first 'cpu cores' value found.
            // This is an approximation, especially for multi-socket systems.
            cpu_info.physical_core_count = physical_cores_per_socket;

        }
        Err(e) => {
            warn!("Could not open {}: {}. Proceeding without CPU info.", CPUINFO_PATH, e);
            // Proceed with default cpu_info (already initialized)
        }
    }

    info!("Detected CPU (from /proc/cpuinfo): {:?}", cpu_info);
    // Store the gathered (or default) CpuInfo.
    // Assuming set_data doesn't return Result based on previous OsInfo implementation.
    ctx.set_data(CPU_INFO_KEY, cpu_info);
    // If set_data returned Result:
    // ctx.set_data(CPU_INFO_KEY, cpu_info).map_err(|e| KernelError::Stage(format!("Failed to set CPU info in context: {}", e)))?;


    Ok(())
}

/// Gathers RAM information by parsing /proc/meminfo.
#[allow(dead_code)] // This function is called by the Stage trait implementation
async fn gather_ram_info_stage(ctx: &mut StageContext) -> KernelResult<()> {
    use log::{info, warn}; // Ensure info and warn are in scope

    info!("Stage: Gathering RAM info from /proc/meminfo...");
    const MEMINFO_PATH: &str = "/proc/meminfo";
    const RAM_INFO_KEY: &str = "env_check:ram_info";
    let mut ram_info = RamInfo::default();

    match fs::File::open(MEMINFO_PATH) {
        Ok(file) => {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                match line {
                    Ok(line_content) => {
                        if let Some((key_raw, value_raw)) = line_content.split_once(':') {
                            let key = key_raw.trim();
                            // Extract the numeric part, ignoring " kB" suffix
                            let value_part = value_raw.trim().split_whitespace().next().unwrap_or("");

                            match key {
                                "MemTotal" if ram_info.total_kb.is_none() => {
                                    match value_part.parse::<u64>() {
                                        Ok(val) => ram_info.total_kb = Some(val),
                                        Err(e) => warn!("Failed to parse MemTotal value '{}': {}", value_part, e),
                                    }
                                }
                                "MemAvailable" if ram_info.available_kb.is_none() => {
                                    match value_part.parse::<u64>() {
                                        Ok(val) => ram_info.available_kb = Some(val),
                                        Err(e) => warn!("Failed to parse MemAvailable value '{}': {}", value_part, e),
                                    }
                                }
                                _ => {} // Ignore other keys
                            }

                            // Stop early if we found both needed values
                            if ram_info.total_kb.is_some() && ram_info.available_kb.is_some() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error reading line from {}: {}", MEMINFO_PATH, e);
                        // Decide whether to continue or bail out. Continuing seems reasonable.
                    }
                }
            }
        }
        Err(e) => {
            warn!("Could not open {}: {}. Proceeding without RAM info.", MEMINFO_PATH, e);
            // Proceed with default ram_info (already initialized)
        }
    }

    info!("Detected RAM (from /proc/meminfo): {:?}", ram_info);
    // Store the gathered (or default) RamInfo.
    ctx.set_data(RAM_INFO_KEY, ram_info);
    // If set_data returned Result:
    // ctx.set_data(RAM_INFO_KEY, ram_info).map_err(|e| KernelError::Stage(format!("Failed to set RAM info in context: {}", e)))?;

    Ok(())
}

/// Extracts the base PCI address (domain:bus:device) from a full PCI ID (domain:bus:device.function).
fn get_base_pci_address(pci_id: &str) -> Option<String> {
    pci_id.rsplit_once('.').map(|(base, _)| base.to_string())
}

/// Gathers GPU information and associated HD Audio devices by parsing /sys/bus/pci/devices/.
#[allow(dead_code)] // This function is called by the Stage trait implementation
async fn gather_gpu_info_stage(ctx: &mut StageContext) -> KernelResult<()> {
    use log::{info, warn, error}; // Ensure logging macros are in scope

    info!("Stage: Gathering GPU and associated Audio info from /sys/bus/pci/devices/...");
    const PCI_DEVICES_PATH: &str = "/sys/bus/pci/devices";
    const GPU_INFO_KEY: &str = "env_check:gpu_info";
    const VGA_CLASS_PREFIX: &str = "0x0300"; // Display controller, VGA compatible
    const AUDIO_CLASS_CODE: &str = "0x040300"; // Audio device, High Definition Audio

    let mut potential_gpus: Vec<GpuInfo> = Vec::new();
    let mut audio_devices: HashMap<String, String> = HashMap::new(); // Map base PCI addr -> full audio PCI ID
    let pci_path = Path::new(PCI_DEVICES_PATH);

    match fs::read_dir(pci_path) {
        Ok(entries) => {
            for entry_result in entries {
                match entry_result {
                    Ok(entry) => {
                        let device_path = entry.path();
                        if !device_path.is_dir() {
                            continue;
                        }

                        let pci_id = match device_path.file_name() {
                            Some(name) => name.to_string_lossy().to_string(),
                            None => {
                                warn!("Could not get PCI device directory name for path: {}", device_path.display());
                                continue;
                            }
                        };

                        let class_path = device_path.join("class");
                        match fs::read_to_string(&class_path) {
                            Ok(class_content) => {
                                let class_code_full = class_content.trim();

                                // Check for GPU (VGA Controller)
                                if class_code_full.starts_with(VGA_CLASS_PREFIX) {
                                    info!("Found potential GPU: {} (Class: {})", pci_id, class_code_full);
                                    let mut gpu_info = GpuInfo {
                                        pci_id: pci_id.clone(),
                                        class_code: Some(class_code_full.to_string()),
                                        ..Default::default()
                                    };

                                    // Read vendor ID
                                    let vendor_path = device_path.join("vendor");
                                    match fs::read_to_string(&vendor_path) {
                                        Ok(vendor_id) => gpu_info.vendor_id = Some(vendor_id.trim().to_string()),
                                        Err(e) => warn!("Could not read vendor ID for {}: {}", pci_id, e),
                                    }

                                    // Read device ID
                                    let device_id_path = device_path.join("device");
                                    match fs::read_to_string(&device_id_path) {
                                        Ok(device_id) => gpu_info.device_id = Some(device_id.trim().to_string()),
                                        Err(e) => warn!("Could not read device ID for {}: {}", pci_id, e),
                                    }
                                    potential_gpus.push(gpu_info);

                                // Check for HD Audio Controller
                                } else if class_code_full == AUDIO_CLASS_CODE {
                                    if let Some(base_addr) = get_base_pci_address(&pci_id) {
                                        info!("Found potential HD Audio device: {} (Base: {})", pci_id, base_addr);
                                        // Store the full audio PCI ID, keyed by the base address
                                        audio_devices.insert(base_addr, pci_id.clone());
                                    } else {
                                        warn!("Could not extract base address from audio device PCI ID: {}", pci_id);
                                    }
                                }
                            }
                            Err(e) => {
                                // Only warn if reading fails unexpectedly (not for NotFound)
                                if e.kind() != ErrorKind::NotFound {
                                    warn!("Could not read class file for {}: {}", pci_id, e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error reading entry in {}: {}", PCI_DEVICES_PATH, e);
                        // Continue processing other devices
                    }
                }
            }
        }
        Err(e) => {
            error!("Could not read PCI devices directory {}: {}. GPU info will be unavailable.", PCI_DEVICES_PATH, e);
            // Proceed with an empty list, but log the error.
            ctx.set_data(GPU_INFO_KEY, Vec::<GpuInfo>::new());
            return Ok(()); // Return early as we cannot proceed
        }
    }

    // Link GPUs to their audio devices
    let mut final_gpu_list: Vec<GpuInfo> = Vec::new();
    for mut gpu in potential_gpus {
        if let Some(base_addr) = get_base_pci_address(&gpu.pci_id) {
            if let Some(audio_id) = audio_devices.get(&base_addr) {
                info!("Linking GPU {} to Audio device {}", gpu.pci_id, audio_id);
                gpu.audio_pci_id = Some(audio_id.clone());
            }
        }
        final_gpu_list.push(gpu);
    }


    info!("Found {} GPU(s). Details: {:?}", final_gpu_list.len(), final_gpu_list);
    // Store the final list (potentially with linked audio devices).
    ctx.set_data(GPU_INFO_KEY, final_gpu_list);
    // If set_data returned Result:
    // ctx.set_data(GPU_INFO_KEY, final_gpu_list).map_err(|e| KernelError::Stage(format!("Failed to set GPU info in context: {}", e)))?;

    Ok(())
}

/// Checks IOMMU status by reading /proc/cmdline and /sys/class/iommu/.
#[allow(dead_code)] // Called dynamically via registry
async fn check_iommu_stage(ctx: &mut StageContext) -> KernelResult<()> {
    use log::{info, warn, error}; // Ensure logging macros are in scope

    info!("Stage: Checking IOMMU status...");
    const IOMMU_INFO_KEY: &str = "env_check:iommu_info";
    const CMDLINE_PATH: &str = "/proc/cmdline";
    const IOMMU_SYSFS_PATH: &str = "/sys/class/iommu";

    let mut iommu_info = IommuInfo::default(); // Initialize with enabled = false

    // 1. Check /proc/cmdline for IOMMU kernel parameters
    match fs::read_to_string(CMDLINE_PATH) {
        Ok(cmdline) => {
            if cmdline.contains("intel_iommu=on") || cmdline.contains("amd_iommu=on") {
                info!("IOMMU kernel parameter found in {}", CMDLINE_PATH);
                iommu_info.enabled = true;
            } else {
                info!("IOMMU kernel parameter not found in {}", CMDLINE_PATH);
            }
        }
        Err(e) => {
            warn!("Could not read {}: {}. Assuming IOMMU is disabled.", CMDLINE_PATH, e);
            // Keep iommu_info.enabled as false
        }
    }

    // 2. If enabled, parse /sys/class/iommu for groups and devices
    if iommu_info.enabled {
        let iommu_path = Path::new(IOMMU_SYSFS_PATH);
        match fs::read_dir(iommu_path) {
            Ok(entries) => {
                let mut groups_with_devices_count = 0; // Count groups actually added to the map
                let mut total_group_dirs_found = 0; // Count all valid group dirs found
                for entry_result in entries {
                    match entry_result {
                        Ok(entry) => {
                            let group_path = entry.path();
                            if !group_path.is_dir() {
                                continue;
                            }
                            let group_id = match group_path.file_name() {
                                Some(name) => name.to_string_lossy().to_string(),
                                None => {
                                    warn!("Could not get IOMMU group directory name for path: {}", group_path.display());
                                    continue;
                                }
                            };

                            // Removed strict numeric check for group_id. Process any directory found.
                            // if group_id.parse::<u32>().is_err() {
                            //     warn!("Skipping non-numeric IOMMU group directory: {}", group_id);
                            //     continue;
                            // }

                            // Increment total count for any valid group directory found
                            total_group_dirs_found += 1;

                            let devices_path = group_path.join("devices");
                            let mut device_ids: Vec<String> = Vec::new();

                            match fs::read_dir(&devices_path) {
                                Ok(device_entries) => {
                                    for device_entry_result in device_entries {
                                        match device_entry_result {
                                            Ok(device_entry) => {
                                                // The directory entry name is the PCI ID (symlink name)
                                                let pci_id = device_entry.file_name().to_string_lossy().to_string();
                                                device_ids.push(pci_id);
                                            }
                                            Err(e) => {
                                                warn!("Error reading device entry in {}: {}", devices_path.display(), e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    // Log error reading devices dir, but continue processing other groups
                                    warn!("Could not read devices directory {}: {}", devices_path.display(), e);
                                }
                            }

                            if !device_ids.is_empty() {
                                info!("Found IOMMU Group {} with devices: {:?}", group_id, device_ids);
                                iommu_info.groups.insert(group_id, device_ids);
                                groups_with_devices_count += 1;
                            } else {
                                info!("Found IOMMU Group {} (directory exists, but no devices listed or devices dir unreadable)", group_id);
                                // Optionally insert empty groups: iommu_info.groups.insert(group_id, device_ids);
                            }
                        }
                        Err(e) => {
                            error!("Error reading entry in {}: {}", IOMMU_SYSFS_PATH, e);
                        }
                    }
                }
                info!(
                    "IOMMU enabled. Found {} total IOMMU group directories. Populated {} group(s) with devices.",
                    total_group_dirs_found, groups_with_devices_count
                );
            }
            Err(e) => {
                // If /sys/class/iommu doesn't exist, it might mean IOMMU isn't *really* active
                // even if the kernel parameter is set (e.g., VT-d disabled in BIOS).
                if e.kind() == ErrorKind::NotFound {
                    warn!("IOMMU sysfs path {} not found, despite kernel parameter. IOMMU might not be active.", IOMMU_SYSFS_PATH);
                    // Consider setting iommu_info.enabled = false here? For now, just warn.
                } else {
                    error!("Could not read IOMMU directory {}: {}. Group information unavailable.", IOMMU_SYSFS_PATH, e);
                }
                // Proceed with potentially empty groups map
            }
        }
    } else {
        info!("IOMMU is disabled (based on kernel parameters).");
    }

    // Store the gathered IommuInfo.
    ctx.set_data(IOMMU_INFO_KEY, iommu_info);
    // If set_data returned Result:
    // ctx.set_data(IOMMU_INFO_KEY, iommu_info).map_err(|e| KernelError::Stage(format!("Failed to set IOMMU info in context: {}", e)))?;

    Ok(())
}

/// Checks kernel parameters relevant to virtualization.
#[allow(dead_code)] // Called by VirtualizationKernelParamsStage
async fn check_kernel_virtualization_params(ctx: &mut StageContext) -> KernelResult<()> {
    use log::{info, warn};
    info!("Stage: Checking Kernel Virtualization Parameters...");
    const KERNEL_VIRT_PARAMS_KEY: &str = "kernel_virtualization_params_info";
    const CMDLINE_PATH: &str = "/proc/cmdline";
    const KVM_INTEL_NESTED_PATH: &str = "/sys/module/kvm_intel/parameters/nested";
    const KVM_AMD_NESTED_PATH: &str = "/sys/module/kvm_amd/parameters/nested";
    const IOMMU_SYSFS_PATH: &str = "/sys/class/iommu";
    const MEMINFO_PATH: &str = "/proc/meminfo"; // For HugePages

    let mut params_info = KernelVirtualizationParamsInfo::default();

    // 1. Read /proc/cmdline
    match fs::read_to_string(CMDLINE_PATH) {
        Ok(cmdline) => {
            params_info.raw_cmdline = cmdline.trim().to_string();
            info!("Raw kernel cmdline: {}", params_info.raw_cmdline);

            // Check for IOMMU params in cmdline
            if params_info.raw_cmdline.contains("intel_iommu=on") {
                params_info.iommu_status = "Enabled (intel_iommu=on)".to_string();
                params_info.iommu_verification_method = "/proc/cmdline".to_string();
            } else if params_info.raw_cmdline.contains("amd_iommu=on") {
                params_info.iommu_status = "Enabled (amd_iommu=on)".to_string();
                params_info.iommu_verification_method = "/proc/cmdline".to_string();
            } else if params_info.raw_cmdline.contains("iommu=pt") {
                params_info.iommu_status = "Enabled (iommu=pt)".to_string();
                params_info.iommu_verification_method = "/proc/cmdline".to_string();
            }
            // Further IOMMU checks via /sys will refine this or confirm if cmdline is missing it
        }
        Err(e) => {
            warn!("Could not read {}: {}. Kernel cmdline parameters will be unavailable.", CMDLINE_PATH, e);
            params_info.raw_cmdline = "Error reading /proc/cmdline".to_string();
            // IOMMU status will be determined by /sys checks or remain "Error checking status"
        }
    }

    // 2. Verify IOMMU status via /sys/class/iommu (more definitive if active)
    let iommu_sys_path = Path::new(IOMMU_SYSFS_PATH);
    match fs::read_dir(iommu_sys_path) {
        Ok(entries) => {
            let mut iommu_dirs_found = false;
            for entry_result in entries {
                if let Ok(entry) = entry_result {
                    if entry.path().is_dir() {
                        // Check for dmar* or amd-iommu* or similar standard directory names
                        if let Some(dir_name_osstr) = entry.path().file_name() {
                            if let Some(dir_name) = dir_name_osstr.to_str() {
                                if dir_name.starts_with("dmar") || dir_name.contains("iommu") { // General check
                                    iommu_dirs_found = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            if iommu_dirs_found {
                if params_info.iommu_status.is_empty() || params_info.iommu_status.contains("Error") {
                    params_info.iommu_status = "Enabled (detected via /sys)".to_string();
                    params_info.iommu_verification_method = IOMMU_SYSFS_PATH.to_string();
                } else if !params_info.iommu_verification_method.contains("/sys") {
                    // Append to existing method if cmdline was also found
                    params_info.iommu_verification_method = format!("{}; {}", params_info.iommu_verification_method, IOMMU_SYSFS_PATH);
                }
                info!("IOMMU active based on presence of directories in {}", IOMMU_SYSFS_PATH);
            } else if params_info.iommu_status.is_empty() || params_info.iommu_status.contains("Error") {
                params_info.iommu_status = "Not Detected/Disabled".to_string();
                params_info.iommu_verification_method = if params_info.iommu_verification_method.is_empty() {
                    IOMMU_SYSFS_PATH.to_string()
                } else {
                    format!("{}; {}", params_info.iommu_verification_method, IOMMU_SYSFS_PATH)
                };
                info!("No IOMMU-specific directories (e.g., dmar*) found in {}. IOMMU likely not active or configured.", IOMMU_SYSFS_PATH);
            }
        }
        Err(e) => {
            warn!("Could not read IOMMU sysfs directory {}: {}. Status might be inaccurate.", IOMMU_SYSFS_PATH, e);
            if params_info.iommu_status.is_empty() {
                params_info.iommu_status = "Error checking status".to_string();
                params_info.iommu_verification_method = IOMMU_SYSFS_PATH.to_string();
            }
        }
    }
    if params_info.iommu_status.is_empty() { // Default if no other status set
        params_info.iommu_status = "Not Detected/Disabled".to_string();
        params_info.iommu_verification_method = "Initial default".to_string();
    }


    // 3. Check Nested Virtualization
    let mut nested_found = false;
    if Path::new(KVM_INTEL_NESTED_PATH).exists() {
        match fs::read_to_string(KVM_INTEL_NESTED_PATH) {
            Ok(val) => {
                if val.trim() == "Y" || val.trim() == "1" || val.trim() == "y" {
                    params_info.nested_virtualization_status = "Enabled (kvm-intel.nested=1)".to_string();
                    nested_found = true;
                } else {
                    params_info.nested_virtualization_status = "Disabled".to_string();
                }
                params_info.nested_virtualization_verification_method = KVM_INTEL_NESTED_PATH.to_string();
            }
            Err(e) => {
                warn!("Could not read {}: {}", KVM_INTEL_NESTED_PATH, e);
                params_info.nested_virtualization_status = "Error checking status".to_string();
                params_info.nested_virtualization_verification_method = KVM_INTEL_NESTED_PATH.to_string();
            }
        }
    } else if Path::new(KVM_AMD_NESTED_PATH).exists() {
        match fs::read_to_string(KVM_AMD_NESTED_PATH) {
            Ok(val) => {
                if val.trim() == "Y" || val.trim() == "1" || val.trim() == "y" {
                    params_info.nested_virtualization_status = "Enabled (kvm-amd.nested=1)".to_string();
                    nested_found = true;
                } else {
                    params_info.nested_virtualization_status = "Disabled".to_string();
                }
                params_info.nested_virtualization_verification_method = KVM_AMD_NESTED_PATH.to_string();
            }
            Err(e) => {
                warn!("Could not read {}: {}", KVM_AMD_NESTED_PATH, e);
                params_info.nested_virtualization_status = "Error checking status".to_string();
                params_info.nested_virtualization_verification_method = KVM_AMD_NESTED_PATH.to_string();
            }
        }
    } else {
        // Check if KVM modules are loaded at all
        let kvm_intel_loaded = Path::new("/sys/module/kvm_intel").exists();
        let kvm_amd_loaded = Path::new("/sys/module/kvm_amd").exists();
        if kvm_intel_loaded || kvm_amd_loaded {
            params_info.nested_virtualization_status = "Disabled (module loaded, nested param file not found or not Y/1)".to_string();
            params_info.nested_virtualization_verification_method = "/sys/module/kvm_intel or /sys/module/kvm_amd".to_string();
        } else {
            params_info.nested_virtualization_status = "Not Applicable (kvm_intel or kvm_amd module not loaded)".to_string();
            params_info.nested_virtualization_verification_method = "/sys/module/".to_string();
        }
    }
    if !nested_found && params_info.nested_virtualization_status.is_empty() {
        params_info.nested_virtualization_status = "Not Detected/Disabled".to_string();
    }


    // 4. Check for Huge Pages (basic check)
    match fs::File::open(MEMINFO_PATH) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut hugepages_total: Option<u64> = None;
            let mut hugepages_size: Option<u64> = None;
            for line in reader.lines() {
                if let Ok(line_content) = line {
                    if let Some((key_raw, value_raw)) = line_content.split_once(':') {
                        let key = key_raw.trim();
                        let value_part = value_raw.trim().split_whitespace().next().unwrap_or("");
                        if key == "HugePages_Total" {
                            if let Ok(val) = value_part.parse::<u64>() { hugepages_total = Some(val); }
                        } else if key == "Hugepagesize" {
                            if let Ok(val) = value_part.parse::<u64>() { hugepages_size = Some(val); }
                        }
                        if hugepages_total.is_some() && hugepages_size.is_some() { break; }
                    }
                }
            }
            if let (Some(total), Some(size)) = (hugepages_total, hugepages_size) {
                if total > 0 {
                    params_info.other_relevant_params.insert("HugePages".to_string(), format!("Enabled (Total: {}, Size: {} kB)", total, size));
                    info!("HugePages detected: Total={}, Size={} kB", total, size);
                } else {
                    params_info.other_relevant_params.insert("HugePages".to_string(), "Configured but Total is 0".to_string());
                    info!("HugePages configured but Total is 0.");
                }
            } else {
                params_info.other_relevant_params.insert("HugePages".to_string(), "Not Detected".to_string());
                info!("HugePages parameters not detected in {}.", MEMINFO_PATH);
            }
        }
        Err(e) => {
            warn!("Could not read {}: {}. HugePages info unavailable.", MEMINFO_PATH, e);
            params_info.other_relevant_params.insert("HugePages".to_string(), format!("Error checking: {}", e));
        }
    }


    info!("Kernel Virtualization Parameters: {:?}", params_info);
    ctx.set_data(KERNEL_VIRT_PARAMS_KEY, params_info);
    Ok(())
}

/// Scans bootloader configuration for kernel parameters.
#[allow(dead_code)] // Called by VirtualizationKernelParamsStage
async fn scan_bootloader_configuration(ctx: &mut StageContext) -> KernelResult<()> {
    use log::{info, warn};
    info!("Stage: Scanning Bootloader Configuration...");
    const BOOTLOADER_SCAN_KEY: &str = "bootloader_scan_info";
    let mut scan_info = BootloaderScanInfo::default();

    const GRUB_DEFAULT_PATH: &str = "/etc/default/grub";
    const GRUB_CFG_PATH: &str = "/boot/grub/grub.cfg"; // Less common to parse directly, but good to check existence
    const SYSTEMD_BOOT_ENTRIES_DIR1: &str = "/boot/loader/entries/";
    const SYSTEMD_BOOT_ENTRIES_DIR2: &str = "/efi/loader/entries/"; // ESP path

    // Check for GRUB
    if Path::new(GRUB_DEFAULT_PATH).exists() {
        scan_info.identified_bootloader = Some("GRUB".to_string());
        scan_info.config_files_checked.push(GRUB_DEFAULT_PATH.to_string());
        info!("GRUB identified by presence of {}", GRUB_DEFAULT_PATH);
        match fs::read_to_string(GRUB_DEFAULT_PATH) {
            Ok(content) => {
                let mut grub_cmdline = String::new();
                for line in content.lines() {
                    if line.starts_with("GRUB_CMDLINE_LINUX_DEFAULT=") {
                        if let Some(val) = line.split_once('=').map(|(_, v)| v.trim().trim_matches('"')) {
                            grub_cmdline.push_str(val);
                            grub_cmdline.push(' ');
                        }
                    } else if line.starts_with("GRUB_CMDLINE_LINUX=") {
                        if let Some(val) = line.split_once('=').map(|(_, v)| v.trim().trim_matches('"')) {
                            grub_cmdline.push_str(val);
                            grub_cmdline.push(' ');
                        }
                    }
                }
                if !grub_cmdline.trim().is_empty() {
                    scan_info.kernel_params_in_config = Some(grub_cmdline.trim().to_string());
                    info!("GRUB params from {}: {}", GRUB_DEFAULT_PATH, scan_info.kernel_params_in_config.as_ref().unwrap());
                } else {
                    info!("No GRUB_CMDLINE_LINUX or GRUB_CMDLINE_LINUX_DEFAULT found in {}", GRUB_DEFAULT_PATH);
                }
            }
            Err(e) => {
                warn!("Could not read {}: {}", GRUB_DEFAULT_PATH, e);
                scan_info.warnings.push(format!("Failed to read {}: {}", GRUB_DEFAULT_PATH, e));
            }
        }
    }
    if Path::new(GRUB_CFG_PATH).exists() {
        scan_info.config_files_checked.push(GRUB_CFG_PATH.to_string());
    }

    // Check for systemd-boot
    let systemd_dirs_to_check = [SYSTEMD_BOOT_ENTRIES_DIR1, SYSTEMD_BOOT_ENTRIES_DIR2];
    // let mut systemd_boot_found = false; // FIX: Removed unused variable
    for dir_path_str in systemd_dirs_to_check.iter() {
        let dir_path = Path::new(dir_path_str);
        if dir_path.exists() && dir_path.is_dir() {
            // systemd_boot_found = true; // FIX: Removed assignment to unused variable
            if scan_info.identified_bootloader.is_none() { // Only set if GRUB wasn't found
                scan_info.identified_bootloader = Some("systemd-boot".to_string());
            }
            info!("systemd-boot identified by presence of {}", dir_path_str);
            scan_info.config_files_checked.push(dir_path_str.to_string());

            match fs::read_dir(dir_path) {
                Ok(entries) => {
                    for entry_result in entries {
                        if let Ok(entry) = entry_result {
                            let path = entry.path();
                            if path.is_file() && path.extension().map_or(false, |ext| ext == "conf") {
                                scan_info.config_files_checked.push(path.display().to_string());
                                match fs::read_to_string(&path) {
                                    Ok(content) => {
                                        for line in content.lines() {
                                            if line.trim().starts_with("options ") {
                                                let params = line.trim().strip_prefix("options ").unwrap_or("").trim().to_string();
                                                if !params.is_empty() {
                                                    if let Some(existing_params) = &mut scan_info.kernel_params_in_config {
                                                        existing_params.push_str(&format!("; (from {}): {}", path.file_name().unwrap_or_default().to_string_lossy(), params));
                                                    } else {
                                                        scan_info.kernel_params_in_config = Some(format!("(from {}): {}", path.file_name().unwrap_or_default().to_string_lossy(), params));
                                                    }
                                                    info!("systemd-boot params from {}: {}", path.display(), params);
                                                }
                                                break; // Assuming one options line per file for simplicity
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Could not read systemd-boot entry {}: {}", path.display(), e);
                                        scan_info.warnings.push(format!("Failed to read {}: {}", path.display(), e));
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Could not read systemd-boot entries directory {}: {}", dir_path_str, e);
                    scan_info.warnings.push(format!("Failed to list entries in {}: {}", dir_path_str, e));
                }
            }
        }
    }

    if scan_info.identified_bootloader.is_none() {
        scan_info.identified_bootloader = Some("unknown".to_string());
        info!("No common bootloader (GRUB, systemd-boot) identified by standard paths.");
        scan_info.recommendations.push("Could not identify a common bootloader. Ensure kernel parameters for virtualization are persistently set.".to_string());
    }

    // Compare with effective parameters if available
    if let Some(kernel_params_info) = ctx.get_data::<KernelVirtualizationParamsInfo>("kernel_virtualization_params_info") {
        let effective_cmdline = &kernel_params_info.raw_cmdline;
        if let Some(config_cmdline) = &scan_info.kernel_params_in_config {
            // This is a simplistic comparison. A more robust one would parse individual params.
            if !effective_cmdline.contains(config_cmdline) && !config_cmdline.split(';').any(|part| effective_cmdline.contains(part.split(':').last().unwrap_or("").trim())) {
                let warning_msg = format!(
                    "Potential discrepancy: Effective kernel cmdline ('{}') may differ significantly from bootloader configured params ('{}'). Review persistence.",
                    effective_cmdline, config_cmdline
                );
                info!("{}", warning_msg);
                scan_info.warnings.push(warning_msg);
                scan_info.recommendations.push("Review bootloader configuration to ensure all desired kernel parameters are persistently set. Refer to core-env docs.".to_string());

            }
            // Check for specific important params like iommu and nested
            if (effective_cmdline.contains("intel_iommu=on") || effective_cmdline.contains("amd_iommu=on")) &&
                !(config_cmdline.contains("intel_iommu=on") || config_cmdline.contains("amd_iommu=on")) {
                let msg = "Effective IOMMU parameter is active, but not found in bootloader config. Consider adding for persistence.".to_string();
                info!("{}", msg);
                scan_info.warnings.push(msg);
            }
            if (effective_cmdline.contains("kvm-intel.nested=1") || effective_cmdline.contains("kvm-amd.nested=1")) &&
                !(config_cmdline.contains("kvm-intel.nested=1") || config_cmdline.contains("kvm-amd.nested=1")) {
                let msg = "Effective nested virtualization parameter is active, but not found in bootloader config. Consider adding for persistence.".to_string();
                info!("{}", msg);
                scan_info.warnings.push(msg);
            }

        } else {
            let msg = "Bootloader configuration scan did not find explicit kernel parameters. Current kernel parameters are active. Ensure these are persistently set in your bootloader. See core-env docs.".to_string();
            info!("{}", msg);
            scan_info.recommendations.push(msg);
        }
    } else {
        let msg = "Could not retrieve effective kernel parameters to compare with bootloader configuration.".to_string();
        info!("{}", msg);
        scan_info.warnings.push(msg);
    }


    info!("Bootloader Scan Info: {:?}", scan_info);
    ctx.set_data(BOOTLOADER_SCAN_KEY, scan_info);
    Ok(())
}

/// Checks swap configuration, ZRAM, and swappiness.
#[allow(dead_code)] // Called by CheckSwapStage
async fn check_swap_config(ctx: &mut StageContext) -> KernelResult<()> {
    use log::{info, warn};
    info!("Stage: Checking Swap Configuration...");
    const SWAP_INFO_KEY: &str = "env_check:swap_info";
    let mut swap_info = SwapInfo::default();

    const SWAPS_PATH: &str = "/proc/swaps";
    const MEMINFO_PATH: &str = "/proc/meminfo";
    const SWAPPINESS_PATH: &str = "/proc/sys/vm/swappiness";
    const ZRAM_SYS_BASE_PATH: &str = "/sys/block";

    // 1. Read /proc/swaps for active swap devices/files
    match fs::read_to_string(SWAPS_PATH) {
        Ok(content) => {
            for line in content.lines().skip(1) { // Skip header
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() { continue; }
                let device_name = parts[0].to_string();
                if device_name.starts_with("/dev/zram") {
                    let mut zram_device = ZramDeviceInfo {
                        device_name: device_name.trim_start_matches("/dev/").to_string(),
                        ..Default::default()
                    };
                    // let zram_id = zram_device.device_name.trim_start_matches("zram"); // zram_id not used further for now

                    // Check /sys/block/zramX for details
                    let zram_sys_path = Path::new(ZRAM_SYS_BASE_PATH).join(&zram_device.device_name);
                    if zram_sys_path.exists() {
                        if let Ok(disk_size_str) = fs::read_to_string(zram_sys_path.join("disksize")) {
                            zram_device.disk_size_bytes = disk_size_str.trim().parse::<u64>().ok();
                        }
                        if let Ok(algo_str) = fs::read_to_string(zram_sys_path.join("comp_algorithm")) {
                            // Format: "[lzo] lzo-rle lz4 ..."
                            zram_device.compression_algo = algo_str.split_whitespace().find_map(|s| s.strip_prefix('[').and_then(|s| s.strip_suffix(']'))).map(str::to_string);
                        }
                        if let Ok(orig_size_str) = fs::read_to_string(zram_sys_path.join("orig_data_size")) {
                            zram_device.original_data_size_bytes = orig_size_str.trim().parse::<u64>().ok();
                        }
                        if let Ok(compr_size_str) = fs::read_to_string(zram_sys_path.join("compr_data_size")) {
                            zram_device.compressed_data_size_bytes = compr_size_str.trim().parse::<u64>().ok();
                        }
                        if let Ok(mem_used_str) = fs::read_to_string(zram_sys_path.join("mem_used_total")) {
                            zram_device.mem_used_total_bytes = mem_used_str.trim().parse::<u64>().ok();
                        }
                    }
                    info!("Found ZRAM device: {:?}", zram_device);
                    swap_info.zram_devices.push(zram_device);

                } else if parts.get(1).map_or(false, |&t| t == "partition") {
                    info!("Found swap partition: {}", device_name);
                    swap_info.active_swap_partitions.push(device_name);
                } else if parts.get(1).map_or(false, |&t| t == "file") {
                    info!("Found swap file: {}", device_name);
                    swap_info.active_swap_files.push(device_name);
                }
            }
        }
        Err(e) => {
            warn!("Could not read {}: {}. Active swap devices info will be incomplete.", SWAPS_PATH, e);
        }
    }

    // 2. Read /proc/meminfo for SwapTotal and SwapFree
    match fs::File::open(MEMINFO_PATH) {
        Ok(file) => {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(line_content) = line {
                    if let Some((key_raw, value_raw)) = line_content.split_once(':') {
                        let key = key_raw.trim();
                        let value_part = value_raw.trim().split_whitespace().next().unwrap_or("");
                        if key == "SwapTotal" {
                            swap_info.total_swap_kb = value_part.parse::<u64>().ok();
                        } else if key == "SwapFree" {
                            swap_info.free_swap_kb = value_part.parse::<u64>().ok();
                        }
                        if swap_info.total_swap_kb.is_some() && swap_info.free_swap_kb.is_some() {
                            break;
                        }
                    }
                }
            }
            info!("Swap from meminfo: Total={:?} kB, Free={:?} kB", swap_info.total_swap_kb, swap_info.free_swap_kb);
        }
        Err(e) => {
            warn!("Could not read {}: {}. Swap total/free info unavailable.", MEMINFO_PATH, e);
        }
    }

    // 3. Read /proc/sys/vm/swappiness
    match fs::read_to_string(SWAPPINESS_PATH) {
        Ok(val_str) => {
            swap_info.swappiness = val_str.trim().parse::<u8>().ok();
            info!("Swappiness: {:?}", swap_info.swappiness);
        }
        Err(e) => {
            warn!("Could not read {}: {}. Swappiness info unavailable.", SWAPPINESS_PATH, e);
        }
    }

    info!("Swap Configuration: {:?}", swap_info);
    ctx.set_data(SWAP_INFO_KEY, swap_info);
    Ok(())
}

/// Performs a best-effort check of LVM configuration.
#[allow(dead_code)] // Called by CheckLvmStage
async fn check_lvm_config(ctx: &mut StageContext) -> KernelResult<()> {
    use log::{info, warn};
    info!("Stage: Checking LVM Configuration (Best Effort)...");
    const LVM_INFO_KEY: &str = "env_check:lvm_info";
    let mut lvm_info = LvmInfo::default();

    const DM_SYS_BASE_PATH: &str = "/sys/class/block/";
    const MAPPER_DEV_PATH: &str = "/dev/mapper/";
    const PROC_MOUNTS_PATH: &str = "/proc/mounts";

    let mut mount_points: HashMap<String, (String, String)> = HashMap::new(); // dev_path -> (mount_point, fs_type)
    match fs::read_to_string(PROC_MOUNTS_PATH) {
        Ok(mounts_content) => {
            for line in mounts_content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    mount_points.insert(parts[0].to_string(), (parts[1].to_string(), parts[2].to_string()));
                }
            }
        }
        Err(e) => {
            warn!("Could not read {}: {}. Filesystem info for LVs will be unavailable.", PROC_MOUNTS_PATH, e);
        }
    }

    if let Ok(entries) = fs::read_dir(DM_SYS_BASE_PATH) {
        for entry_result in entries {
            if let Ok(entry) = entry_result {
                let device_name = entry.file_name().to_string_lossy().to_string();
                if !device_name.starts_with("dm-") { continue; }

                let dm_path = Path::new(DM_SYS_BASE_PATH).join(&device_name);
                let dm_uuid_path = dm_path.join("dm/uuid");
                let dm_name_path = dm_path.join("dm/name");
                let dm_size_path = dm_path.join("size"); // Sectors

                if let Ok(uuid_content) = fs::read_to_string(&dm_uuid_path) {
                    if uuid_content.trim().starts_with("LVM-") { // This is an LVM LV
                        let mut lv = LogicalVolumeInfo::default();
                        lv.lv_uuid = Some(uuid_content.trim().to_string());

                        if let Ok(name_content) = fs::read_to_string(&dm_name_path) {
                            let mapper_name = name_content.trim().to_string();
                            // Try to infer VG and LV name (e.g., "vg01-lv_root" -> vg01, lv_root)
                            if let Some((vg, l_name)) = mapper_name.split_once('-') {
                                lv.vg_name = vg.to_string();
                                lv.lv_name = l_name.to_string();
                            } else {
                                lv.lv_name = mapper_name.clone(); // Could be just LV name if VG not in mapper name
                            }
                            lv.lv_path = format!("{}{}", MAPPER_DEV_PATH, mapper_name);
                            info!("Found LVM LV: {} (Mapper: {})", lv.lv_path, device_name);
                        } else {
                            warn!("Could not read dm name for {}", device_name);
                            lv.lv_path = format!("/dev/{}", device_name); // Fallback path
                        }

                        if let Ok(size_content) = fs::read_to_string(&dm_size_path) {
                            if let Ok(sectors) = size_content.trim().parse::<u64>() {
                                lv.lv_size_bytes = Some(sectors * 512);
                            }
                        }

                        let dev_path_for_mount = format!("/dev/{}", device_name);
                        let lv_path_for_mount = lv.lv_path.clone();

                        if let Some((mp, fs)) = mount_points.get(&lv_path_for_mount).or_else(|| mount_points.get(&dev_path_for_mount)) {
                            lv.mount_point = Some(mp.clone());
                            lv.filesystem_type = Some(fs.clone());
                        }

                        // Thin/snapshot/cache/RAID detection is very hard without LVM tools
                        // Add warnings about this limitation
                        if lvm_info.warnings.is_empty() { // Add only once
                            lvm_info.warnings.push("Detailed LVM features (thin provisioning, snapshots, cache, RAID, PV/VG specifics) are hard to detect reliably without LVM tools. Checks are best-effort based on /sys and /dev/mapper.".to_string());
                        }

                        lvm_info.logical_volumes.push(lv);
                    }
                }
            }
        }
    } else {
        warn!("Could not read {}. LVM info will be unavailable.", DM_SYS_BASE_PATH);
        lvm_info.warnings.push(format!("Could not read sysfs block devices at {}.", DM_SYS_BASE_PATH));
    }

    // Best effort to populate VG info based on LVs found
    let mut vg_map: HashMap<String, VolumeGroupInfo> = HashMap::new();
    for lv in &lvm_info.logical_volumes {
        let vg_entry = vg_map.entry(lv.vg_name.clone()).or_insert_with(|| VolumeGroupInfo {
            vg_name: lv.vg_name.clone(),
            ..Default::default()
        });
        vg_entry.lv_count += 1;
        if let Some(lv_size) = lv.lv_size_bytes {
            vg_entry.vg_size_bytes = Some(vg_entry.vg_size_bytes.unwrap_or(0) + lv_size);
        }
    }
    lvm_info.volume_groups = vg_map.into_values().collect();


    info!("LVM Configuration (Best Effort): {:?}", lvm_info);
    ctx.set_data(LVM_INFO_KEY, lvm_info);
    Ok(())
}

/// Checks network virtualization settings like IP forwarding, bridges, and TAP devices.
#[allow(dead_code)] // Called by CheckNetworkVirtStage
async fn check_network_virt_config(ctx: &mut StageContext) -> KernelResult<()> {
    use log::{info, warn};
    info!("Stage: Checking Network Virtualization Settings...");
    const NET_VIRT_INFO_KEY: &str = "env_check:network_virt_info";
    let mut net_info = NetworkVirtInfo::default();

    const IP_FORWARD_PATH: &str = "/proc/sys/net/ipv4/ip_forward";
    const NET_CLASS_PATH: &str = "/sys/class/net/";

    // 1. Check IP Forwarding
    match fs::read_to_string(IP_FORWARD_PATH) {
        Ok(val_str) => {
            net_info.ip_forwarding_enabled = Some(val_str.trim() == "1");
            info!("IPv4 forwarding: {:?}", net_info.ip_forwarding_enabled);
        }
        Err(e) => {
            warn!("Could not read {}: {}. IP forwarding status unknown.", IP_FORWARD_PATH, e);
            net_info.warnings.push(format!("Failed to read IP forwarding status: {}", e));
        }
    }

    // 2. Check for Bridges and TAP devices
    if let Ok(entries) = fs::read_dir(NET_CLASS_PATH) {
        for entry_result in entries {
            if let Ok(entry) = entry_result {
                let iface_name = entry.file_name().to_string_lossy().to_string();
                let iface_path = Path::new(NET_CLASS_PATH).join(&iface_name);

                // Check for Bridge
                let bridge_sysfs_path = iface_path.join("bridge");
                if bridge_sysfs_path.exists() && bridge_sysfs_path.is_dir() {
                    let mut bridge = BridgeInfo { bridge_name: iface_name.clone(), ..Default::default() };
                    if let Ok(mac_addr) = fs::read_to_string(iface_path.join("address")) {
                        bridge.mac_address = Some(mac_addr.trim().to_string());
                    }
                    let brif_path = bridge_sysfs_path.join("brif"); // Corrected path
                    if brif_path.exists() { // Check if brif directory exists
                        if let Ok(br_entries) = fs::read_dir(brif_path) {
                            for br_entry_res in br_entries {
                                if let Ok(br_entry) = br_entry_res {
                                    bridge.interfaces.push(br_entry.file_name().to_string_lossy().to_string());
                                }
                            }
                        }
                    } else {
                        // It's normal for a bridge to have no interfaces attached yet.
                        // info!("Bridge {} has no interfaces currently attached (brif directory does not exist).", iface_name);
                    }
                    info!("Found bridge: {:?}", bridge);
                    net_info.bridges.push(bridge);
                    continue; // An interface is either a bridge or potentially a tap, not both directly
                }

                // Check for TAP device (best effort)
                let tap_flags_path = iface_path.join("tun_flags");
                if tap_flags_path.exists() {
                    match fs::read_to_string(&tap_flags_path) {
                        Ok(flags_hex) => {
                            if let Ok(flags_val) = u32::from_str_radix(flags_hex.trim().trim_start_matches("0x"), 16) {
                                if (flags_val & 0x0002) != 0 { // IFF_TAP is 0x0002
                                    let mut tap_device = TapDeviceInfo { tap_name: iface_name.clone(), ..Default::default() };
                                    if let Ok(mac_addr) = fs::read_to_string(iface_path.join("address")) {
                                        tap_device.mac_address = Some(mac_addr.trim().to_string());
                                    }
                                    if let Ok(owner_str) = fs::read_to_string(iface_path.join("owner")) { // owner might not exist or be readable
                                        tap_device.owner_uid = owner_str.trim().parse::<u32>().ok();
                                    }
                                    if let Ok(flags_content) = fs::read_to_string(iface_path.join("flags")) { // general interface flags
                                        tap_device.flags = Some(flags_content.trim().to_string());
                                    }
                                    info!("Found TAP device (via tun_flags): {:?}", tap_device);
                                    net_info.tap_devices.push(tap_device);
                                }
                            } else {
                                warn!("Could not parse tun_flags for {}: {}", iface_name, flags_hex);
                            }
                        }
                        Err(_e) => { /* File might not be readable, or not exist for non-tuntap */ }
                    }
                }
            }
        }
    } else {
        warn!("Could not read {}. Network interface info will be unavailable.", NET_CLASS_PATH);
        net_info.warnings.push(format!("Failed to list network interfaces: {}", NET_CLASS_PATH));
    }


    info!("Network Virtualization Info: {:?}", net_info);
    ctx.set_data(NET_VIRT_INFO_KEY, net_info);
    Ok(())
}

/// Identifies package manager and checks for essential virtualization packages.
#[allow(dead_code)] // Called by CheckSystemPackagesStage
async fn check_system_packages(ctx: &mut StageContext) -> KernelResult<()> {
    use log::{info, warn};
    info!("Stage: Checking System Packages...");
    const PACKAGES_INFO_KEY: &str = "env_check:system_packages_info";
    let mut packages_info = SystemPackagesInfo::default();

    // Package Manager Identification (best effort)
    const DPKG_STATUS_FILE: &str = "/var/lib/dpkg/status";
    const APT_BIN: &str = "/usr/bin/apt"; // For Debian/Ubuntu family
    const RPM_PACKAGES_DB: &str = "/var/lib/rpm/Packages"; // File, not dir
    const RPM_BIN: &str = "/usr/bin/rpm"; // Generic RPM binary
    const DNF_BIN: &str = "/usr/bin/dnf"; // For Fedora/RHEL family
    const YUM_BIN: &str = "/usr/bin/yum"; // For older RHEL/CentOS
    const PACMAN_LOCAL_DB_DIR: &str = "/var/lib/pacman/local/";
    const PACMAN_BIN: &str = "/usr/bin/pacman";


    if Path::new(DPKG_STATUS_FILE).exists() || Path::new(APT_BIN).exists() {
        packages_info.package_manager.identified_manager = Some("dpkg/apt".to_string());
        packages_info.package_manager.detection_method = Some(
            if Path::new(DPKG_STATUS_FILE).exists() { DPKG_STATUS_FILE.to_string() } else { APT_BIN.to_string()}
        );
    } else if Path::new(RPM_PACKAGES_DB).exists() || Path::new(RPM_BIN).exists() || Path::new(DNF_BIN).exists() || Path::new(YUM_BIN).exists() {
        packages_info.package_manager.identified_manager = Some("rpm/dnf/yum".to_string());
        let method = if Path::new(RPM_PACKAGES_DB).exists() { RPM_PACKAGES_DB.to_string() }
        else if Path::new(DNF_BIN).exists() { DNF_BIN.to_string() }
        else if Path::new(YUM_BIN).exists() { YUM_BIN.to_string() }
        else { RPM_BIN.to_string() };
        packages_info.package_manager.detection_method = Some(method);
    } else if Path::new(PACMAN_LOCAL_DB_DIR).is_dir() || Path::new(PACMAN_BIN).exists() {
        packages_info.package_manager.identified_manager = Some("pacman".to_string());
        packages_info.package_manager.detection_method = Some(
            if Path::new(PACMAN_LOCAL_DB_DIR).is_dir() { PACMAN_LOCAL_DB_DIR.to_string() } else { PACMAN_BIN.to_string()}
        );
    } else {
        packages_info.package_manager.identified_manager = Some("unknown".to_string());
        packages_info.package_manager.detection_method = Some("No common package manager indicators found".to_string());
        warn!("Could not identify a common package manager.");
        packages_info.warnings.push("Unable to identify package manager. Package checks will be limited or skipped.".to_string());
    }
    info!("Package manager identified as {} via {}",
        packages_info.package_manager.identified_manager.as_ref().unwrap_or(&"unknown".to_string()),
        packages_info.package_manager.detection_method.as_ref().unwrap_or(&"N/A".to_string())
    );

    // Packages derived from docs: kvm.md, qemu.md, libvirt.md, lvm.md
    // (Canonical Name, Common Binary/File Path for fallback, Vec<Alternative/DistroSpecificPackageNames>)
    let packages_to_verify: Vec<(&str, &str, Vec<&str>)> = vec![
        // QEMU/KVM Core & UEFI
        ("qemu", "/usr/bin/qemu-system-x86_64", vec!["qemu-kvm", "qemu-system-x86", "qemu-base", "qemu-desktop", "qemu-full"]),
        ("qemu-utils", "/usr/bin/qemu-img", vec![]),
        ("edk2-ovmf", "/usr/share/edk2/x64/OVMF_CODE.4m.fd", vec!["ovmf"]), // Path might vary more
        ("swtpm", "/usr/bin/swtpm", vec!["swtpm-tools"]),
        // Libvirt Stack
        ("libvirt", "/usr/sbin/libvirtd", vec!["libvirt-daemon", "libvirt-clients", "libvirt-bin"]),
        ("dnsmasq", "/usr/sbin/dnsmasq", vec!["dnsmasq-base"]),
        // LVM
        ("lvm2", "/usr/sbin/lvdisplay", vec![]),
        // Networking
        ("bridge-utils", "/usr/sbin/brctl", vec![]),
        // Optional but useful / mentioned in docs
        ("qemu-guest-agent", "/usr/bin/qemu-ga", vec![]),
        ("virtiofsd", "/usr/lib/virtiofsd", vec![]), // Path might be /usr/libexec/virtiofsd
        ("spice-vdagent", "/usr/bin/spice-vdagent", vec![]),
        ("virt-manager", "/usr/bin/virt-manager", vec![]),
        ("virt-viewer", "/usr/bin/virt-viewer", vec![]),
        ("libguestfs", "/usr/bin/guestfish", vec![]), // May pull many dependencies
        ("virt-install", "/usr/bin/virt-install", vec![]),
        ("samba", "/usr/sbin/smbd", vec![]),
        ("openbsd-netcat", "/usr/bin/nc", vec!["netcat", "ncat"]), // nc can be from different packages
        ("multipath-tools", "/usr/sbin/kpartx", vec![]),
        ("nbd", "/usr/sbin/nbd-server", vec!["nbd-client"]), // nbd-server might not always exist
        ("virt-firmware", "/usr/share/virt-firmware", vec![]) // This is often a meta-package or directory
    ];

    if let Some(manager_type) = &packages_info.package_manager.identified_manager {
        for (base_pkg_name, common_path, original_alt_names) in packages_to_verify {
            let alt_names = original_alt_names.clone();
            let mut pkg_check_info = PackageCheckInfo {
                package_name: base_pkg_name.to_string(),
                ..Default::default()
            };
            let mut found_by_specific_check = false;
            let mut specific_name_found = base_pkg_name.to_string();

            let mut names_to_check_vec = vec![base_pkg_name];
            names_to_check_vec.extend_from_slice(&alt_names); // Iterate over slice to avoid consuming alt_names


            match manager_type.as_str() {
                "dpkg/apt" => {
                    if Path::new(DPKG_STATUS_FILE).exists() {
                        pkg_check_info.check_method = format!("Parsed {}", DPKG_STATUS_FILE);
                        if let Ok(status_content) = fs::read_to_string(DPKG_STATUS_FILE) {
                            for pkg_to_find in &names_to_check_vec {
                                let mut current_package_found_in_para = false;
                                let mut current_installed_ok = false;
                                let mut version_line: Option<String> = None;

                                for paragraph in status_content.split("\n\n") {
                                    if paragraph.contains(&format!("Package: {}", pkg_to_find)) {
                                        current_package_found_in_para = true;
                                        if paragraph.contains("Status: install ok installed") {
                                            current_installed_ok = true;
                                        }
                                        // Try to find version
                                        if let Some(line) = paragraph.lines().find(|l| l.starts_with("Version: ")) {
                                            version_line = Some(line.trim_start_matches("Version: ").to_string());
                                        }
                                        break;
                                    }
                                }
                                if current_package_found_in_para {
                                    pkg_check_info.is_installed = Some(current_installed_ok);
                                    pkg_check_info.version = version_line;
                                    if current_installed_ok { specific_name_found = pkg_to_find.to_string(); }
                                    found_by_specific_check = true;
                                    break;
                                }
                            }
                            if !found_by_specific_check { pkg_check_info.is_installed = Some(false); }
                        } else {
                            warn!("Could not read dpkg status file {}", DPKG_STATUS_FILE);
                            pkg_check_info.check_method = format!("Error reading {}; fallback to path", DPKG_STATUS_FILE);
                        }
                    }
                }
                "rpm/dnf/yum" => {
                    pkg_check_info.check_method = format!("Path check (RPM DB unparsable): {}", common_path);
                    if Path::new(common_path).exists() {
                        pkg_check_info.is_installed = Some(true);
                        found_by_specific_check = true;
                    } else {
                        // Try alt paths if base path fails
                        // FIX: Iterate over &alt_names (reference) to avoid move
                        for alt_pkg_name_candidate in &alt_names {
                            let alt_common_path = format!("/usr/bin/{}", alt_pkg_name_candidate);
                            let alt_sbin_path = format!("/usr/sbin/{}", alt_pkg_name_candidate);
                            if Path::new(&alt_common_path).exists() || Path::new(&alt_sbin_path).exists() {
                                pkg_check_info.is_installed = Some(true);
                                specific_name_found = alt_pkg_name_candidate.to_string();
                                pkg_check_info.check_method.push_str(&format!("; found alt path for {}", specific_name_found));
                                found_by_specific_check = true;
                                break;
                            }
                        }
                        if !found_by_specific_check { pkg_check_info.is_installed = Some(false); }
                    }
                }
                "pacman" => {
                    pkg_check_info.check_method = format!("Scanned {}", PACMAN_LOCAL_DB_DIR);
                    let mut found_in_pacman_db = false;
                    if Path::new(PACMAN_LOCAL_DB_DIR).is_dir() {
                        for pkg_to_find in &names_to_check_vec {
                            if let Ok(entries) = fs::read_dir(PACMAN_LOCAL_DB_DIR) { // Re-open for each name to check
                                for entry_result in entries.filter_map(Result::ok) {
                                    let path = entry_result.path();
                                    if path.is_dir() {
                                        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                                            if dir_name.starts_with(&format!("{}-", pkg_to_find)) {
                                                pkg_check_info.is_installed = Some(true);
                                                specific_name_found = pkg_to_find.to_string();
                                                let desc_path = path.join("desc");
                                                if let Ok(desc_content) = fs::read_to_string(&desc_path) {
                                                    let mut in_version_block = false;
                                                    for line in desc_content.lines() {
                                                        if line == "%VERSION%" { in_version_block = true; continue; }
                                                        if in_version_block { pkg_check_info.version = Some(line.to_string()); break; }
                                                    }
                                                }
                                                found_in_pacman_db = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            if found_in_pacman_db { break; }
                        }
                        if !found_in_pacman_db { pkg_check_info.is_installed = Some(false); }
                        found_by_specific_check = true;
                    }
                }
                _ => {
                    pkg_check_info.check_method = format!("Path check (unknown manager): {}", common_path);
                }
            }
            pkg_check_info.package_name = specific_name_found; // Ensure package_name reflects what was found

            // Fallback to path check if not found by specific method or manager is unknown
            if !found_by_specific_check || pkg_check_info.is_installed.is_none() {
                let mut path_found = false;
                let mut path_used_for_check = common_path.to_string();

                if Path::new(common_path).exists() {
                    path_found = true;
                } else {
                     // FIX: Iterate over &alt_names (reference) to avoid move
                    for alt_pkg_name_candidate in &alt_names {
                        let alt_bin_path = format!("/usr/bin/{}", alt_pkg_name_candidate);
                        let alt_sbin_path = format!("/usr/sbin/{}", alt_pkg_name_candidate);
                        let alt_share_path = format!("/usr/share/{}", alt_pkg_name_candidate);

                        if Path::new(&alt_bin_path).exists() {
                            pkg_check_info.package_name = alt_pkg_name_candidate.to_string();
                            path_used_for_check = alt_bin_path;
                            path_found = true;
                            break;
                        } else if Path::new(&alt_sbin_path).exists() {
                            pkg_check_info.package_name = alt_pkg_name_candidate.to_string();
                            path_used_for_check = alt_sbin_path;
                            path_found = true;
                            break;
                        } else if Path::new(&alt_share_path).is_dir() || Path::new(&alt_share_path).is_file() {
                            pkg_check_info.package_name = alt_pkg_name_candidate.to_string();
                            path_used_for_check = alt_share_path;
                            path_found = true;
                            break;
                        }
                    }
                }
                pkg_check_info.is_installed = Some(path_found);
                if pkg_check_info.check_method.is_empty() || !pkg_check_info.check_method.contains("Path check") {
                    pkg_check_info.check_method.push_str(&format!("; Fallback path check: {}", path_used_for_check));
                }
            }

            if !pkg_check_info.is_installed.unwrap_or(false) {
                let msg = format!("Required package '{}' might be missing. Check method: {}.", pkg_check_info.package_name, pkg_check_info.check_method);
                warn!("{}", msg);
                packages_info.warnings.push(msg.clone());
                packages_info.recommendations.push(format!("Consider installing package '{}'. Refer to core-env docs or your distribution's documentation.", pkg_check_info.package_name));
            } else {
                info!("Package check for '{}': Installed ({:?}), Version: {:?}, Method: {}", pkg_check_info.package_name, pkg_check_info.is_installed, pkg_check_info.version, pkg_check_info.check_method);
            }
            packages_info.checked_packages.push(pkg_check_info);
        }
    }

    if let Some(lvm_data) = ctx.get_data::<LvmInfo>("env_check:lvm_info") {
        if !lvm_data.logical_volumes.is_empty() || !lvm_data.volume_groups.is_empty() {
            if let Some(lvm_pkg_check) = packages_info.checked_packages.iter().find(|p| p.package_name == "lvm2") {
                if !lvm_pkg_check.is_installed.unwrap_or(false) {
                    let msg = "LVM volumes/groups detected, but 'lvm2' package appears to be missing. LVM management tools might be unavailable.".to_string();
                    warn!("{}", msg);
                    packages_info.warnings.push(msg);
                    packages_info.recommendations.push("Install 'lvm2' package for proper LVM management.".to_string());
                }
            }
        }
    }

    info!("System Packages Info: {:?}", packages_info);
    ctx.set_data(PACKAGES_INFO_KEY, packages_info);
    Ok(())
}


// Export the plugin implementation - Keep commented out until macro location/necessity is confirmed
// // use gini_core::plugin_impl;
// // plugin_impl!(EnvironmentCheckPlugin);


// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*; // Import items from parent module
                  // use gini_core::stage_manager::context::ExecutionMode; // Removed unused import
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    // Test function needs to be async because gather_os_info_from_file is async
    #[tokio::test]
    async fn test_gather_os_info_success() {
        // 1. Create a temporary file with mock os-release content
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(temp_file, "ID=testos").unwrap();
        writeln!(temp_file, "NAME=\"Test OS\"").unwrap();
        writeln!(temp_file, "VERSION_ID=\"1.2.3\"").unwrap();
        writeln!(temp_file, "PRETTY_NAME=\"Test OS 1.2.3 Alpha\"").unwrap();
        writeln!(temp_file, "# This is a comment").unwrap();
        writeln!(temp_file, "SOME_OTHER_VAR=something").unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // 2. Create a StageContext
        // Use a dummy config path for testing
        let mut ctx = StageContext::new_live(PathBuf::from("/tmp/dummy-config"));

        // 3. Call the logic function with the temp file path
        let result = gather_os_info_from_file(&mut ctx, &temp_path).await;

        // 4. Assert success and correct data in context
        assert!(result.is_ok(), "gather_os_info_from_file failed: {:?}", result.err());

        let os_info_opt = ctx.get_data::<OsInfo>("env_check:os_info");
        assert!(os_info_opt.is_some(), "OsInfo not found in context");

        if let Some(os_info) = os_info_opt {
            assert_eq!(os_info.id.as_deref(), Some("testos"));
            assert_eq!(os_info.name.as_deref(), Some("Test OS"));
            assert_eq!(os_info.version_id.as_deref(), Some("1.2.3"));
            assert_eq!(os_info.pretty_name.as_deref(), Some("Test OS 1.2.3 Alpha"));
        }
    }

    #[tokio::test]
    async fn test_gather_os_info_file_not_found() {
        let mut ctx = StageContext::new_live(PathBuf::from("/tmp/dummy-config"));
        let non_existent_path = PathBuf::from("/tmp/this/path/does/not/exist/os-release");

        let result = gather_os_info_from_file(&mut ctx, &non_existent_path).await;

        // Now expects Ok(()) because we log a warning and proceed with default OsInfo
        assert!(result.is_ok(), "Expected Ok(()) for non-existent file, got {:?}", result.err());

        // Check that default (empty) OsInfo was stored
        let os_info_opt = ctx.get_data::<OsInfo>("env_check:os_info");
        assert!(os_info_opt.is_some(), "OsInfo not found in context after non-existent file");
        if let Some(os_info) = os_info_opt {
            assert!(os_info.id.is_none());
            assert!(os_info.name.is_none());
            assert!(os_info.version_id.is_none());
            assert!(os_info.pretty_name.is_none());
        }
    }

    #[tokio::test]
    async fn test_gather_os_info_empty_file() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let temp_path = temp_file.path().to_path_buf();
        let mut ctx = StageContext::new_live(PathBuf::from("/tmp/dummy-config"));

        let result = gather_os_info_from_file(&mut ctx, &temp_path).await;
        assert!(result.is_ok(), "gather_os_info_from_file failed for empty file: {:?}", result.err());

        let os_info_opt = ctx.get_data::<OsInfo>("env_check:os_info");
        assert!(os_info_opt.is_some(), "OsInfo not found in context for empty file");
        if let Some(os_info) = os_info_opt {
            assert!(os_info.id.is_none());
            assert!(os_info.name.is_none());
            assert!(os_info.version_id.is_none());
            assert!(os_info.pretty_name.is_none());
        }
    }
}
