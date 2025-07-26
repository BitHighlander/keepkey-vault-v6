// commands/device/get_features.rs

use tauri::State;
use keepkey_rust::features::DeviceFeatures;
use crate::commands::DeviceQueueManager;
use super::get_or_create_device_queue;

/// Get features for a specific device with proper bootloader mode communication
#[tauri::command]
pub async fn get_features(
    device_id: String,
    queue_manager: State<'_, DeviceQueueManager>,
) -> Result<DeviceFeatures, String> {
    println!("🔍 Getting features for device: {}", device_id);
    
    // First try the normal queue-based approach
    let queue_handle = get_or_create_device_queue(&device_id, &queue_manager).await?;
    
    match queue_handle.get_features().await {
        Ok(features) => {
            println!("✅ Successfully got features for device via queue: {}", device_id);
            Ok(convert_features_to_device_features(features))
        }
        Err(e) => {
            // Check if this might be an OOB bootloader communication issue
            let error_str = e.to_string();
            if error_str.contains("HID write failed") || error_str.contains("Device is disconnected") {
                println!("🔧 Queue-based features failed for {}: {}", device_id, error_str);
                println!("🔄 Attempting OOB bootloader detection method...");
                
                // Try the proper OOB bootloader detection method
                try_oob_bootloader_detection(&device_id).await
            } else {
                let error_msg = format!("Failed to get device features for {}: {}", device_id, e);
                println!("❌ {}", error_msg);
                Err(error_msg)
            }
        }
    }
}

/// Try to get device features using OOB bootloader detection methods
async fn try_oob_bootloader_detection(device_id: &str) -> Result<DeviceFeatures, String> {
    println!("🔧 Attempting OOB bootloader detection for device {}", device_id);
    
    // Get list of connected devices to find the physical device
    let devices = keepkey_rust::features::list_connected_devices();
    
    // Find device by exact ID match
    let target_device = devices.iter()
        .find(|d| d.unique_id == device_id)
        .ok_or_else(|| format!("Device {} not found in connected devices", device_id))?;
    
    // Use the proper OOB bootloader detection method
    let result = tokio::task::spawn_blocking({
        let device = target_device.clone();
        move || -> Result<DeviceFeatures, String> {
            keepkey_rust::features::get_device_features_with_fallback(&device)
                .map_err(|e| e.to_string())
        }
    }).await;
    
    match result {
        Ok(Ok(features)) => {
            println!("✅ OOB bootloader detection successful for device {}", device_id);
            println!("   - bootloader_mode: {}", features.bootloader_mode);
            println!("   - version: {}", features.version);
            println!("   - initialized: {}", features.initialized);
            Ok(features)
        }
        Ok(Err(e)) => {
            let error_msg = format!("OOB bootloader detection failed for {}: {}", device_id, e);
            println!("❌ {}", error_msg);
            Err(error_msg)
        }
        Err(e) => {
            let error_msg = format!("Task execution error for {}: {}", device_id, e);
            println!("❌ {}", error_msg);
            Err(error_msg)
        }
    }
}

/// Look up bootloader version from hash using releases.json
fn bootloader_version_from_hash(hash: &str) -> Option<String> {
    // Try to load releases.json from various possible locations
    let possible_paths = [
        "firmware/releases.json",
        "./firmware/releases.json", 
        "../firmware/releases.json",
        "../../firmware/releases.json",
    ];
    
    for path in &possible_paths {
        if let Ok(contents) = std::fs::read_to_string(path) {
            if let Ok(releases) = serde_json::from_str::<serde_json::Value>(&contents) {
                if let Some(hashes) = releases["hashes"]["bootloader"].as_object() {
                    if let Some(version) = hashes.get(hash) {
                        if let Some(version_str) = version.as_str() {
                            // Remove 'v' prefix if present for consistency
                            let clean_version = version_str.trim_start_matches('v');
                            log::info!("🔍 Found bootloader version {} for hash {}", clean_version, hash);
                            return Some(clean_version.to_string());
                        }
                    }
                }
            }
        }
    }
    
    log::warn!("🔍 No bootloader version found for hash {}", hash);
    None
}

/// Convert raw Features to DeviceFeatures
pub fn convert_features_to_device_features(features: keepkey_rust::messages::Features) -> DeviceFeatures {
    // Log the raw features we're getting from the device
    log::info!("🔍 Raw device features received:");
    log::info!("   - firmware version: {}.{}.{}", 
        features.major_version.unwrap_or(0),
        features.minor_version.unwrap_or(0), 
        features.patch_version.unwrap_or(0)
    );
    log::info!("   - bootloader_mode: {:?}", features.bootloader_mode);
    log::info!("   - bootloader_hash (raw): {:?}", features.bootloader_hash);
    log::info!("   - firmware_hash (raw): {:?}", features.firmware_hash);
    
    // First create the basic device features
    let mut device_features = DeviceFeatures {
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
        bootloader_version: None, // Will be populated below
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
    };

    // Log what we've converted so far
    log::info!("🔍 Converted device features (before bootloader version):");
    log::info!("   - firmware_hash (hex): {:?}", device_features.firmware_hash);
    log::info!("   - bootloader_hash (hex): {:?}", device_features.bootloader_hash);

    // Determine bootloader version from hash if available
    if device_features.bootloader_version.is_none() {
        if let Some(ref bootloader_hash) = device_features.bootloader_hash {
            device_features.bootloader_version = bootloader_version_from_hash(bootloader_hash);
        }
        
        // If still no bootloader version, use fallback logic like v5
        if device_features.bootloader_version.is_none() {
            if device_features.bootloader_mode {
                // Device is in bootloader mode - use the firmware version as bootloader version for old bootloaders
                if device_features.version.starts_with("1.") {
                    device_features.bootloader_version = Some(device_features.version.clone());
                } else {
                    device_features.bootloader_version = Some("unknown".to_string());
                }
            } else {
                // Device is in normal firmware mode - check if it's an OOB device  
                if device_features.version.starts_with("1.0.") {
                    // OOB device: firmware version 1.0.3 = bootloader version 1.0.3
                    device_features.bootloader_version = Some(device_features.version.clone());
                } else {
                    // For modern firmware, assume recent bootloader if not specified
                    device_features.bootloader_version = Some("2.1.4".to_string());
                }
            }
        }
    }
    
    log::info!("🔍 Final bootloader version: {:?}", device_features.bootloader_version);
    
    device_features
} 