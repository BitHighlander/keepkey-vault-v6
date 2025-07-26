// commands/device/get_connected_devices.rs

use serde::{Serialize, Deserialize};

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
            ConnectedDevice {
                device_id: device.unique_id,
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