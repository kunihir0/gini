mod kernel;
mod plugin_system;
mod stage_manager;
mod storage;
mod event;
mod ui_bridge;
mod utils;

use crate::kernel::bootstrap::Application;

fn main() {
    println!("OSX-Forge: QEMU/KVM Deployment System");
    println!("Initializing application...");
    
    // Create and initialize the application
    let mut app = match Application::new() {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Failed to initialize application: {}", e);
            return;
        }
    };
    
    // Run the application
    if let Err(e) = app.run() {
        eprintln!("Application error: {}", e);
    }
    
    println!("Shutting down application...");
}
