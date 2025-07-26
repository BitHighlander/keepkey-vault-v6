// commands/device/mod.rs - Device operation commands

pub mod get_connected_devices;
pub mod get_features;
pub mod get_device_status;
pub mod wipe_device;
pub mod set_device_label;
pub mod get_device_info_by_id;
pub mod get_queue_status;
pub mod get_blocking_actions;
pub mod check_device_bootloader;
pub mod register_device;
pub mod get_devices_needing_setup;

// Re-export command functions
pub use get_connected_devices::get_connected_devices;
pub use get_features::get_features;
pub use get_device_status::get_device_status;
pub use check_device_bootloader::check_device_bootloader;
pub use get_devices_needing_setup::get_devices_needing_setup;

// TODO: Add re-exports for other device commands as they are implemented
// pub use wipe_device::wipe_device;
// pub use set_device_label::set_device_label;
// pub use get_device_info_by_id::get_device_info_by_id;
// pub use get_queue_status::get_queue_status;
// pub use get_blocking_actions::get_blocking_actions;
// pub use register_device::{register_device, get_device_registry, get_device_from_registry, 
//                          update_device_setup_step, mark_device_setup_complete, 
//                          device_needs_setup, get_incomplete_setup_devices, reset_device_setup};

// Shared utilities for device commands
use crate::commands::DeviceQueueManager;
use keepkey_rust::device_queue::{DeviceQueueFactory, DeviceQueueHandle};

/// Get or create a device queue handle for the given device ID
pub async fn get_or_create_device_queue(
    device_id: &str,
    queue_manager: &DeviceQueueManager,
) -> Result<DeviceQueueHandle, String> {
    let mut manager = queue_manager.lock().await;
    
    // Check if we already have a queue for the requested deviceId
    if let Some(existing_handle) = manager.get(device_id) {
        // Check if device was temporarily disconnected - if so, we need a fresh queue
        if crate::commands::was_device_temporarily_disconnected(device_id) {
            log::info!("🔄 Device {} was temporarily disconnected - recreating queue for fresh transport", device_id);
            manager.remove(device_id);
            // Clear the temporary disconnection flag
            let _ = crate::commands::clear_temporary_disconnection(device_id);
        } else {
            return Ok(existing_handle.clone());
        }
    }
    
    // Get list of connected devices
    let devices = keepkey_rust::features::list_connected_devices();
    
    // Find device by multiple strategies:
    // 1. Exact ID match
    // 2. Canonical ID match (for aliases)
    // 3. Check if any device might be the same physical device
    let device = devices.iter()
        .find(|d| d.unique_id == device_id)
        .or_else(|| {
            // Check if this device might be aliased (for bootloader mode transitions)
            let canonical_id = crate::commands::get_canonical_device_id(device_id);
            if canonical_id != device_id {
                devices.iter().find(|d| d.unique_id == canonical_id)
            } else {
                None
            }
        })
        .or_else(|| {
            // Check if any connected device might be the same physical device
            devices.iter().find(|d| {
                crate::commands::are_devices_potentially_same(&d.unique_id, device_id)
            })
        })
        .ok_or_else(|| format!("Device {} not found in connected devices", device_id))?;
    
    // Create a new queue handle
    log::info!("🚀 Creating new device worker for device: {}", device_id);
    let handle = DeviceQueueFactory::spawn_worker(device_id.to_string(), device.clone());
    
    // Insert the queue under the device ID
    manager.insert(device_id.to_string(), handle.clone());
    
    Ok(handle)
} 