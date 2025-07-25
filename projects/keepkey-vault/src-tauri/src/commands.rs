use tauri::State;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use keepkey_rust::{
    device_queue::{DeviceQueueFactory, DeviceQueueHandle},
    features::DeviceFeatures,
    index_db::IndexDb,
};
use tauri::{AppHandle, Emitter};
use serde::{Serialize, Deserialize};

// Type alias for the device queue manager
pub type DeviceQueueManager = Arc<Mutex<HashMap<String, DeviceQueueHandle>>>;

// Add frontend readiness state and queued events
lazy_static::lazy_static! {
    static ref FRONTEND_READY_STATE: Arc<tokio::sync::RwLock<FrontendReadyState>> = Arc::new(tokio::sync::RwLock::new(FrontendReadyState::default()));
    // One-time initialization flag to prevent duplicate ready signals
    static ref FRONTEND_READY_ONCE: Arc<tokio::sync::Mutex<bool>> = Arc::new(tokio::sync::Mutex::new(false));
}

#[derive(Debug, Clone)]
struct FrontendReadyState {
    is_ready: bool,
    queued_events: Vec<QueuedEvent>,
}

impl Default for FrontendReadyState {
    fn default() -> Self {
        Self {
            is_ready: false,
            queued_events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueuedEvent {
    event_name: String,
    payload: serde_json::Value,
    timestamp: u64,
}

/// Get features for a specific device
#[tauri::command]
pub async fn get_features(
    device_id: String,
    queue_manager: State<'_, DeviceQueueManager>,
) -> Result<DeviceFeatures, String> {
    println!("🔍 Getting features for device: {}", device_id);
    
    // Get or create device queue handle
    let queue_handle = get_or_create_device_queue(&device_id, &queue_manager).await?;
    
    // Fetch features through the queue
    match queue_handle.get_features().await {
        Ok(features) => {
            println!("✅ Successfully got features for device: {}", device_id);
            Ok(convert_features_to_device_features(features))
        }
        Err(e) => {
            println!("❌ Failed to get features for device {}: {}", device_id, e);
            Err(format!("Failed to get features: {}", e))
        }
    }
}

/// Get connected devices
#[tauri::command]
pub async fn get_connected_devices() -> Result<Vec<serde_json::Value>, String> {
    let devices = keepkey_rust::features::list_connected_devices();
    
    Ok(devices.into_iter()
        .filter(|d| d.is_keepkey)
        .map(|device| {
            serde_json::json!({
                "device_id": device.unique_id,
                "name": device.name,
                "features": null,
            })
        })
        .collect())
}

/// Centralized function to get or create device queue
/// This is THE ONLY place where DeviceQueueFactory::spawn_worker should be called
pub async fn get_or_create_device_queue(
    device_id: &str, 
    queue_manager: &DeviceQueueManager
) -> Result<DeviceQueueHandle, String> {
    // First check if we already have a handle
    {
        let manager = queue_manager.lock().await;
        if let Some(handle) = manager.get(device_id) {
            println!("♻️  Reusing existing queue handle for device: {}", device_id);
            return Ok(handle.clone());
        }
    }
    
    // Find the device by ID
    let devices = keepkey_rust::features::list_connected_devices();
    let device_info = devices
        .iter()
        .find(|d| d.unique_id == device_id)
        .ok_or_else(|| format!("Device {} not found", device_id))?;

    // Create new worker with proper locking to prevent race conditions
    let mut manager = queue_manager.lock().await;
    
    // Double-check after acquiring lock (race condition protection)
    if let Some(handle) = manager.get(device_id) {
        println!("♻️  Reusing existing queue handle for device (after lock): {}", device_id);
        return Ok(handle.clone());
    }
    
    // Spawn a new device worker - this happens ONLY when truly needed
    println!("🚀 Creating new device worker for: {}", device_id);
    let handle = DeviceQueueFactory::spawn_worker(device_id.to_string(), device_info.clone());
    manager.insert(device_id.to_string(), handle.clone());
    
    Ok(handle)
}

/// Convert raw Features to DeviceFeatures
fn convert_features_to_device_features(features: keepkey_rust::messages::Features) -> DeviceFeatures {
    DeviceFeatures {
        vendor: Some(features.vendor.unwrap_or_default()),
        label: Some(features.label.unwrap_or_default()),
        model: Some(features.model.unwrap_or_default()),
        firmware_variant: features.firmware_variant.clone(),
        device_id: Some(features.device_id.unwrap_or_default()),
        language: Some(features.language.unwrap_or_default()),
        bootloader_mode: features.bootloader_mode.unwrap_or(false),
        version: format!("{}.{}.{}", 
            features.major_version.unwrap_or(0),
            features.minor_version.unwrap_or(0), 
            features.patch_version.unwrap_or(0)
        ),
        firmware_hash: features.firmware_hash.clone().map(hex::encode),
        bootloader_hash: features.bootloader_hash.clone().map(hex::encode),
        bootloader_version: None, // Derived from hash, not directly available
        initialized: features.initialized.unwrap_or(false),
        imported: features.imported,
        no_backup: features.no_backup.unwrap_or(false),
        pin_protection: features.pin_protection.unwrap_or(false),
        pin_cached: features.pin_cached.unwrap_or(false),
        passphrase_protection: features.passphrase_protection.unwrap_or(false),
        passphrase_cached: features.passphrase_cached.unwrap_or(false),
        wipe_code_protection: features.wipe_code_protection.unwrap_or(false),
        auto_lock_delay_ms: features.auto_lock_delay_ms.map(|x| x as u64),
        policies: features.policies.into_iter()
            .map(|p| p.policy_name().to_string())
            .collect(),
    }
} 

/// Signal that the frontend is ready to receive events
#[tauri::command]
pub async fn frontend_ready(app: AppHandle) -> Result<(), String> {
    // Check if we've already processed the ready signal
    let mut already_ready = FRONTEND_READY_ONCE.lock().await;
    
    if *already_ready {
        log::debug!("🎯 Frontend ready signal already processed, ignoring duplicate call");
        return Ok(());
    }
    
    *already_ready = true;
    drop(already_ready); // Release lock before processing
    
    log::info!("🎯 Frontend ready signal received - enabling event emission");
    
    let mut state = FRONTEND_READY_STATE.write().await;
    state.is_ready = true;
    
    // Flush any queued events
    if !state.queued_events.is_empty() {
        log::info!("📦 Flushing {} queued events to frontend", state.queued_events.len());
        
        for event in state.queued_events.drain(..) {
            log::debug!("📡 Sending queued event: {} (queued at: {})", event.event_name, event.timestamp);
            if let Err(e) = app.emit(&event.event_name, &event.payload) {
                log::error!("❌ Failed to emit queued event {}: {}", event.event_name, e);
            }
        }
        
        log::info!("✅ All queued events have been sent to frontend");
    } else {
        log::debug!("✅ No queued events to flush");
    }
    
    Ok(())
}

/// Helper function to emit events (either immediately or queue them)
pub async fn emit_or_queue_event(app: &AppHandle, event_name: &str, payload: serde_json::Value) -> Result<(), String> {
    let state = FRONTEND_READY_STATE.read().await;
    
    if state.is_ready {
        // Frontend is ready, emit immediately
        app.emit(event_name, &payload)
            .map_err(|e| format!("Failed to emit event {}: {}", event_name, e))?;
        println!("📡 Emitted event: {}", event_name);
    } else {
        // Frontend not ready, queue the event
        drop(state); // Release read lock
        let mut state = FRONTEND_READY_STATE.write().await;
        
        let queued_event = QueuedEvent {
            event_name: event_name.to_string(),
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };
        
        state.queued_events.push(queued_event);
        println!("📋 Queued event: {} (total queued: {})", event_name, state.queued_events.len());
    }
    
    Ok(())
} 

/// Get the config directory path
fn get_config_dir() -> Result<std::path::PathBuf, String> {
    let home_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "Could not find home directory")?;
    
    let config_dir = std::path::PathBuf::from(home_dir).join(".keepkey");
    
    // Create directory if it doesn't exist
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    
    Ok(config_dir)
}

/// Get the config file path
fn get_config_file_path() -> Result<std::path::PathBuf, String> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("keepkey.json"))
}

