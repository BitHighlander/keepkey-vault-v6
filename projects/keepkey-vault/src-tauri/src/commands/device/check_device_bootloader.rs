// commands/device/check_device_bootloader.rs

use tauri::State;
use std::sync::Arc;
use keepkey_db::Database;
use crate::commands::DeviceQueueManager;
use super::get_or_create_device_queue;
use super::get_features::convert_features_to_device_features;
use crate::device::updates::{check_bootloader_status, FrontendBootloaderCheck, VersionComparison};

/// Check device bootloader status and determine if update is needed
#[tauri::command]
pub async fn check_device_bootloader(
    device_id: String,
    queue_manager: State<'_, DeviceQueueManager>,
    database: State<'_, Arc<Database>>,
) -> Result<FrontendBootloaderCheck, String> {
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
            
            // Check bootloader status using existing logic
            if let Some(bootloader_check) = check_bootloader_status(&device_features) {
                // Convert internal BootloaderCheck to frontend format
                let severity = match bootloader_check.comparison {
                    VersionComparison::MajorBehind => "critical",
                    VersionComparison::MinorBehind => "high", 
                    VersionComparison::PatchBehind => "medium",
                    VersionComparison::Current => "low",
                };
                
                Ok(FrontendBootloaderCheck {
                    needs_update: bootloader_check.is_outdated,
                    current_version: bootloader_check.current_version,
                    latest_version: bootloader_check.latest_version,
                    is_required: bootloader_check.is_critical,
                    severity: severity.to_string(),
                })
            } else {
                // Device not in bootloader mode - assume bootloader is fine for now
                // TODO: Add logic to check bootloader version even when not in bootloader mode
                Ok(FrontendBootloaderCheck {
                    needs_update: false,
                    current_version: device_features.version.clone(),
                    latest_version: device_features.version.clone(),
                    is_required: false,
                    severity: "low".to_string(),
                })
            }
        }
        Err(e) => {
            log::error!("❌ Failed to get features for device {}: {}", device_id, e);
            Err(format!("Failed to get device features: {}", e))
        }
    }
} 