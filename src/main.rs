mod kernel;
mod plugin_system;
mod stage_manager;
mod storage;
mod event;
mod ui_bridge;
mod utils;

use crate::kernel::bootstrap::Application;

#[tokio::main]
async fn main() {
    println!("OSX-Forge: QEMU/KVM Deployment System");
    println!("Initializing application...");
    
    // Create and initialize the application
    let mut app = match Application::new(None) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Failed to initialize application: {}", e);
            return;
        }
    };
    
    // Run the application
    let run_result = app.run().await;
    if let Err(e) = run_result {
        eprintln!("Application error: {}", e);
    }
    
    println!("Shutting down application...");
}
