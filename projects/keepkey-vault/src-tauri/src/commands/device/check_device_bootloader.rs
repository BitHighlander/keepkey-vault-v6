// commands/device/check_device_bootloader.rs

use tauri::State;
use std::sync::Arc;
use keepkey_db::Database;
use crate::commands::DeviceQueueManager;
use super::get_or_create_device_queue;
use super::get_features::convert_features_to_device_features;
use keepkey_rust::device_update::{check_bootloader_status, BootloaderCheck};

/// Check device bootloader status and determine if update is needed
/// SECURITY: This function MUST fail safe - if bootloader version cannot be determined, it MUST return an error
#[tauri::command]
pub async fn check_device_bootloader(
    device_id: String,
    queue_manager: State<'_, DeviceQueueManager>,
    database: State<'_, Arc<Database>>,
) -> Result<BootloaderCheck, String> {
    log::info!("🔍 Checking bootloader status for device: {}", device_id);
    
    // Get device features first
    let queue_handle = get_or_create_device_queue(&device_id, &queue_manager).await?;
    
    match queue_handle.get_features().await {
        Ok(features) => {
            log::info!("✅ Got features for device {}: bootloader_mode={}", device_id, features.bootloader_mode.unwrap_or(false));
            
            // Convert to DeviceFeatures for compatibility with existing code
            let device_features = convert_features_to_device_features(features.clone());
            
            // Store/update device features in database
            let features_json = serde_json::to_string(&device_features).map_err(|e| e.to_string())?;
            if let Err(e) = database.update_device_features(&device_id, &features_json).await {
                log::warn!("Failed to update device features in database: {}", e);
            }
            
            // SIMPLE: Try to get bootloader status
            let bootloader_check = check_bootloader_status(&device_features);
            
            // We got a bootloader check result - use it directly
            let severity = if bootloader_check.is_critical {
                "critical"
            } else if bootloader_check.needs_update {
                "high"
            } else {
                "low"
            };

            log::info!("🔒 Bootloader check completed: version={}, needs_update={}, severity={}", 
                bootloader_check.current_version, bootloader_check.needs_update, severity);

            Ok(bootloader_check)
        }
        Err(e) => {
            log::error!("❌ Failed to get features for device {}: {}", device_id, e);
            Err(format!("Failed to get device features: {}", e))
        }
    }
} 