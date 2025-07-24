use tauri::State;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use keepkey_rust::{
    device_queue::{DeviceQueueFactory, DeviceQueueHandle},
    features::DeviceFeatures,
};

// Type alias for the device queue manager
pub type DeviceQueueManager = Arc<Mutex<HashMap<String, DeviceQueueHandle>>>;

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