# Plugin Development Guide for Gini Framework

This guide provides a comprehensive overview of plugin development for the Gini framework, based on our analysis of the example compatibility check plugin. It covers the plugin lifecycle, implementation requirements, safety considerations, and best practices for creating robust, reliable plugins.

## 1. Plugin Architecture Overview

Plugins in Gini are dynamic libraries that implement the `Plugin` trait and expose a standardized FFI interface. The framework loads these plugins at runtime, checks their compatibility, and integrates them into the application's functionality.

### Key Components

1. **Plugin Trait**: The core interface that all plugins must implement
2. **FFI Interface**: The C-compatible interface for cross-language boundaries
3. **VTable**: Function pointer table that enables the host to call plugin functions
4. **Plugin Lifecycle**: Init, preflight check, operation, and shutdown phases

### Plugin Loading Process

1. The host searches for plugin libraries in designated directories
2. It loads the library and calls the `_plugin_init` entry point
3. The plugin returns a VTable with function pointers
4. The host validates compatibility and initializes the plugin
5. After preflight checks succeed, the plugin is activated

## 2. The Plugin Trait Definition

The `Plugin` trait defines the core contract that all plugins must implement. It specifies metadata, lifecycle methods, and integration points with the Gini framework.

### Trait Definition

```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    // Basic identification
    fn name(&self) -> &'static str;
    fn version(&self) -> &str;
    fn is_core(&self) -> bool;
    fn priority(&self) -> PluginPriority;

    // Compatibility and dependencies
    fn compatible_api_versions(&self) -> Vec<VersionRange>;
    fn dependencies(&self) -> Vec<PluginDependency>;
    fn conflicts_with(&self) -> Vec<String>;
    fn incompatible_with(&self) -> Vec<PluginDependency>;

    // Lifecycle
    fn init(&self, app: &mut crate::kernel::bootstrap::Application) -> std::result::Result<(), PluginSystemError>;
    async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginSystemError> {
        // Default: No pre-flight check needed
        Ok(())
    }
    fn shutdown(&self) -> std::result::Result<(), PluginSystemError>;

    // Stage management
    fn required_stages(&self) -> Vec<StageRequirement>;
    fn register_stages(&self, registry: &mut StageRegistry) -> std::result::Result<(), PluginSystemError>;
}
```

### Example Implementation

```rust
struct MyPlugin {
    // Non-zero size to prevent UB with FFI pointers
    _marker: u8,
}

#[async_trait]
impl Plugin for MyPlugin {
    fn name(&self) -> &'static str {
        "MyPlugin"
    }
    
    fn version(&self) -> &str {
        "0.1.0"
    }
    
    fn is_core(&self) -> bool {
        false
    }
    
    fn priority(&self) -> PluginPriority {
        PluginPriority::ThirdParty(150)
    }
    
    fn compatible_api_versions(&self) -> Vec<VersionRange> {
        vec![VersionRange::from_constraint("~0.1.0")
                .expect("Invalid version constraint")]
    }
    
    fn dependencies(&self) -> Vec<PluginDependency> {
        vec![
            PluginDependency::new("RequiredPlugin", ">=1.0.0", true),
            PluginDependency::new("OptionalPlugin", ">=0.5.0", false),
        ]
    }
    
    // Other method implementations...
}
```

## 3. Plugin Lifecycle Management

Understanding the plugin lifecycle is essential for proper resource management and integration.

### Initialization

The `init` method is called once when the plugin is loaded and should:
- Set up resources and state
- Register with application systems
- Initialize configuration
- Prepare for operation

```rust
fn init(&self, app: &mut crate::kernel::bootstrap::Application) -> std::result::Result<(), PluginSystemError> {
    // Initialize resources
    // Register with services
    // Load configuration
    println!("Plugin initialized");
    Ok(())
}
```

### Preflight Check

The `preflight_check` method verifies that the plugin can operate correctly in the current environment:
- Validates prerequisites
- Checks for required resources
- Verifies configuration
- Ensures compatibility with the current application state

```rust
async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginSystemError> {
    // Check for required services
    // Validate configuration values
    // Verify resource availability
    
    if !requirements_met {
        return Err(PluginSystemError::PreflightCheckFailed{
            plugin_id: self.name().to_string(),
            message: "Required service not available".to_string()
        });
    }
    
    Ok(())
}
```

### Shutdown

The `shutdown` method is called when the plugin is being unloaded and should:
- Release all resources
- Save any persistent state
- Deregister from application services
- Ensure clean termination

```rust
fn shutdown(&self) -> std::result::Result<(), PluginSystemError> {
    // Save state
    // Release resources
    // Deregister from services
    println!("Plugin shutdown complete");
    Ok(())
}
```

## 4. FFI Interface Implementation

The FFI interface bridges between the Rust plugin and the host application, requiring careful attention to memory safety and error handling.

### VTable Creation

The plugin must provide a VTable with function pointers for all operations:

