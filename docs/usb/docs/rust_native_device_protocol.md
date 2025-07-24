# Rust-Native KeepKey Device Protocol Integration

This document explains how to use the new Rust-native KeepKey device protocol implementation (`keepkey-device-protocol` crate) within the KeepKey Desktop Tauri application. The Rust-native approach provides significant advantages over the previous JavaScript-based implementation, including improved type safety, better performance, and simplified integration with the Tauri backend.

## Overview

The `keepkey-device-protocol` crate provides a pure Rust implementation of the KeepKey device protocol, using Protocol Buffers (protobuf) for message serialization and deserialization. This implementation:

1. Uses `prost` for efficient, type-safe Protocol Buffer handling in Rust
2. Provides Rust structs for all device message types
3. Includes serialization/deserialization via `serde` for JSON conversion when needed
4. Eliminates the need for JavaScript interop when handling device messages

## Integration Steps

### 1. Add the Crate Dependency

Add the `keepkey-device-protocol` crate to your Tauri app's `Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...
keepkey-device-protocol = { path = "../../../crates/keepkey-device-protocol" }
```

This adds a path dependency to the Rust crate within the monorepo structure. For production, you might want to publish the crate to crates.io and use a version dependency instead.

### 2. Update Your Rust Backend Code

Replace the current protobuf-related imports and message handling with the new Rust-native approach:

#### Before:

```rust
use protobuf::Message;

// Constants for message types
const MSG_TYPE_INITIALIZE: u16 = 0;
const MSG_TYPE_FEATURES: u16 = 17;

// Manual parsing and serialization
fn request_features(device: &HidDevice) -> Result<FeatureInfo, String> {
    // Send initialize message
    let (msg_type, data) = exchange_message(device, MSG_TYPE_INITIALIZE, &[])?;
    
    if msg_type == MSG_TYPE_FEATURES {
        // Manually parse the protobuf message
        let proto_features = match protobuf::Message::parse_from_bytes(&data) {
            Ok(features) => features,
            Err(e) => return Err(format!("Failed to parse Features message: {}", e)),
        };
        
        // Extract fields manually
        // ...
    } else {
        Err(format!("Unexpected response type: {}", msg_type))
    }
}
```

#### After:

```rust
use keepkey_device_protocol::proto::{Initialize, Features};

fn request_features(device: &HidDevice) -> Result<FeatureInfo, String> {
    // Create an Initialize message
    let init_msg = Initialize::default();
    
    // Serialize the message
    let init_data = init_msg.encode_to_vec();
    
    // Send the message (using existing HID communication code)
    let (msg_type, response_data) = exchange_message(device, 0, &init_data)?;
    
    if msg_type == 17 { // Features message type
        // Parse the response with type safety
        match Features::decode(response_data.as_slice()) {
            Ok(features) => {
                // Access fields through the Rust struct
                let feature_info = FeatureInfo {
                    vendor: features.vendor,
                    device_id: features.device_id,
                    firmware_version: Some(format!(
                        "{}.{}.{}",
                        features.major_version.unwrap_or(0),
                        features.minor_version.unwrap_or(0),
                        features.patch_version.unwrap_or(0)
                    )),
                    bootloader_mode: features.bootloader_mode.unwrap_or(false),
                    bootloader_hash: features.bootloader_hash.map(|bytes| hex::encode(bytes)),
                    revision: features.revision.map(|bytes| hex::encode(bytes)),
                    initialized: features.initialized.unwrap_or(false),
                    pin_protection: features.pin_protection.unwrap_or(false),
                    passphrase_protection: features.passphrase_protection.unwrap_or(false),
                    policies: features.policies.into_iter()
                        .map(|p| PolicyInfo {
                            name: p.policy_name,
                            enabled: p.enabled.unwrap_or(false),
                        })
                        .collect(),
                };
                
                Ok(feature_info)
            },
            Err(e) => Err(format!("Failed to decode Features message: {}", e)),
        }
    } else {
        Err(format!("Unexpected response type: {}", msg_type))
    }
}
```

### 3. Handling Multiple Messages in Device Communication

The new protocol implementation supports the full range of KeepKey messages. Here's how to handle various message types:

```rust
use keepkey_device_protocol::proto::{
    Initialize, Features, GetAddress, Address,
    SignTx, TxRequest, ButtonRequest, ButtonAck
};