/// Load configuration from file
fn load_config() -> Result<serde_json::Value, String> {
    let config_path = get_config_file_path()?;
    
    if !config_path.exists() {
        // Return default config if file doesn't exist
        return Ok(serde_json::json!({
            "language": "en",
            "isOnboarded": false,
            "theme": "dark",
            "notifications": true
        }));
    }
    
    let config_str = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    
    serde_json::from_str(&config_str)
        .map_err(|e| format!("Failed to parse config file: {}", e))
}

/// Save configuration to file
fn save_config(config: &serde_json::Value) -> Result<(), String> {
    let config_path = get_config_file_path()?;
    
    let config_str = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    
    std::fs::write(&config_path, config_str)
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    
    Ok(())
}

/// Check if this is the first time install
#[tauri::command]
pub async fn is_first_time_install() -> Result<bool, String> {
    let config = load_config()?;
    let is_onboarded = config.get("isOnboarded")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    Ok(!is_onboarded)
}

/// Check if user is onboarded
#[tauri::command]
pub async fn is_onboarded() -> Result<bool, String> {
    let config = load_config()?;
    let is_onboarded = config.get("isOnboarded")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    Ok(is_onboarded)
}

/// Mark onboarding as completed
#[tauri::command]
pub async fn set_onboarding_completed(app: AppHandle) -> Result<(), String> {
    let mut config = load_config()?;
    
    if let Some(obj) = config.as_object_mut() {
        obj.insert("isOnboarded".to_string(), serde_json::Value::Bool(true));
    }
    
    save_config(&config)?;
    log::info!("✅ Onboarding marked as completed");
    
    // Emit onboarding completion event
    if let Err(e) = emit_or_queue_event(&app, "onboarding:completed", serde_json::json!({
        "completed": true,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    })).await {
        log::error!("Failed to emit onboarding completion event: {}", e);
    }
    
    Ok(())
}