```rust
#[no_mangle]
pub extern "C" fn _plugin_init() -> *mut PluginVTable {
    // Use catch_unwind to prevent panics from crossing FFI boundary
    let result = panic::catch_unwind(|| {
        // Create plugin instance
        let plugin_instance = Box::new(MyPlugin { _marker: 0 });
        
        // Create VTable with all function pointers
        let vtable = PluginVTable {
            instance: Box::into_raw(plugin_instance) as *mut c_void,
            destroy: ffi_destroy,
            name: ffi_get_name,
            free_name: ffi_free_name,
            // ... other function pointers
        };
        
        Box::into_raw(Box::new(vtable))
    });
    
    match result {
        Ok(ptr) => ptr,
        Err(_) => {
            eprintln!("Panic during plugin initialization!");
            ptr::null_mut()
        }
    }
}
```

### Memory Management

Careful memory management is essential for FFI operations:

1. **String Handling**:
   ```rust
   extern "C" fn ffi_get_name(instance: *const c_void) -> *const c_char {
       let plugin = unsafe { &*(instance as *const MyPlugin) };
       match CString::new(plugin.name()) {
           Ok(s) => s.into_raw(),  // Transfer ownership to host
           Err(_) => ptr::null(),
       }
   }
   
   extern "C" fn ffi_free_name(name_ptr: *mut c_char) {
       if !name_ptr.is_null() {
           unsafe {
               let _ = CString::from_raw(name_ptr);  // Reclaim ownership and drop
           }
       }
   }
   ```

2. **Vector/Slice Handling**:
   ```rust
   extern "C" fn ffi_get_dependencies(instance: *const c_void) -> FfiSlice<FfiPluginDependency> {
       // Convert Vec to FfiSlice, transferring ownership
       let ptr = ffi_deps.as_mut_ptr();
       let len = ffi_deps.len();
       std::mem::forget(ffi_deps);  // Prevent Vec from dropping
       
       FfiSlice { ptr, len }
   }
   
   extern "C" fn ffi_free_dependencies(slice: FfiSlice<FfiPluginDependency>) {
       if !slice.ptr.is_null() {
           unsafe {
               // Reconstruct Vec and let it drop
               let deps = Vec::from_raw_parts(slice.ptr, slice.len, slice.len);
               // Free any inner resources
               for dep in deps {
                   // Free strings, etc.
               }
           }
       }
   }
   ```

## 5. Safety and Error Handling

Robust safety practices are critical for reliable plugins.

### FFI Safety

1. **Prevent Zero-Sized Types**:
   ```rust
   struct MyPlugin {
       _marker: u8,  // Ensure non-zero size for FFI safety
   }
   ```

2. **Panic Prevention**:
   ```rust
   let result = panic::catch_unwind(|| {
       // Plugin initialization code
   });
   ```

3. **Null Checking**:
   ```rust
   if !instance.is_null() {
       // Safe to dereference
   }
   ```

4. **Memory Ownership**:
   - Use appropriate ownership transfer functions (`into_raw`, `from_raw`)
   - Provide matching allocation and deallocation functions
   - Avoid double-free or use-after-free scenarios

### Error Handling

1. **Typed Errors**:
   ```rust
   return Err(PluginSystemError::PreflightCheckFailed{
       plugin_id: self.name().to_string(),
       message: "Detailed error description".to_string()
   });
   ```

2. **FFI Error Reporting**:
   ```rust
   extern "C" fn ffi_init(instance: *mut c_void, app_ptr: *mut c_void) -> FfiResult {
       if instance.is_null() || app_ptr.is_null() {
           return FfiResult::Err;
       }
       
       // Operation code...
       FfiResult::Ok
   }
   ```

3. **Logging**:
   ```rust
   if let Err(e) = result {
       eprintln!("Error in plugin operation: {}", e);
       return FfiResult::Err;
   }
   ```

## 6. Integration with Framework Components

### Stage Manager Integration

Plugins can register custom stages for the application pipeline:

```rust
fn register_stages(&self, registry: &mut StageRegistry) -> std::result::Result<(), PluginSystemError> {
    // Create and register a custom stage
    let my_stage = Box::new(MyCustomStage::new());
    registry.register_stage(my_stage)?;
    Ok(())
}

fn required_stages(&self) -> Vec<StageRequirement> {
    vec![
        // Require a stage with ID "init" to run before this plugin
        StageRequirement::require("init"),
        // Optional dependency on "logger" stage
        StageRequirement::optional("logger"),
        // This plugin provides a "my_custom" stage
        StageRequirement::provide("my_custom"),
    ]
}
```

### Context Data Sharing

Plugins can access and modify shared context data:

```rust
async fn preflight_check(&self, _context: &StageContext) -> std::result::Result<(), PluginSystemError> {
    // Read from context
    if let Some(config) = context.get_data::<Config>("app_config") {
        // Use configuration
    }
    
    // Write to context
    context.set_data("my_plugin_result", MyResult { value: 42 })?;
    
    Ok(())
}
```

### Event System Integration

Plugins can subscribe to and publish events:

