#![allow(dead_code)] // Allow unused structs for FFI definition

use std::os::raw::c_char;

// These are simplified versions of what gini-core expects.
// They don't need to be fully functional, just define the expected FFI shape.

#[repr(C)]
pub struct PluginDescriptor {
    pub name: *const c_char,
    pub version: *const c_char,
    pub gini_version: *const c_char,
    // ... other fields as expected by the core FFI contract
}

#[repr(C)]
pub struct PluginVTable {
    pub descriptor: extern "C" fn() -> *const PluginDescriptor,
    pub on_load: extern "C" fn(),
    pub on_unload: extern "C" fn(),
    // ... other function pointers as expected
}

// This is the function the loader will look for, but it's misnamed.
// The loader expects "_plugin_init".
#[no_mangle]
pub extern "C" fn _plugin_initialize_incorrectly() -> *const PluginVTable {
    // This function would normally return a valid VTable pointer.
    // For this test, it doesn't matter what it returns as it won't be found.
    std::ptr::null()
}

// To make it a valid library, we can include a correctly named function
// that isn't the one the FFI loader is looking for, or just leave it as is.
// For this test, the key is that `_plugin_init` is missing or misnamed.