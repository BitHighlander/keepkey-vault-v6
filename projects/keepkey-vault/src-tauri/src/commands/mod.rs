// commands/mod.rs - Organized command modules

// Core types used across commands
use std::sync::Arc;
use tauri::State;
use keepkey_rust::device_queue::{DeviceQueueHandle, DeviceQueueFactory};
use std::collections::HashMap;
use tokio::sync::Mutex;
use lazy_static::lazy_static;

pub mod api;
pub mod device;
pub mod pin;
pub mod config;
pub mod events;
pub mod recovery;
pub mod verification;
pub mod logging;
pub mod cache;
pub mod test;

// Export the API endpoint function (if it exists in api module)
// pub use api::handle_api_endpoint;

// State type for device queue management
pub type DeviceQueueManager = Arc<tokio::sync::Mutex<HashMap<String, DeviceQueueHandle>>>;

// Recovery flow tracking
lazy_static! {
    static ref RECOVERY_DEVICE_FLOWS: std::sync::Mutex<std::collections::HashSet<String>> = std::sync::Mutex::new(std::collections::HashSet::new());
    static ref RECOVERY_DEVICE_ALIASES: std::sync::Mutex<HashMap<String, String>> = std::sync::Mutex::new(HashMap::new());
    static ref TEMPORARILY_DISCONNECTED_DEVICES: std::sync::Mutex<std::collections::HashSet<String>> = std::sync::Mutex::new(std::collections::HashSet::new());
}

// Re-export common commands
pub use events::{emit_or_queue_event, frontend_ready};
pub use device::{get_connected_devices, get_features, check_device_bootloader};
pub use config::{is_first_time_install, is_onboarded, set_onboarding_completed, get_preference, set_preference, debug_onboarding_state};

/// Mark device as being in recovery/firmware update flow
pub fn mark_device_in_recovery_flow(device_id: &str) -> Result<(), String> {
    let mut flows = RECOVERY_DEVICE_FLOWS.lock().map_err(|_| "Failed to lock recovery device flows".to_string())?;
    flows.insert(device_id.to_string());
    log::info!("Device {} marked as in recovery flow", device_id);
    Ok(())
}

/// Check if device is currently in recovery flow
pub fn is_device_in_recovery_flow(device_id: &str) -> bool {
    if let Ok(flows) = RECOVERY_DEVICE_FLOWS.lock() {
        flows.contains(device_id)
    } else {
        false
    }
}

/// Remove device from recovery flow state
pub fn unmark_device_in_recovery_flow(device_id: &str) -> Result<(), String> {
    let mut flows = RECOVERY_DEVICE_FLOWS.lock().map_err(|_| "Failed to lock recovery device flows".to_string())?;
    flows.remove(device_id);
    log::info!("Device {} removed from recovery flow", device_id);
    
    // Also clean up any aliases
    if let Ok(mut aliases) = RECOVERY_DEVICE_ALIASES.lock() {
        aliases.retain(|_, v| v != device_id);
    }
    
    Ok(())
}

/// Add device ID alias for recovery flow
pub fn add_recovery_device_alias(alias_id: &str, canonical_id: &str) -> Result<(), String> {
    let mut aliases = RECOVERY_DEVICE_ALIASES.lock()
        .map_err(|_| "Failed to lock recovery device aliases".to_string())?;
    aliases.insert(alias_id.to_string(), canonical_id.to_string());
    log::info!("Added recovery device alias: {} -> {}", alias_id, canonical_id);
    Ok(())
}

/// Get canonical device ID from alias
pub fn get_canonical_device_id(device_id: &str) -> String {
    if let Ok(aliases) = RECOVERY_DEVICE_ALIASES.lock() {
        if let Some(canonical) = aliases.get(device_id) {
            log::info!("Resolved device alias {} to canonical ID {}", device_id, canonical);
            return canonical.clone();
        }
    }
    device_id.to_string()
}

/// Check if two device IDs might be the same device
pub fn are_devices_potentially_same(id1: &str, id2: &str) -> bool {
    // Check if they're already the same
    if id1 == id2 {
        return true;
    }
    
    // Check if one is an alias of the other
    let canonical1 = get_canonical_device_id(id1);
    let canonical2 = get_canonical_device_id(id2);
    if canonical1 == canonical2 {
        return true;
    }
    
    // Check if they're both KeepKey devices (same VID/PID)
    let keepkey_pattern = "keepkey_2b24_0002_";
    if id1.contains(keepkey_pattern) && id2.contains(keepkey_pattern) {
        log::info!("Both {} and {} appear to be KeepKey devices", id1, id2);
        return true;
    }
    
    // Check if one has a serial and the other is a bus/address format
    let is_serial_format = |id: &str| id.len() == 24 && id.chars().all(|c| c.is_alphanumeric());
    let is_bus_addr_format = |id: &str| id.contains("bus") && id.contains("addr");
    
    if (is_serial_format(id1) && is_bus_addr_format(id2)) ||
       (is_bus_addr_format(id1) && is_serial_format(id2)) {
        log::info!("Device IDs {} and {} might be the same device (serial vs bus/addr)", id1, id2);
        return true;
    }
    
    false
}

/// Check if a device was marked as temporarily disconnected
pub fn was_device_temporarily_disconnected(device_id: &str) -> bool {
    if let Ok(disconnected_devices) = TEMPORARILY_DISCONNECTED_DEVICES.lock() {
        disconnected_devices.contains(device_id)
    } else {
        false
    }
}

/// Clear temporary disconnection flag for a device
pub fn clear_temporary_disconnection(device_id: &str) -> Result<(), String> {
    if let Ok(mut disconnected_devices) = TEMPORARILY_DISCONNECTED_DEVICES.lock() {
        disconnected_devices.remove(device_id);
        log::info!("🧹 Cleared temporary disconnection flag for device: {}", device_id);
        Ok(())
    } else {
        Err("Failed to access temporary disconnection tracking".to_string())
    }
}

/// Mark a device as temporarily disconnected
pub fn mark_device_temporarily_disconnected(device_id: &str) -> Result<(), String> {
    if let Ok(mut disconnected_devices) = TEMPORARILY_DISCONNECTED_DEVICES.lock() {
        disconnected_devices.insert(device_id.to_string());
        log::info!("🔌❓ Marked device {} as temporarily disconnected", device_id);
        Ok(())
    } else {
        Err("Failed to access temporary disconnection tracking".to_string())
    }
} 