/// Debug onboarding state
#[tauri::command]
pub async fn debug_onboarding_state() -> Result<String, String> {
    let config = load_config()?;
    Ok(format!("Config: {}", serde_json::to_string_pretty(&config).unwrap_or_else(|_| "Unable to serialize".to_string())))
}

/// Get a preference value
#[tauri::command]
pub async fn get_preference(key: String) -> Result<Option<String>, String> {
    let config = load_config()?;
    
    let value = config.get(&key)
        .and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Bool(b) => Some(b.to_string()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        });
    
    Ok(value)
}

/// Set a preference value
#[tauri::command]
pub async fn set_preference(key: String, value: String) -> Result<(), String> {
    let mut config = load_config()?;
    if let Some(obj) = config.as_object_mut() {
        // Try to parse as different types
        let parsed_value = if value == "true" || value == "false" {
            serde_json::Value::Bool(value == "true")
        } else if let Ok(num) = value.parse::<i64>() {
            serde_json::Value::Number(serde_json::Number::from(num))
        } else {
            serde_json::Value::String(value)
        };
        obj.insert(key, parsed_value);
    }
    save_config(&config)?;
    Ok(())
}

// ============ Device Registry Commands ============

/// Register a device in the registry
#[tauri::command]
pub async fn register_device(
    device_id: String,
    serial_number: Option<String>,
    features: Option<String>,
) -> Result<(), String> {
    let db = IndexDb::open().map_err(|e| e.to_string())?;
    db.register_device(&device_id, serial_number.as_deref(), features.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Get all devices in the registry
#[tauri::command]
pub async fn get_device_registry() -> Result<Vec<serde_json::Value>, String> {
    let db = IndexDb::open().map_err(|e| e.to_string())?;
    db.get_device_registry().map_err(|e| e.to_string())
}

/// Get a specific device from the registry
#[tauri::command]
pub async fn get_device_from_registry(device_id: String) -> Result<Option<serde_json::Value>, String> {
    let db = IndexDb::open().map_err(|e| e.to_string())?;
    db.get_device_by_id(&device_id).map_err(|e| e.to_string())
}

/// Update setup step for a device
#[tauri::command]
pub async fn update_device_setup_step(device_id: String, step: i32) -> Result<(), String> {
    let db = IndexDb::open().map_err(|e| e.to_string())?;
    db.update_device_setup_step(&device_id, step)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Mark device setup as complete
#[tauri::command]
pub async fn mark_device_setup_complete(
    device_id: String,
    eth_address: Option<String>,
) -> Result<(), String> {
    let db = IndexDb::open().map_err(|e| e.to_string())?;
    db.mark_device_setup_complete(&device_id, eth_address.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Check if a device needs setup
#[tauri::command]
pub async fn device_needs_setup(device_id: String) -> Result<bool, String> {
    let db = IndexDb::open().map_err(|e| e.to_string())?;
    db.device_needs_setup(&device_id).map_err(|e| e.to_string())
}

/// Get devices with incomplete setup
#[tauri::command]
pub async fn get_incomplete_setup_devices() -> Result<Vec<serde_json::Value>, String> {
    let db = IndexDb::open().map_err(|e| e.to_string())?;
    db.get_incomplete_setup_devices().map_err(|e| e.to_string())
}

/// Reset device setup (for testing/debugging)
#[tauri::command]
pub async fn reset_device_setup(device_id: String) -> Result<(), String> {
    let db = IndexDb::open().map_err(|e| e.to_string())?;
    db.reset_device_setup(&device_id).map_err(|e| e.to_string())?;
    Ok(())
}

/// Get ETH address for a device (if available)
#[tauri::command]
pub async fn get_device_eth_address(
    device_id: String,
    queue_manager: State<'_, DeviceQueueManager>,
) -> Result<Option<String>, String> {
    // First check if we have it cached in the registry
    let db = IndexDb::open().map_err(|e| e.to_string())?;
    if let Some(device) = db.get_device_by_id(&device_id).map_err(|e| e.to_string())? {
        if let Some(eth_address) = device.get("eth_address").and_then(|v| v.as_str()) {
            if !eth_address.is_empty() {
                return Ok(Some(eth_address.to_string()));
            }
        }
    }
    
    // If not cached, try to get it from the device
    println!("🔍 Getting ETH address for device: {}", device_id);
    
    let manager = queue_manager.lock().await;
    let queue = manager.get(&device_id)
        .ok_or_else(|| format!("Device queue not found for device: {}", device_id))?;
    
    // Get ETH address for the standard derivation path
    let address_n = vec![44 + 0x80000000, 60 + 0x80000000, 0 + 0x80000000, 0, 0]; // m/44'/60'/0'/0/0
    
    match queue.get_address(address_n, "Ethereum".to_string(), None, Some(false)).await {
        Ok(address) => {
            // Cache the address in the database
            if let Err(e) = IndexDb::open().map_err(|e| e.to_string())?.mark_device_setup_complete(&device_id, Some(&address)) {
                log::warn!("Failed to cache ETH address: {}", e);
            }
            Ok(Some(address))
        }
        Err(e) => {
            println!("❌ Failed to get ETH address: {}", e);
            Ok(None)
        }
    }
}

/// Check if device is ready and handle onboarding status
pub async fn check_device_ready_and_onboarding(device_id: &str, app: &AppHandle, queue_manager: &DeviceQueueManager) -> Result<(), String> {
    log::info!("🔍 Checking device readiness and onboarding status for device: {}", device_id);
    
    // Get device features first
    let queue_handle = get_or_create_device_queue(device_id, queue_manager).await?;
    
    match queue_handle.get_features().await {
        Ok(features) => {
            log::info!("✅ Got features for device {}: initialized={}", device_id, features.initialized.unwrap_or(false));
            
            // Check if device is initialized
            if features.initialized.unwrap_or(false) {
                // Device is initialized, now check onboarding status
                match is_onboarded().await {
                    Ok(is_onboarded) => {
                        if is_onboarded {
                            log::info!("✅ Device {} is ready and user is onboarded", device_id);
                            
                            // Emit device ready event
                            let ready_payload = serde_json::json!({
                                "device_id": device_id,
                                "status": "device_ready",
                                "features": convert_features_to_device_features(features),
                                "message": "Device is ready and user has completed onboarding"
                            });
                            
                            if let Err(e) = emit_or_queue_event(app, "device:ready", ready_payload).await {
                                log::error!("❌ Failed to emit device ready event: {}", e);
                            }
                        } else {
                            log::info!("📚 Device {} is ready but user needs onboarding", device_id);
                            
                            // Emit onboarding required event
                            let onboarding_payload = serde_json::json!({
                                "device_id": device_id,
                                "status": "device_ready_onboarding_needed",
                                "features": convert_features_to_device_features(features),
                                "message": "Device is ready but user needs to complete onboarding"
                            });
                            
                            if let Err(e) = emit_or_queue_event(app, "device:onboarding-required", onboarding_payload).await {
                                log::error!("❌ Failed to emit onboarding required event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("❌ Failed to check onboarding status: {}", e);
                        
                        // Default to showing onboarding if we can't determine status
                        let onboarding_payload = serde_json::json!({
                            "device_id": device_id,
                            "status": "device_ready_onboarding_needed",
                            "features": convert_features_to_device_features(features),
                            "message": "Device is ready but onboarding status unclear - showing onboarding"
                        });
                        
                        if let Err(e) = emit_or_queue_event(app, "device:onboarding-required", onboarding_payload).await {
                            log::error!("❌ Failed to emit onboarding required event: {}", e);
                        }
                    }
                }
            } else {
                log::info!("⏳ Device {} is not initialized yet", device_id);
                
                // Emit device needs initialization event
                let init_payload = serde_json::json!({
                    "device_id": device_id,
                    "status": "device_needs_initialization",
                    "features": convert_features_to_device_features(features),
                    "message": "Device is connected but needs initialization"
                });
                
                if let Err(e) = emit_or_queue_event(app, "device:needs-initialization", init_payload).await {
                    log::error!("❌ Failed to emit device needs initialization event: {}", e);
                }
            }
        }
        Err(e) => {
            log::error!("❌ Failed to get features for device {}: {}", device_id, e);
            
            // Emit error event
            let error_payload = serde_json::json!({
                "device_id": device_id,
                "status": "device_error",
                "error": e.to_string(),
                "message": "Failed to get device features"
            });
            
            if let Err(e) = emit_or_queue_event(app, "device:error", error_payload).await {
                log::error!("❌ Failed to emit device error event: {}", e);
            }
        }
    }
    
    Ok(())
} 