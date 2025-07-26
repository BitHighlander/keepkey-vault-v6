use tauri::State;
use std::sync::Arc;
use keepkey_db::Database;
use crate::commands::DeviceQueueManager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceNeedingSetup {
    pub device_id: String,
    pub device_name: String,
    pub serial_number: String,
}

#[tauri::command]
pub async fn get_devices_needing_setup(
    database: tauri::State<'_, Arc<Database>>,
) -> Result<Vec<DeviceNeedingSetup>, String> {
    log::info!("🔍 Checking for devices that need setup...");
    
    // Get all registered devices from the database
    let devices = database.get_device_registry().await
        .map_err(|e| format!("Failed to get devices from database: {}", e))?;
    
    let mut devices_needing_setup = Vec::new();
    
    for device_json in devices {
        // Parse the device JSON
        let device_id = device_json.get("device_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        let device_name = device_json.get("device_name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Device");
        
        let serial_number = device_json.get("serial_number")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
            
        // Check if this device needs setup
        match database.device_needs_setup(device_id).await {
            Ok(needs_setup) => {
                if needs_setup {
                    log::info!("🔍 Device {} needs setup", device_id);
                    devices_needing_setup.push(DeviceNeedingSetup {
                        device_id: device_id.to_string(),
                        device_name: device_name.to_string(),
                        serial_number: serial_number.to_string(),
                    });
                } else {
                    log::debug!("🔍 Device {} setup is complete", device_id);
                }
            }
            Err(e) => {
                log::error!("Failed to check setup status for device {}: {}", device_id, e);
            }
        }
    }
    
    log::info!("🔍 Found {} device(s) that need setup", devices_needing_setup.len());
    Ok(devices_needing_setup)
} 