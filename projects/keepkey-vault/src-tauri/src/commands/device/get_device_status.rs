use serde::{Deserialize, Serialize};
use crate::commands::DeviceQueueManager;
use super::get_or_create_device_queue;
use tauri::State;
use keepkey_rust::features::DeviceFeatures;
use std::fs;
use std::path::Path;

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
    
    // If we can't get device features, the device is not properly accessible
    if features.is_none() {
        log::warn!("Device {} features unavailable - device may be disconnected or inaccessible", device_id);
        status.connected = false;
        // Don't set any firmware or bootloader checks since we can't read the device
        return status;
    }
    
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
        let latest_firmware_version = get_latest_firmware_version().unwrap_or_else(|_| "7.10.0".to_string());
        let needs_firmware_update = if features.bootloader_mode {
            // In bootloader mode, firmware might need update - compare current to latest
            // Note: In bootloader mode, version might be bootloader version, not firmware version
            // For safety, assume firmware needs update if we're already in bootloader mode
            // unless we can confirm it's already the latest
            !needs_bootloader_update && is_version_older(&features.version, &latest_firmware_version)
        } else {
            // In normal mode, compare current firmware version to latest
            is_version_older(&features.version, &latest_firmware_version)
        };
        
        status.needs_firmware_update = needs_firmware_update;
        status.firmware_check = Some(FirmwareCheck {
            current_version: features.version.clone(),
            latest_version: latest_firmware_version,
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

/// Helper function to read latest firmware version from releases.json
fn get_latest_firmware_version() -> Result<String, String> {
    let possible_paths = [
        "firmware/releases.json",
        "./firmware/releases.json", 
        "../firmware/releases.json",
        "../../firmware/releases.json",
    ];
    
    for path in &possible_paths {
        if Path::new(path).exists() {
            if let Ok(contents) = fs::read_to_string(path) {
                if let Ok(releases) = serde_json::from_str::<serde_json::Value>(&contents) {
                    if let Some(version) = releases["latest"]["firmware"]["version"].as_str() {
                        // Remove 'v' prefix if present
                        let clean_version = version.trim_start_matches('v');
                        return Ok(clean_version.to_string());
                    }
                }
            }
        }
    }
    
    // Fallback to known latest version if releases.json not found
    Ok("7.10.0".to_string())
}

/// Helper function to compare semantic versions
fn is_version_older(current: &str, latest: &str) -> bool {
    let current_parts: Vec<u32> = current.trim_start_matches('v')
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let latest_parts: Vec<u32> = latest.trim_start_matches('v')
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    // Pad with zeros if needed
    let max_len = current_parts.len().max(latest_parts.len());
    let mut current_padded = current_parts.clone();
    let mut latest_padded = latest_parts.clone();
    current_padded.resize(max_len, 0);
    latest_padded.resize(max_len, 0);
    
    for (curr, latest) in current_padded.iter().zip(latest_padded.iter()) {
        if curr < latest {
            return true;
        } else if curr > latest {
            return false;
        }
    }
    
    false // Versions are equal
}

/// Get device status command
#[tauri::command]
pub async fn get_device_status(
    device_id: String,
    queue_manager: State<'_, DeviceQueueManager>,
) -> Result<Option<DeviceStatus>, String> {
    log::info!("Getting device status for: {}", device_id);
    
    // Resolve to canonical device ID in case this is an alias
    let canonical_id = crate::commands::get_canonical_device_id(&device_id);
    log::info!("Canonical device ID: {}", canonical_id);
    
    // Get connected devices to find the one we want
    let devices = keepkey_rust::features::list_connected_devices();
    
    // Find device by exact ID match or potential same device
    let actual_device_to_check = devices.iter()
        .find(|d| d.unique_id == canonical_id || 
                  crate::commands::are_devices_potentially_same(&d.unique_id, &canonical_id))
        .cloned();
    
    if let Some(device_info) = actual_device_to_check {
        log::info!("🔍 Found device for status check: {}", device_info.unique_id);
        
        // If the device ID is different from what we started with, set up alias
        if device_info.unique_id != device_id && device_info.unique_id != canonical_id {
            log::info!("Setting up device alias: {} -> {}", device_info.unique_id, canonical_id);
            let _ = crate::commands::add_recovery_device_alias(&device_info.unique_id, &canonical_id);
        }
        
        // Get or create device queue handle - use the canonical device ID for queue management
        // This ensures temporary disconnection tracking works properly
        let queue_handle = get_or_create_device_queue(&canonical_id, &queue_manager).await?;
        
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
                log::error!("Failed to get features for device {}: {}", device_info.unique_id, e);
                None
            }
            Err(_) => {
                log::error!("Timeout getting features for device {}", device_info.unique_id);
                None
            }
        };
        
        // Evaluate device status - use the original device_id for consistency
        let status = evaluate_device_status(device_id.clone(), features.as_ref());
        
        Ok(Some(status))
    } else {
        log::warn!("Device {} not found (canonical: {})", device_id, canonical_id);
        Ok(None)
    }
} 