// Example: Request Bitcoin address
fn request_bitcoin_address(device: &HidDevice, path: &[u32]) -> Result<String, String> {
    // Create GetAddress message
    let mut get_address = GetAddress::default();
    get_address.address_n = path.to_vec();
    get_address.coin_name = Some("Bitcoin".to_string());
    get_address.show_display = Some(true);
    
    // Serialize and send
    let address_data = get_address.encode_to_vec();
    let (msg_type, response_data) = exchange_message(device, 29, &address_data)?; // 29 = GetAddress type
    
    // Handle response based on message type
    match msg_type {
        // Address response
        30 => { // Address type
            match Address::decode(response_data.as_slice()) {
                Ok(address) => Ok(address.address.unwrap_or_default()),
                Err(e) => Err(format!("Failed to decode Address: {}", e)),
            }
        },
        // Button request (need user interaction)
        26 => { // ButtonRequest type
            // Send ButtonAck
            let button_ack = ButtonAck::default();
            let ack_data = button_ack.encode_to_vec();
            
            // Send acknowledgment and wait for next response
            let (next_msg_type, next_data) = exchange_message(device, 27, &ack_data)?; // 27 = ButtonAck type
            
            // Recursively handle the next response
            // ...
            
            Err("ButtonRequest handling not fully implemented".to_string())
        },
        // Other message types
        _ => Err(format!("Unexpected message type: {}", msg_type)),
    }
}
```

### 4. Handling Large Messages and Multi-part Responses

When dealing with large Protocol Buffer messages that span multiple HID reports, you need to modify the `exchange_message` function to collect all parts of the message before decoding:

```rust
fn exchange_message(
    device: &HidDevice, 
    msg_type: u16, 
    data: &[u8]
) -> Result<(u16, Vec<u8>), String> {
    // Prepare and send the message (existing code)
    let message = prepare_message(msg_type, data);
    // ... (send the message)
    
    // Read the first response to get header info
    let mut buf = [0u8; HID_REPORT_SIZE];
    if device.read(&mut buf).map_err(|e| format!("Error reading response: {}", e))? <= 0 {
        return Err("No data received from device".to_string());
    }
    
    // Parse the header
    let (response_type, mut payload, total_msg_size) = parse_header(&buf)?;
    
    // Calculate how many more reports we need to read
    let bytes_per_report = HID_REPORT_SIZE - HEADER_SIZE; // Usable bytes per report after header
    let bytes_remaining = total_msg_size.saturating_sub(payload.len());
    let reports_needed = (bytes_remaining + bytes_per_report - 1) / bytes_per_report;
    
    // Read additional reports if needed
    for _ in 0..reports_needed {
        let mut next_buf = [0u8; HID_REPORT_SIZE];
        if device.read(&mut next_buf).map_err(|e| format!("Error reading continuation: {}", e))? <= 0 {
            return Err("Incomplete message received".to_string());
        }
        
        // For continuation packets, skip header and just take payload
        payload.extend_from_slice(&next_buf[HEADER_SIZE..]);
    }
    
    // Trim payload to expected length
    if payload.len() > total_msg_size {
        payload.truncate(total_msg_size);
    }
    
    Ok((response_type, payload))
}
```

## Usage Examples

### 1. Basic Device Initialization

```rust
use keepkey_device_protocol::proto::{Initialize, Features};
use hidapi::{HidApi, HidDevice};

fn initialize_device() -> Result<Features, String> {
    // Initialize HidApi
    let api = HidApi::new().map_err(|e| format!("Failed to initialize HidApi: {}", e))?;
    
    // Open KeepKey device (VID=0x2B24, PID=0x0001)
    let device = api.open(0x2B24, 0x0001)
        .map_err(|e| format!("Failed to open KeepKey device: {}", e))?;
    
    // Create Initialize message
    let init_msg = Initialize::default();
    
    // Send message and receive response
    let init_data = init_msg.encode_to_vec();
    let (msg_type, response_data) = exchange_message(&device, 0, &init_data)?;
    
    if msg_type == 17 { // Features type
        Features::decode(response_data.as_slice())
            .map_err(|e| format!("Failed to decode Features: {}", e))
    } else {
        Err(format!("Unexpected response type: {}", msg_type))
    }
}
```

### 2. Integration with Tauri Commands

```rust
use keepkey_device_protocol::proto::{Features, GetFeatures};
use tauri::{command, State};

// Define a struct to hold device state in Tauri
struct DeviceState {
    features: Option<Features>,
}

#[command]
fn get_device_info(state: State<DeviceState>) -> Result<serde_json::Value, String> {
    // Check if device is initialized
    if let Some(features) = &state.features {
        // Convert to JSON using serde
        serde_json::to_value(features)
            .map_err(|e| format!("Failed to serialize device info: {}", e))
    } else {
        Err("Device not initialized".to_string())
    }
}

// In main.rs
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Initialize device state
            app.manage(DeviceState { features: None });
            
            // Start device detection (in a separate thread)
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                // Detect and initialize device
                match initialize_device() {
                    Ok(features) => {
                        // Update device state
                        let state = app_handle.state::<DeviceState>();
                        *state.features.lock().unwrap() = Some(features);
                        
                        // Notify frontend
                        app_handle.emit_all("device-connected", ()).ok();
                    },
                    Err(e) => {
                        eprintln!("Failed to initialize device: {}", e);
                    }
                }
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_device_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Benefits Over JavaScript Implementation

The Rust-native KeepKey device protocol offers several advantages:

1. **Type Safety**: All Protocol Buffer messages are strongly typed in Rust, preventing many potential runtime errors that could occur with dynamically-typed JavaScript.

2. **Performance**: Native Rust serialization and deserialization is significantly faster than JavaScript, especially for complex nested structures.

3. **Reduced Dependencies**: Eliminates the need for JavaScript interop and the `@keepkey/device-protocol` npm package, simplifying the build process.

4. **Better Error Handling**: Rust's Result type provides clearer error propagation and handling.

5. **WASM Compatibility**: The same Rust code can be compiled to WebAssembly if needed for browser environments.

6. **Single Source of Truth**: One Rust crate defines the protocol for all parts of the system, ensuring consistency.

## Development Workflow

When developing with the Rust-native device protocol:

1. **Running in Development Mode**: Use `cargo tauri dev` to start the application. Note that the command is configured with a timeout to prevent indefinite hangs during development.

2. **Debugging**: Rust's strong type system helps catch many errors at compile time, but you can also use `println!` or the Tauri logging system for runtime debugging.

3. **Protocol Updates**: When the device protocol changes, update the `.proto` files in the `keepkey-device-protocol` crate and rebuild.

## Conclusion

Adopting the Rust-native KeepKey device protocol implementation provides a more robust, maintainable, and efficient foundation for device communication in the KeepKey Desktop application. By leveraging Rust's strong type system and performance, along with the `prost` Protocol Buffer implementation, the application can handle device interactions more reliably while providing a better developer experience.
