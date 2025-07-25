// device/updates.rs - Device bootloader and firmware update operations

use tauri::State;
use serde::{Serialize, Deserialize};
use keepkey_rust::features::DeviceFeatures;
use std::cmp::Ordering;
use crate::commands::DeviceQueueManager;

// Bootloader update types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VersionComparison {
    Current,
    PatchBehind,
    MinorBehind,
    MajorBehind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootloaderCheck {
    pub is_outdated: bool,
    pub current_version: String,
    pub latest_version: String,
    pub is_critical: bool,
    pub comparison: VersionComparison,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendBootloaderCheck {
    pub needs_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub is_required: bool,
    pub severity: String, // 'low' | 'medium' | 'high' | 'critical'
}

/// Check bootloader status with proper version logic
/// Defaults to "needs update" if bootloader version cannot be determined for security
pub fn check_bootloader_status(features: &DeviceFeatures) -> Option<BootloaderCheck> {
    let latest_bootloader_version = "2.1.4".to_string(); // Latest official bootloader version
    
    // Determine current bootloader version based on device state and mode
    let current_bootloader_version = if features.bootloader_mode {
        // Device is in bootloader mode - use the firmware version as bootloader version
        if features.version.starts_with("1.") {
            features.version.clone() // OOB bootloader versions like 1.0.3
        } else {
            // Modern bootloader in bootloader mode - use version directly
            features.version.clone()
        }
    } else {
        // Device is in normal firmware mode - check if it's an OOB device
        if features.version.starts_with("1.0.") {
            // OOB device: firmware version 1.0.3 = bootloader version 1.0.3
            features.version.clone()
        } else if let Some(ref bl_version) = features.bootloader_version {
            // Use explicit bootloader version if available
            bl_version.clone()
        } else {
            // SECURITY DEFAULT: If we can't determine bootloader version, assume it needs update
            // This is safer than assuming it's current
            "0.0.0".to_string() // This will trigger "needs update"
        }
    };
    
    // Compare versions using semantic versioning
    let comparison = match compare_versions(&current_bootloader_version, &latest_bootloader_version) {
        Ok(comp) => comp,
        Err(_) => {
            // If version comparison fails, assume needs update for security
            VersionComparison::MajorBehind
        }
    };
    
    let is_outdated = !matches!(comparison, VersionComparison::Current);
    let is_critical = matches!(comparison, VersionComparison::MajorBehind);
    
    Some(BootloaderCheck {
        current_version: current_bootloader_version,
        latest_version: latest_bootloader_version.clone(),
        is_outdated,
        is_critical,
        comparison,
    })
}

/// Compare two semantic versions
pub fn compare_versions(current: &str, latest: &str) -> Result<VersionComparison, String> {
    // Helper to parse version string into (major, minor, patch)
    let parse_version = |v: &str| -> Result<(u32, u32, u32), String> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version format: {}", v));
        }
        
        let major = parts[0].parse::<u32>().map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1].parse::<u32>().map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2].parse::<u32>().map_err(|_| format!("Invalid patch version: {}", parts[2]))?;
        
        Ok((major, minor, patch))
    };
    
    let (curr_major, curr_minor, curr_patch) = parse_version(current)?;
    let (latest_major, latest_minor, latest_patch) = parse_version(latest)?;
    
    match (curr_major.cmp(&latest_major), curr_minor.cmp(&latest_minor), curr_patch.cmp(&latest_patch)) {
        (Ordering::Less, _, _) => Ok(VersionComparison::MajorBehind),
        (Ordering::Equal, Ordering::Less, _) => Ok(VersionComparison::MinorBehind),
        (Ordering::Equal, Ordering::Equal, Ordering::Less) => Ok(VersionComparison::PatchBehind),
        (Ordering::Equal, Ordering::Equal, Ordering::Equal) => Ok(VersionComparison::Current),
        _ => Ok(VersionComparison::Current), // Current version is newer than latest (unusual but treat as current)
    }
}

/// Determine bootloader version for device features during conversion
pub fn determine_bootloader_version(device_features: &DeviceFeatures) -> String {
    let latest_bootloader_version = "2.1.4".to_string();
    
    if device_features.bootloader_mode {
        // Device is in bootloader mode - use the firmware version as bootloader version
        if device_features.version.starts_with("1.") {
            device_features.version.clone() // OOB bootloader versions like 1.0.3
        } else {
            // Modern bootloader in bootloader mode - use version directly
            device_features.version.clone()
        }
    } else {
        // Device is in normal firmware mode - check if it's an OOB device
        if device_features.version.starts_with("1.0.") {
            // OOB device: firmware version 1.0.3 = bootloader version 1.0.3
            device_features.version.clone()
        } else if let Some(ref bl_hash) = device_features.bootloader_hash {
            // For modern devices, try to use the bootloader hash
            // TODO: Could implement hash-to-version mapping here
            bl_hash.clone()
        } else {
            // SECURITY DEFAULT: If we can't determine bootloader version, assume it needs update
            // This is safer than assuming it's current
            "0.0.0".to_string() // This will trigger "needs update"
        }
    }
}

/// Update device bootloader
#[tauri::command]
pub async fn update_device_bootloader(
    device_id: String,
    target_version: String,
    _queue_manager: State<'_, DeviceQueueManager>,
) -> Result<bool, String> {
    log::info!("🔄 Updating bootloader for device {} to version {}", device_id, target_version);
    
    // TODO: Implement actual bootloader update logic
    // This should:
    // 1. Put device into bootloader mode if not already
    // 2. Flash the new bootloader
    // 3. Verify the update was successful
    // 4. Return success/failure
    
    // For now, return an error indicating it's not implemented
    Err("Bootloader update not yet implemented".to_string())
}

/// Update device firmware  
#[tauri::command]
pub async fn update_device_firmware(
    device_id: String,
    target_version: String,
    _queue_manager: State<'_, DeviceQueueManager>,
) -> Result<bool, String> {
    log::info!("🔄 Updating firmware for device {} to version {}", device_id, target_version);
    
    // TODO: Implement actual firmware update logic
    // This should:
    // 1. Ensure device is in bootloader mode
    // 2. Flash the new firmware
    // 3. Verify the update was successful
    // 4. Return success/failure
    
    // For now, return an error indicating it's not implemented
    Err("Firmware update not yet implemented".to_string())
} 