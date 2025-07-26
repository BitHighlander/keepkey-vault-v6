// commands/device/get_connected_devices.rs

use serde::{Serialize, Deserialize};

// Development mode: Force all devices to use this specific deviceId
const DEV_FORCE_DEVICE_ID: &str = "932313031174732313008100";
const DEV_MODE: bool = true; // Set to false for production

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedDevice {
    pub device_id: String,
    pub name: String,
    pub manufacturer: Option<String>,
    pub vid: u16,
    pub pid: u16,
    pub is_keepkey: bool,
}

/// Get connected devices
#[tauri::command]
pub async fn get_connected_devices() -> Result<Vec<ConnectedDevice>, String> {
    println!("🔍 Getting connected devices");
    
    let devices = keepkey_rust::features::list_connected_devices();
    
    let connected_devices: Vec<ConnectedDevice> = devices
        .into_iter()
        .filter(|device| device.is_keepkey)
        .map(|device| {
            let device_id = if DEV_MODE {
                println!("🛠️ DEV MODE: Mapping device {} to {}", device.unique_id, DEV_FORCE_DEVICE_ID);
                DEV_FORCE_DEVICE_ID.to_string()
            } else {
                device.unique_id
            };
            
            ConnectedDevice {
                device_id,
                name: device.name,
                manufacturer: device.manufacturer,
                vid: device.vid,
                pid: device.pid,
                is_keepkey: device.is_keepkey,
            }
        })
        .collect();
    
    println!("✅ Found {} connected KeepKey devices", connected_devices.len());
    Ok(connected_devices)
} 