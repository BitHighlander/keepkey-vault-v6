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

// Development mode: Force all devices to use this specific deviceId
pub const DEV_FORCE_DEVICE_ID: &str = "932313031174732313008100";
pub const DEV_MODE: bool = true; // Set to false for production

/// Get or create a device queue handle for the given device ID
pub async fn get_or_create_device_queue(
    device_id: &str,
    queue_manager: &DeviceQueueManager,
) -> Result<DeviceQueueHandle, String> {
    let mut manager = queue_manager.lock().await;
    
    // Check if we already have a queue for the requested deviceId
    if let Some(existing_handle) = manager.get(device_id) {
        return Ok(existing_handle.clone());
    }
    
    // Get list of connected devices
    let devices = keepkey_rust::features::list_connected_devices();
    
    // In development mode, if requesting the hardcoded deviceId, map to physical device
    let (physical_device, real_device_id) = if DEV_MODE && device_id == DEV_FORCE_DEVICE_ID {
        println!("🛠️ DEV MODE: Looking for any physical KeepKey for deviceId {}", DEV_FORCE_DEVICE_ID);
        
        // Find any connected KeepKey device to use as the physical device
        let physical_device = devices.iter()
            .find(|d| d.is_keepkey)
            .ok_or_else(|| format!("No physical KeepKey device found for development deviceId {}", device_id))?;
            
        println!("🛠️ DEV MODE: Using physical device {} for deviceId {}", physical_device.unique_id, device_id);
        
        // Check if we already have a queue for the real device
        if let Some(existing_handle) = manager.get(&physical_device.unique_id) {
            println!("🛠️ DEV MODE: Reusing existing queue for physical device {}", physical_device.unique_id);
            // Clone the handle to avoid borrow checker issues
            let cloned_handle = existing_handle.clone();
            // Map the hardcoded deviceId to the existing real device queue
            manager.insert(device_id.to_string(), cloned_handle.clone());
            return Ok(cloned_handle);
        }
        
        (physical_device.clone(), physical_device.unique_id.clone())
    } else {
        // Normal mode: find device by exact ID match
        let device = devices.iter()
            .find(|d| d.unique_id == device_id)
            .ok_or_else(|| format!("Device {} not found in connected devices", device_id))?;
        (device.clone(), device_id.to_string())
    };
    
    // Create a new queue handle using the real device ID to avoid duplicates
    println!("🚀 Creating new device worker for physical device: {}", real_device_id);
    let handle = DeviceQueueFactory::spawn_worker(real_device_id.clone(), physical_device);
    
    // Insert the queue under the real device ID
    manager.insert(real_device_id.clone(), handle.clone());
    
    // In dev mode, also map the hardcoded deviceId to the same queue
    if DEV_MODE && device_id == DEV_FORCE_DEVICE_ID && device_id != real_device_id {
        println!("🛠️ DEV MODE: Mapping {} to physical device queue {}", device_id, real_device_id);
        manager.insert(device_id.to_string(), handle.clone());
    }
    
    Ok(handle)
} 