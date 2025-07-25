// device/updates.rs - Device bootloader and firmware update operations

use tauri::State;
use serde::{Serialize, Deserialize};
use keepkey_rust::features::DeviceFeatures;
use std::cmp::Ordering;
use std::collections::HashMap;
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

// Releases.json structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleasesData {
    pub latest: LatestVersions,
    pub hashes: HashMappings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersions {
    pub bootloader: BootloaderVersionInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootloaderVersionInfo {
    pub version: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashMappings {
    pub bootloader: HashMap<String, String>,
}

/// Load releases.json and map bootloader hash to version
pub fn bootloader_version_from_hash(hash: &str) -> Option<String> {
    // Load releases.json from the firmware directory
    let releases_path = std::path::PathBuf::from("firmware/releases.json");
    
    match std::fs::read_to_string(&releases_path) {
        Ok(json_str) => {
            match serde_json::from_str::<ReleasesData>(&json_str) {
                Ok(releases) => {
                    if let Some(version) = releases.hashes.bootloader.get(hash) {
                        // Clean up version string (remove 'v' prefix if present)
                        let clean_version = version.trim_start_matches('v');
                        log::info!("🔍 Mapped bootloader hash {} to version {}", &hash[..8], clean_version);
                        Some(clean_version.to_string())
                    } else {
                        log::warn!("⚠️ No bootloader version found for hash {}", &hash[..8]);
                        None
                    }
                }
                Err(e) => {
                    log::error!("❌ Failed to parse releases.json: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            log::error!("❌ Failed to load releases.json from {:?}: {}", releases_path, e);
            None
        }
    }
}

/// Check device bootloader status and determine if update is needed
/// SIMPLE: No hash = error, hash = 2.1.4 = success, otherwise needs update
pub fn check_bootloader_status(features: &DeviceFeatures) -> Option<BootloaderCheck> {
    let latest_bootloader_version = "2.1.4".to_string();
    
    // Get the bootloader version (which should be the hash)
    let current_bootloader_version = if let Some(ref bl_version) = features.bootloader_version {
        bl_version.clone()
    } else {
        "NO_BOOTLOADER_HASH".to_string()
    };
    
    log::info!("🔒 Simple bootloader check: current='{}', latest='{}'", current_bootloader_version, latest_bootloader_version);
    
    // SIMPLE logic:
    if current_bootloader_version == "NO_BOOTLOADER_HASH" {
        // Failed to get bootloader hash
        log::error!("🚨 Failed to get bootloader hash");
        return None; // This will cause an error in the calling function
    } else if current_bootloader_version == latest_bootloader_version {
        // Valid bootloader version
        log::info!("✅ Valid bootloader version: {}", current_bootloader_version);
        Some(BootloaderCheck {
            current_version: current_bootloader_version,
            latest_version: latest_bootloader_version,
            is_outdated: false,
            is_critical: false,
            comparison: VersionComparison::Current,
        })
    } else {
        // Bootloader needs update
        log::info!("📋 Bootloader version: {} (needs update to {})", current_bootloader_version, latest_bootloader_version);
        Some(BootloaderCheck {
            current_version: current_bootloader_version,
            latest_version: latest_bootloader_version,
            is_outdated: true,
            is_critical: true,
            comparison: VersionComparison::MajorBehind,
        })
    }
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
/// SIMPLE: Just return the bootloader hash if available, otherwise error state
pub fn determine_bootloader_version(device_features: &DeviceFeatures) -> String {
    log::info!("🔍 determine_bootloader_version called for device:");
    log::info!("   - firmware_version: {}", device_features.version);
    log::info!("   - bootloader_mode: {}", device_features.bootloader_mode);
    log::info!("   - bootloader_hash: {:?}", device_features.bootloader_hash);
    
    // If we have bootloader hash, try to map it to a version
    if let Some(ref bl_hash) = device_features.bootloader_hash {
        log::info!("📋 Bootloader hash found: {}", bl_hash);
        
        // Try to map hash to version
        if let Some(version) = bootloader_version_from_hash(bl_hash) {
            log::info!("✅ Successfully mapped hash to version: {}", version);
            version
        } else {
            log::warn!("⚠️ Hash not found in releases.json, returning hash as version");
            bl_hash.clone() // Fallback to hash if not found in mapping
        }
    } else {
        log::error!("🚨 Failed to get bootloader hash");
        "NO_BOOTLOADER_HASH".to_string() // Clear error marker
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