```rust
fn init(&self, app: &mut crate::kernel::bootstrap::Application) -> std::result::Result<(), PluginSystemError> {
    // Get event manager
    let event_mgr = app.get_component::<EventManager>()?;
    
    // Subscribe to events
    event_mgr.subscribe("app_event", Box::new(|event| {
        println!("Received event: {:?}", event);
    }))?;
    
    // Publish an event
    event_mgr.publish(Event::new("plugin_loaded", self.name())?)?;
    
    Ok(())
}
```

## 7. Testing Strategies

### Unit Testing

Test individual plugin components:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_compatibility() {
        let plugin = MyPlugin { _marker: 0 };
        let versions = plugin.compatible_api_versions();
        
        assert!(!versions.is_empty());
        assert!(versions[0].is_compatible_with("0.1.5"));
        assert!(!versions[0].is_compatible_with("0.2.0"));
    }
}
```

### Integration Testing

Test the plugin with a mock application environment:

```rust
#[tokio::test]
async fn test_plugin_lifecycle() {
    // Create mock application
    let mut app = MockApplication::new();
    
    // Create plugin
    let plugin = MyPlugin { _marker: 0 };
    
    // Test initialization
    plugin.init(&mut app).expect("Init should succeed");
    
    // Test preflight check
    let context = StageContext::new();
    plugin.preflight_check(&context).await.expect("Preflight should succeed");
    
    // Test shutdown
    plugin.shutdown().expect("Shutdown should succeed");
}
```

### FFI Testing

Test the FFI interface with raw pointer operations:

```rust
#[test]
fn test_ffi_interface() {
    unsafe {
        // Get VTable
        let vtable_ptr = _plugin_init();
        assert!(!vtable_ptr.is_null());
        let vtable = &*vtable_ptr;
        
        // Test name function
        let name_ptr = (vtable.name)(vtable.instance);
        assert!(!name_ptr.is_null());
        let name = CStr::from_ptr(name_ptr).to_str().unwrap();
        assert_eq!(name, "MyPlugin");
        
        // Free resources
        (vtable.free_name)(name_ptr as *mut c_char);
        (vtable.destroy)(vtable.instance);
        Box::from_raw(vtable_ptr);
    }
}
```

## 8. Best Practices

### Plugin Structure

1. **Separate Logic from FFI**:
   - Keep core plugin logic in separate modules
   - Use FFI layer only for interface translation
   - Keep FFI functions simple and focused on memory management

2. **Resource Management**:
   - Initialize resources in `init` method
   - Clean up resources in `shutdown` method
   - Use RAII patterns for automatic cleanup
   - Implement Drop for plugin types if needed

3. **Error Handling**:
   - Provide detailed error messages
   - Use appropriate error types
   - Handle all potential failure points
   - Log errors for diagnostics

### FFI Safety

1. **Pointer Safety**:
   - Always check pointers before dereferencing
   - Ensure proper ownership transfer
   - Use `catch_unwind` at FFI boundaries
   - Avoid complex logic in FFI functions

2. **Memory Management**:
   - Provide pairs of allocation/deallocation functions
   - Document ownership transfer clearly
   - Use consistent patterns for memory handling
   - Consider using helper functions for common patterns

3. **Type Safety**:
   - Use non-zero sized types for FFI
   - Implement proper Debug/Display for error reporting
   - Use strongly typed interfaces when possible
   - Validate all inputs from FFI

## 9. Common Pitfalls

1. **Zero-Sized Types in FFI**: Can lead to undefined behavior with raw pointers
2. **Missing Resource Cleanup**: Causes memory leaks and resource exhaustion
3. **Panic Across FFI**: Results in undefined behavior if not caught
4. **Incorrect String Handling**: Missing null termination or encoding issues
5. **Double-Free**: Attempting to free memory multiple times
6. **Use-After-Free**: Accessing memory after it's been deallocated
7. **Insufficient Error Handling**: Not checking for failures in FFI functions
8. **Thread Safety Issues**: Not accounting for concurrent plugin access

## 10. Debugging Plugins

1. **Logging**:
   - Add detailed logging throughout the plugin
   - Log entry/exit from key functions
   - Include context data in logs
   - Use different log levels appropriately

2. **Debugging Tools**:
   - Use `RUST_BACKTRACE=1` for stack traces
   - Consider enabling debug symbols in release builds
   - Use debuggers that support FFI (GDB, LLDB)
   - Consider adding debug hooks and assertions

3. **Common Issues**:
   - Plugin not found: Check search paths and library name
   - Symbol not found: Check entry point name (`_plugin_init`)
   - Segmentation fault: Check pointer validity and memory management
   - Unexpected behavior: Verify API version compatibility

## Conclusion

Developing plugins for the Gini framework requires careful attention to the Plugin trait implementation, FFI safety, and proper lifecycle management. By following these guidelines and the example patterns from the compatibility check plugin, you can create robust, reliable plugins that extend the framework's functionality while maintaining system stability and security.

This guide should be considered alongside the framework's API documentation and the example plugin implementations provided with the framework. As the framework evolves, plugin development patterns may be refined and expanded.