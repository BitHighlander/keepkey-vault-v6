use serde::{Deserialize, Serialize};
use crate::commands::DeviceQueueManager;
use super::get_or_create_device_queue;
use tauri::State;

// Development mode: Force all devices to use this specific deviceId
const DEV_FORCE_DEVICE_ID: &str = "932313031174732313008100";
const DEV_MODE: bool = true; // Set to false for production

// DeviceStatus and related structs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    pub device_id: String,
    pub connected: bool,
    pub features: Option<keepkey_rust::features::DeviceFeatures>,
    pub needs_bootloader_update: bool,
    pub needs_firmware_update: bool,
    pub needs_initialization: bool,
    pub needs_pin_unlock: bool,
    pub bootloader_check: Option<BootloaderCheck>,
    pub firmware_check: Option<FirmwareCheck>,
    pub initialization_check: Option<InitializationCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootloaderCheck {
    pub current_version: String,
    pub latest_version: String,
    pub needs_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareCheck {
    pub current_version: String,
    pub latest_version: String,
    pub needs_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializationCheck {
    pub initialized: bool,
    pub has_backup: bool,
    pub imported: bool,
    pub needs_setup: bool,
}

/// Evaluate device status to determine what actions are needed
pub fn evaluate_device_status(device_id: String, features: Option<&keepkey_rust::features::DeviceFeatures>) -> DeviceStatus {
    let mut status = DeviceStatus {
        device_id: device_id.clone(),
        connected: true,
        features: features.cloned(),
        needs_bootloader_update: false,
        needs_firmware_update: false,
        needs_initialization: false,
        needs_pin_unlock: false,
        bootloader_check: None,
        firmware_check: None,
        initialization_check: None,
    };
    
    if let Some(features) = features {
        let latest_bootloader_version = "2.1.4".to_string();
        
        // Get current bootloader version
        let current_bootloader_version = features.bootloader_version.clone().unwrap_or_else(|| {
            if features.bootloader_mode {
                features.version.clone()
            } else if features.version.starts_with("1.0.") {
                features.version.clone()
            } else {
                "2.1.4".to_string()
            }
        });
        
        // Check if bootloader needs update
        let needs_bootloader_update = if features.bootloader_mode {
            current_bootloader_version.starts_with("1.")
        } else if current_bootloader_version == "Unknown bootloader" {
            false
        } else {
            // Simple version comparison for now
            current_bootloader_version != latest_bootloader_version && 
            !current_bootloader_version.starts_with("2.1.")
        };
        
        status.needs_bootloader_update = needs_bootloader_update;
        status.bootloader_check = Some(BootloaderCheck {
            current_version: current_bootloader_version.clone(),
            latest_version: latest_bootloader_version,
            needs_update: needs_bootloader_update,
        });
        
        // Check firmware status
        let needs_firmware_update = features.bootloader_mode && !needs_bootloader_update;
        status.needs_firmware_update = needs_firmware_update;
        status.firmware_check = Some(FirmwareCheck {
            current_version: features.version.clone(),
            latest_version: "4.0.0".to_string(), // Current latest firmware
            needs_update: needs_firmware_update,
        });
        
        // Check initialization status
        let needs_initialization = !features.initialized;
        let has_backup = !features.no_backup;
        status.needs_initialization = needs_initialization;
        status.initialization_check = Some(InitializationCheck {
            initialized: features.initialized,
            has_backup,
            imported: features.imported.unwrap_or(false),
            needs_setup: needs_initialization,
        });
        
        // Check PIN status
        status.needs_pin_unlock = features.pin_protection && !features.pin_cached;
    }
    
    status
}

/// Get device status command
#[tauri::command]
pub async fn get_device_status(
    device_id: String,
    queue_manager: State<'_, DeviceQueueManager>,
) -> Result<Option<DeviceStatus>, String> {
    log::info!("Getting device status for: {}", device_id);
    
    // Get connected devices to find the one we want
    let devices = keepkey_rust::features::list_connected_devices();
    
    // In development mode, if requesting the hardcoded deviceId, map to physical device
    let actual_device_to_check = if DEV_MODE && device_id == DEV_FORCE_DEVICE_ID {
        log::info!("🛠️ DEV MODE: Mapping deviceId {} to physical device", DEV_FORCE_DEVICE_ID);
        
        // Find any connected KeepKey device to use as the physical device
        devices.iter()
            .find(|d| d.is_keepkey)
            .cloned()
    } else {
        // Normal mode: find device by exact ID match
        devices.iter()
            .find(|d| d.unique_id == device_id)
            .cloned()
    };
    
    if let Some(device_info) = actual_device_to_check {
        log::info!("🔍 Found device for status check: {}", device_info.unique_id);
        
        // Get or create device queue handle using the original device_id for mapping
        let queue_handle = get_or_create_device_queue(&device_id, &queue_manager).await?;
        
        // Fetch device features through the queue
        let features = match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            queue_handle.get_features()
        ).await {
            Ok(Ok(raw_features)) => {
                // Convert features to our format
                Some(crate::commands::device::get_features::convert_features_to_device_features(raw_features))
            }
            Ok(Err(e)) => {
                log::error!("Failed to get features for device {}: {}", device_id, e);
                None
            }
            Err(_) => {
                log::error!("Timeout getting features for device {}", device_id);
                None
            }
        };
        
        // Evaluate device status
        let status = evaluate_device_status(device_id.clone(), features.as_ref());
        
        Ok(Some(status))
    } else {
        log::warn!("Device {} not found", device_id);
        Ok(None)
    }
} 