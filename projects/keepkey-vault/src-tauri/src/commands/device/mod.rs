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
pub use check_device_bootloader::check_device_bootloader;
pub use get_devices_needing_setup::get_devices_needing_setup;

// TODO: Add re-exports for other device commands as they are implemented
// pub use get_device_status::get_device_status;
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
    
    if let Some(existing_handle) = manager.get(device_id) {
        // Use existing handle if available
        return Ok(existing_handle.clone());
    }
    
    // Need device info to create queue, get it from keepkey_rust
    let devices = keepkey_rust::features::list_connected_devices();
    let device_info = devices.iter()
        .find(|d| d.unique_id == device_id)
        .ok_or_else(|| format!("Device {} not found in connected devices", device_id))?;
    
    // Create a new queue handle using spawn_worker
    println!("🚀 Creating new device worker for: {}", device_id);
    let handle = DeviceQueueFactory::spawn_worker(device_id.to_string(), device_info.clone());
    manager.insert(device_id.to_string(), handle.clone());
    
    Ok(handle)
} 