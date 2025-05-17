#![allow(dead_code)] // Allow unused structs for FFI definition

use std::os::raw::c_char;

// Simplified FFI definitions
#[repr(C)]
pub struct PluginDescriptor {
    pub name: *const c_char,
    pub version: *const c_char,
    pub gini_version: *const c_char,
}

#[repr(C)]
pub struct PluginVTable {
    pub descriptor: extern "C" fn() -> *const PluginDescriptor,
    pub on_load: extern "C" fn(),
    pub on_unload: extern "C" fn(),
}

// A static VTable instance. In a real plugin, this would be properly initialized.
// For this test, it doesn't need to be fully functional since init will panic.
static FAKE_VTABLE: PluginVTable = PluginVTable {
    descriptor: fake_descriptor,
    on_load: fake_on_load,
    on_unload: fake_on_unload,
};

extern "C" fn fake_descriptor() -> *const PluginDescriptor {
    // This function would normally return a valid PluginDescriptor pointer.
    // It won't be called if _plugin_init panics.
    std::ptr::null()
}

extern "C" fn fake_on_load() {
    // This function would normally contain plugin load logic.
}

extern "C" fn fake_on_unload() {
    // This function would normally contain plugin unload logic.
}

#[no_mangle]
pub extern "C-unwind" fn _plugin_init() -> *const PluginVTable {
    panic!("Plugin initialization deliberately failed!");
    // The line below is unreachable due to the panic, but included for completeness
    // if the panic were to be removed for other testing.
    // &FAKE_VTABLE as *const _
}