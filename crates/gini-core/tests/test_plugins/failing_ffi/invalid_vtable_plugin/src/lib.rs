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

#[no_mangle]
pub extern "C" fn _plugin_init() -> *const PluginVTable {
    // Intentionally return a null pointer, simulating a corrupted or invalid VTable.
    std::ptr::null()
}