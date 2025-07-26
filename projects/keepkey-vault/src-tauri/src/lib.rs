// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

mod commands;
mod device;

use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use tauri::{Manager};
use keepkey_db::Database;
use keepkey_rust;

#[derive(Debug, Clone)]
struct DeviceConnectionInfo {
    device: keepkey_rust::friendly_usb::FriendlyUsbDevice,
    disconnected_at: Option<Instant>,
}

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
            
            // Initialize database
            log::info!("🗄️ Initializing database...");
            let database = tauri::async_runtime::block_on(async {
                Database::new().await
            }).map_err(|e| {
                log::error!("Failed to initialize database: {}", e);
                e
            })?;
            
            app.manage(Arc::new(database));
            
            // Initialize device queue manager (like v5)
            let device_queue_manager = Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::<String, keepkey_rust::device_queue::DeviceQueueHandle>::new()
            ));
            app.manage(device_queue_manager);

            // Initialize USB management system for connect/disconnect events
            log::info!("🔌 Initializing USB device management...");
            
            // Use the USB manager from keepkey_rust to get proper event handling
            let app_handle = app.handle().clone();
            
            // Get device queue manager and database to pass to USB monitoring
            let device_queue_manager = app.state::<commands::DeviceQueueManager>().inner().clone();
            let database = app.state::<Arc<Database>>().inner().clone();
            
            // Start USB monitoring in background
            tauri::async_runtime::spawn(async move {
                // Initialize the USB monitoring with proper event emission
                if let Err(e) = start_usb_monitoring(app_handle, device_queue_manager, database).await {
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
            // Device commands
            commands::device::get_features::get_features,
            commands::device::get_connected_devices::get_connected_devices,
            commands::device::get_device_status::get_device_status,
            commands::device::check_device_bootloader::check_device_bootloader,
            commands::device::get_devices_needing_setup::get_devices_needing_setup,
            commands::device::reset_usb_subsystem,
            // Update commands  
            device::updates::update_device_bootloader,
            device::updates::update_device_firmware,
            // Event and config commands
            commands::events::frontend_ready,
            commands::config::is_first_time_install,
            commands::config::is_onboarded,
            commands::config::set_onboarding_completed,
            commands::config::debug_onboarding_state,
            commands::config::get_preference,
            commands::config::set_preference,
            // Legacy commands (TODO: move to appropriate modules)
            register_device,
            get_device_registry,
            get_device_from_registry,
            update_device_setup_step,
            mark_device_setup_complete,
            device_needs_setup,
            get_incomplete_setup_devices,
            reset_device_setup,
            get_device_eth_address,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Legacy command stubs that need to be moved to proper modules
#[tauri::command]
async fn register_device() -> Result<(), String> { Ok(()) }
#[tauri::command]
async fn get_device_registry() -> Result<Vec<String>, String> { Ok(vec![]) }
#[tauri::command]
async fn get_device_from_registry() -> Result<Option<String>, String> { Ok(None) }
#[tauri::command]
async fn update_device_setup_step() -> Result<(), String> { Ok(()) }
#[tauri::command]
async fn mark_device_setup_complete() -> Result<(), String> { Ok(()) }
#[tauri::command]
async fn device_needs_setup() -> Result<bool, String> { Ok(false) }
#[tauri::command]
async fn get_incomplete_setup_devices() -> Result<Vec<String>, String> { Ok(vec![]) }
#[tauri::command]
async fn reset_device_setup() -> Result<(), String> { Ok(()) }
#[tauri::command]
async fn get_device_eth_address() -> Result<String, String> { Ok("0x".to_string()) }

/// Start USB monitoring with proper event emission
async fn start_usb_monitoring(
    app_handle: tauri::AppHandle, 
    device_queue_manager: Arc<tokio::sync::Mutex<std::collections::HashMap<String, keepkey_rust::device_queue::DeviceQueueHandle>>>,
    database: Arc<Database>
) -> Result<(), String> {
    log::info!("🔍 Starting USB device monitoring for connect/disconnect events...");
    
    // Monitor device connections in a loop
    tokio::spawn(async move {
        let mut last_devices = std::collections::HashSet::new();
        let mut known_devices: HashMap<String, DeviceConnectionInfo> = HashMap::new();
        const GRACE_PERIOD_SECS: u64 = 10;
        
        loop {
            // Get current devices with full device info
            let current_device_list = keepkey_rust::features::list_connected_devices();
            let current_devices: std::collections::HashSet<String> = current_device_list
                .iter()
                .filter(|d| d.is_keepkey)
                .map(|d| d.unique_id.clone())
                .collect();
            
            // Process each currently connected device
            for device in &current_device_list {
                if !device.is_keepkey {
                    continue;
                }
                
                let device_key = device.serial_number.as_ref()
                    .unwrap_or(&device.unique_id)
                    .clone();
                
                if let Some(info) = known_devices.get_mut(&device_key) {
                    // Device was known - check if it was temporarily disconnected
                    if info.disconnected_at.is_some() {
                        log::info!("🔄 Device {} reconnected (was temporarily disconnected)", device_key);
                        info.disconnected_at = None;
                        info.device = device.clone();
                        
                        // Clear temporary disconnection flag using unique_id for consistency
                        let _ = commands::clear_temporary_disconnection(&device.unique_id);
                        
                        // Check if ANY device in recovery flow might match this reconnected device
                        let mut found_recovery_device = false;
                        let mut recovery_device_id = String::new();
                        
                        // Check direct match first
                        if commands::is_device_in_recovery_flow(&device.unique_id) {
                            found_recovery_device = true;
                            recovery_device_id = device.unique_id.clone();
                        } else {
                            // Check if this device might be the same as any device in recovery flow
                            // This handles cases where device ID changes after firmware update
                            if let Ok(recovery_flows) = commands::get_all_recovery_flow_devices() {
                                for recovery_id in recovery_flows {
                                    if commands::are_devices_potentially_same(&device.unique_id, &recovery_id) {
                                        log::info!("🔗 Device {} appears to be {} returning from recovery flow", device.unique_id, recovery_id);
                                        found_recovery_device = true;
                                        // Set up alias for the new ID
                                        let _ = commands::add_recovery_device_alias(&device.unique_id, &recovery_id);
                                        recovery_device_id = recovery_id;
                                        break;
                                    }
                                }
                            }
                        }
                        
                        // If device was in recovery flow and has now reconnected, wait for it to be fully ready
                        // This ensures the device is stable after firmware update reboot
                        if found_recovery_device {
                            log::info!("🛡️ Device {} reconnected after recovery flow (original: {}). Waiting for device to be fully ready...", 
                                device.unique_id, recovery_device_id);
                            let device_id_for_cleanup = recovery_device_id;
                            let current_device_id = device.unique_id.clone();
                            let app_handle_clone = app_handle.clone();
                            let queue_manager_clone = device_queue_manager.clone();
                            
                            tokio::spawn(async move {
                                // Try to get device features for up to 30 seconds
                                let mut attempts = 0;
                                let max_attempts = 30;
                                let mut device_ready = false;
                                
                                while attempts < max_attempts && !device_ready {
                                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                                    attempts += 1;
                                    
                                    // Try to get device features to confirm it's ready
                                    // Use the current device ID for communication, not the recovery ID
                                    match commands::device::get_or_create_device_queue(
                                        &current_device_id,
                                        &queue_manager_clone
                                    ).await {
                                        Ok(queue_handle) => {
                                            if let Ok(features) = queue_handle.get_features().await {
                                                let converted_features = commands::device::get_features::convert_features_to_device_features(features);
                                                log::info!("✅ Device {} is ready after {} seconds (firmware: {})", 
                                                    current_device_id, attempts, converted_features.version);
                                                device_ready = true;
                                                
                                                // Emit device ready event with the original device ID for UI consistency
                                                let _ = commands::emit_or_queue_event(
                                                    &app_handle_clone,
                                                    "device:ready-after-update",
                                                    serde_json::json!({
                                                        "deviceId": device_id_for_cleanup.clone(),
                                                        "firmwareVersion": converted_features.version
                                                    })
                                                ).await;
                                            }
                                        }
                                        Err(_) => {
                                            // Device not ready yet, log periodically
                                            if attempts % 5 == 0 {
                                                log::info!("⏳ Still waiting for device {} to be ready... ({}/{})", 
                                                    current_device_id, attempts, max_attempts);
                                            }
                                        }
                                    }
                                }
                                
                                if device_ready {
                                    // Clear recovery flow only after device is truly ready
                                    if let Err(e) = commands::unmark_device_in_recovery_flow(&device_id_for_cleanup) {
                                        log::error!("Failed to unmark device from recovery flow after reconnection: {}", e);
                                    } else {
                                        log::info!("✅ Device {} recovery flow cleared after successful initialization", device_id_for_cleanup);
                                    }
                                } else {
                                    log::error!("❌ Device {} failed to become ready after {} seconds", device_id_for_cleanup, max_attempts);
                                    
                                    // Emit event suggesting USB reset
                                    let _ = commands::emit_or_queue_event(
                                        &app_handle_clone,
                                        "device:recovery-failed",
                                        serde_json::json!({
                                            "deviceId": device_id_for_cleanup.clone(),
                                            "reason": "Device failed to initialize after firmware update",
                                            "suggestAction": "reset_usb"
                                        })
                                    ).await;
                                    
                                    // Still clear recovery flow to avoid stuck state
                                    let _ = commands::unmark_device_in_recovery_flow(&device_id_for_cleanup);
                                }
                            });
                        }
                        
                        // Emit reconnection event
                        let _ = commands::emit_or_queue_event(
                            &app_handle,
                            "device:reconnected",
                            serde_json::json!({
                                "deviceId": device.unique_id,
                                "wasTemporary": true
                            })
                        ).await;
                    }
                } else {
                // New device - add to known devices
                known_devices.insert(device_key.clone(), DeviceConnectionInfo {
                    device: device.clone(),
                    disconnected_at: None,
                });
                
                // Check if this might be a device returning from temporary disconnection
                for (known_key, known_info) in known_devices.iter() {
                    if known_key != &device_key && 
                       known_info.disconnected_at.is_some() &&
                       commands::are_devices_potentially_same(known_key, &device_key) {
                        log::info!("🔄 Device {} might be {} returning from disconnection", device_key, known_key);
                        let _ = commands::add_recovery_device_alias(&device_key, known_key);
                        break;
                    }
                }
                    
                    // Register device in the database
                    let serial_number = device.serial_number.as_deref();
                    let features_json = serde_json::to_string(&device).ok();
                    
                    if let Err(e) = database.register_device(&device.unique_id, serial_number, features_json.as_deref()).await {
                        log::error!("Failed to register device in registry: {}", e);
                    } else {
                        log::info!("📝 Registered device in registry: {}", device.unique_id);
                    }
                    
                    // Check if device needs setup
                    match database.device_needs_setup(&device.unique_id).await {
                        Ok(needs_setup) => {
                            if needs_setup {
                                log::info!("⚠️  Device {} needs setup - will emit setup-required event", device.unique_id);
                                
                                // Emit setup-required event
                                if let Err(e) = commands::emit_or_queue_event(
                                    &app_handle,
                                    "device:setup-required",
                                    serde_json::json!({
                                        "device_id": device.unique_id,
                                        "device_name": device.name,
                                        "serial_number": device.serial_number
                                    })
                                ).await {
                                    log::error!("Failed to emit setup-required event: {}", e);
                                }
                            } else {
                                log::info!("✅ Device {} setup is complete", device.unique_id);
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to check setup status for device {}: {}", device.unique_id, e);
                        }
                    }
                    
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
                        log::info!("📡 Successfully emitted/queued device:connected event for {}", device.unique_id);
                    }
                    
                    // Also emit a status update
                    let status_payload = serde_json::json!({
                        "status": format!("Device connected: {}", device.unique_id)
                    });
                    
                    if let Err(e) = commands::emit_or_queue_event(&app_handle, "status:update", status_payload).await {
                        log::error!("❌ Failed to emit/queue status update: {}", e);
                    }
                }
            }
            
            // Check for devices that are no longer present and apply grace period
            known_devices.retain(|device_key, info| {
                // Check if this device is still connected
                let still_connected = current_device_list.iter().any(|d| {
                    d.is_keepkey && (
                        d.serial_number.as_ref().unwrap_or(&d.unique_id) == device_key ||
                        &d.unique_id == &info.device.unique_id
                    )
                });
                
                if !still_connected {
                    if info.disconnected_at.is_none() {
                        // Device just disconnected - start grace period
                        log::info!("🔌❓ Device {} disconnected - starting grace period", info.device.unique_id);
                        info.disconnected_at = Some(Instant::now());
                        
                        // Mark device as temporarily disconnected for queue management
                        let _ = commands::mark_device_temporarily_disconnected(&info.device.unique_id);
                        
                        // Clear any existing device queue to force recreation on reconnection
                        // This is necessary because USB transports become invalid after disconnection
                        if let Some(state) = app_handle.try_state::<commands::DeviceQueueManager>() {
                            let queue_manager_arc = state.inner().clone();
                            let device_id = info.device.unique_id.clone();
                            let in_recovery = commands::is_device_in_recovery_flow(&info.device.unique_id);
                            tokio::spawn(async move {
                                let mut manager = queue_manager_arc.lock().await;
                                if manager.remove(&device_id).is_some() {
                                    if in_recovery {
                                        log::info!("🗑️ Removed device queue for firmware update reboot: {}", device_id);
                                    } else {
                                        log::info!("🗑️ Removed stale device queue for disconnected device: {}", device_id);
                                    }
                                }
                            });
                        }
                        
                        return true; // Keep in list during grace period
                    } else {
                        // Check if grace period has expired
                        let disconnected_duration = info.disconnected_at.unwrap().elapsed();
                        if disconnected_duration.as_secs() >= GRACE_PERIOD_SECS {
                            // Grace period expired - emit disconnection and remove
                            log::info!("🔌❌ Device {} disconnected permanently after grace period", info.device.unique_id);
                            
                            // Emit device:disconnected event
                            let disconnect_payload = serde_json::json!({
                                "device_id": info.device.unique_id
                            });
                            
                            tokio::spawn({
                                let app_handle = app_handle.clone();
                                let device_id = info.device.unique_id.clone();
                                let device_key = device_key.clone();
                                async move {
                                    if let Err(e) = commands::emit_or_queue_event(&app_handle, "device:disconnected", disconnect_payload).await {
                                        log::error!("❌ Failed to emit/queue device:disconnected event: {}", e);
                                    } else {
                                        log::info!("📡 Successfully emitted/queued device:disconnected event for {}", device_id);
                                    }
                                    
                                    // Clear temporary disconnection tracking
                                    let _ = commands::clear_temporary_disconnection(&device_key);
                                    
                                    // Also emit a status update
                                    let status_payload = serde_json::json!({
                                        "status": format!("Device disconnected: {}", device_id)
                                    });
                                    
                                    if let Err(e) = commands::emit_or_queue_event(&app_handle, "status:update", status_payload).await {
                                        log::error!("❌ Failed to emit/queue status update: {}", e);
                                    }
                                }
                            });
                            
                            return false; // Remove from tracking
                        }
                    }
                }
                
                true // Keep device in tracking
            });
            
            last_devices = current_devices;
            
            // Poll every 500ms for device changes
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });
    
    Ok(())
}
