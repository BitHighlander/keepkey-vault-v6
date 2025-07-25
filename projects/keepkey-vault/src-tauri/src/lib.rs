// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

mod commands;
use std::sync::Arc;
use tauri::{Manager};

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
            
            // Get device queue manager to pass to USB monitoring
            let device_queue_manager = app.state::<commands::DeviceQueueManager>().inner().clone();
            
            // Start USB monitoring in background
            tauri::async_runtime::spawn(async move {
                // Initialize the USB monitoring with proper event emission
                if let Err(e) = start_usb_monitoring(app_handle, device_queue_manager).await {
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
            commands::frontend_ready,
            commands::is_first_time_install,
            commands::is_onboarded,
            commands::set_onboarding_completed,
            commands::debug_onboarding_state,
            commands::get_preference,
            commands::set_preference,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Start USB monitoring with proper event emission
async fn start_usb_monitoring(app_handle: tauri::AppHandle, device_queue_manager: Arc<tokio::sync::Mutex<std::collections::HashMap<String, keepkey_rust::device_queue::DeviceQueueHandle>>>) -> Result<(), String> {
    log::info!("🔍 Starting USB device monitoring for connect/disconnect events...");
    
    // Monitor device connections in a loop
    tokio::spawn(async move {
        let mut last_devices = std::collections::HashSet::new();
        
        loop {
            // Get current devices with full device info
            let current_device_list = keepkey_rust::features::list_connected_devices();
            let current_devices: std::collections::HashSet<String> = current_device_list
                .iter()
                .filter(|d| d.is_keepkey)
                .map(|d| d.unique_id.clone())
                .collect();
            
            // Check for new connections
            for device_id in &current_devices {
                if !last_devices.contains(device_id) {
                    log::info!("🔌 Device connected: {}", device_id);
                    
                    // Find the full device info for this connected device
                    if let Some(device) = current_device_list.iter().find(|d| &d.unique_id == device_id) {
                        // Emit device:connected event with full device info using emit_or_queue_event
                        let device_payload = serde_json::json!({
                            "unique_id": device.unique_id,
                            "name": device.name,
                            "manufacturer": device.manufacturer,
                            "vid": device.vid,
                            "pid": device.pid,
                            "is_keepkey": device.is_keepkey
                        });
                        
                        if let Err(e) = commands::emit_or_queue_event(&app_handle, "device:connected", device_payload).await {
                            log::error!("❌ Failed to emit/queue device:connected event: {}", e);
                        } else {
                            log::info!("📡 Successfully emitted/queued device:connected event for {}", device_id);
                        }
                        
                        // Also emit a status update
                        let status_payload = serde_json::json!({
                            "status": format!("Device connected: {}", device_id)
                        });
                        
                        if let Err(e) = commands::emit_or_queue_event(&app_handle, "status:update", status_payload).await {
                            log::error!("❌ Failed to emit/queue status update: {}", e);
                        }
                        
                        // After a short delay, check device readiness and onboarding status
                        let device_id_clone = device_id.clone();
                        let app_handle_clone = app_handle.clone();
                        let queue_manager_clone = device_queue_manager.clone();
                        
                        tokio::spawn(async move {
                            // Wait a moment for device to be fully ready
                            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                            
                            if let Err(e) = commands::check_device_ready_and_onboarding(&device_id_clone, &app_handle_clone, &queue_manager_clone).await {
                                log::error!("❌ Failed to check device readiness for {}: {}", device_id_clone, e);
                            }
                        });
                    }
                }
            }
            
            // Check for disconnections
            for device_id in &last_devices {
                if !current_devices.contains(device_id) {
                    log::info!("🔌 Device disconnected: {}", device_id);
                    
                    // Emit device:disconnected event using emit_or_queue_event
                    let disconnect_payload = serde_json::json!({
                        "device_id": device_id
                    });
                    
                    if let Err(e) = commands::emit_or_queue_event(&app_handle, "device:disconnected", disconnect_payload).await {
                        log::error!("❌ Failed to emit/queue device:disconnected event: {}", e);
                    } else {
                        log::info!("📡 Successfully emitted/queued device:disconnected event for {}", device_id);
                    }
                    
                    // Also emit a status update
                    let status_payload = serde_json::json!({
                        "status": format!("Device disconnected: {}", device_id)
                    });
                    
                    if let Err(e) = commands::emit_or_queue_event(&app_handle, "status:update", status_payload).await {
                        log::error!("❌ Failed to emit/queue status update: {}", e);
                    }
                }
            }
            
            last_devices = current_devices;
            
            // Poll every 500ms for device changes
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });
    
    Ok(())
}
