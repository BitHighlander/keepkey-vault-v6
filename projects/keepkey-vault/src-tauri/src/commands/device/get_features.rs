// commands/device/get_features.rs

use tauri::State;
use keepkey_rust::features::DeviceFeatures;
use crate::commands::DeviceQueueManager;
use super::get_or_create_device_queue;
use crate::device::updates::determine_bootloader_version;

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

    // Set the bootloader version using our determining logic
    let bootloader_version = determine_bootloader_version(&device_features);
    device_features.bootloader_version = Some(bootloader_version.clone());
    
    log::info!("🔍 Final bootloader version determined: {}", bootloader_version);
    
    device_features
} 