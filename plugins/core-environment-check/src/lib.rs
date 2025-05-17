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
    error::PluginSystemError, // Import PluginSystemError
    traits::{Plugin, PluginPriority}, // Import Plugin, PluginPriority
    version::VersionRange,                          // Import VersionRange
};
use gini_core::stage_manager::{
    context::StageContext,         // Import StageContext
    requirement::StageRequirement, // Import StageRequirement
    registry::StageRegistry,       // Import StageRegistry
    Stage,                         // Import Stage trait (defined in stage_manager/mod.rs)
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
        vec![]
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
        Ok(())
    }

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