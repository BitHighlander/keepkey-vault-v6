// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

mod commands;
use std::sync::Arc;
use tauri::{Manager, Emitter};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging first
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();
    
    log::info!("🚀 KeepKey Vault starting up...");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            log::info!("🔧 Setting up KeepKey Vault application...");
            
            // Initialize device queue manager
            let device_queue_manager = Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::<String, keepkey_rust::device_queue::DeviceQueueHandle>::new()
            ));
            app.manage(device_queue_manager);

            // Initialize USB management system for connect/disconnect events
            log::info!("🔌 Initializing USB device management...");
            
            // Use the USB manager from keepkey_rust to get proper event handling
            let app_handle = app.handle().clone();
            
            // Start USB monitoring in background
            tauri::async_runtime::spawn(async move {
                // Initialize the USB monitoring with proper event emission
                if let Err(e) = start_usb_monitoring(app_handle).await {
                    log::error!("❌ Failed to start USB monitoring: {}", e);
                } else {
                    log::info!("✅ USB monitoring started successfully");
                }
            });

            log::info!("✅ KeepKey Vault setup completed");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::get_features,
            commands::get_connected_devices,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Start USB monitoring with proper event emission
async fn start_usb_monitoring(app_handle: tauri::AppHandle) -> Result<(), String> {
    log::info!("🔍 Starting USB device monitoring for connect/disconnect events...");
    
    // Monitor device connections in a loop
    tokio::spawn(async move {
        let mut last_devices = std::collections::HashSet::new();
        
        loop {
            // Get current devices
            let current_devices: std::collections::HashSet<String> = keepkey_rust::features::list_connected_devices()
                .into_iter()
                .filter(|d| d.is_keepkey)
                .map(|d| d.unique_id.clone())
                .collect();
            
            // Check for new connections
            for device_id in &current_devices {
                if !last_devices.contains(device_id) {
                    log::info!("🔌 Device connected: {}", device_id);
                    let _ = app_handle.emit("device:connected", device_id);
                }
            }
            
            // Check for disconnections
            for device_id in &last_devices {
                if !current_devices.contains(device_id) {
                    log::info!("🔌 Device disconnected: {}", device_id);
                    let _ = app_handle.emit("device:disconnected", device_id);
                }
            }
            
            last_devices = current_devices;
            
            // Poll every 500ms for device changes
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });
    
    Ok(())
}
