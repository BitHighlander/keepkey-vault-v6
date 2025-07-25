// commands/mod.rs - Organized command modules

// Core types used across commands
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use keepkey_rust::device_queue::DeviceQueueHandle;

pub type DeviceQueueManager = Arc<Mutex<HashMap<String, DeviceQueueHandle>>>;

// Command modules organized by functionality
pub mod device;
pub mod pin;
pub mod recovery; 
pub mod verification;
pub mod logging;
pub mod config;
pub mod api;
pub mod cache;
pub mod test;

// Event handling utilities
pub mod events;

// Re-export commonly used functions
pub use events::{emit_or_queue_event, frontend_ready};
pub use device::{get_connected_devices, get_features, check_device_bootloader};
pub use config::{is_first_time_install, is_onboarded, set_onboarding_completed, get_preference, set_preference, debug_onboarding_state}; 