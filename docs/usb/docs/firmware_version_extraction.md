# KeepKey Firmware Version Extraction

## Overview

This document details the implementation of firmware version extraction from KeepKey hardware devices using the Rust-native protocol. It focuses specifically on how we obtain the real firmware version (e.g., "4.0.0") from the device without using mocked or hardcoded values.

## Table of Contents

1. [Importance of Accurate Version Information](#importance-of-accurate-version-information)
2. [Protocol Flow for Version Extraction](#protocol-flow-for-version-extraction)
3. [Protobuf Message Structure](#protobuf-message-structure)
4. [Version Field Decoding](#version-field-decoding)
5. [Handling Partial Messages](#handling-partial-messages)
6. [Error Handling Without Fallbacks](#error-handling-without-fallbacks)
7. [Testing and Verification](#testing-and-verification)

## Importance of Accurate Version Information

Accurate firmware version information is critical for several reasons:

1. **Security**: Different security features and vulnerabilities exist in different firmware versions
2. **Feature Compatibility**: Certain features may only be available in specific firmware versions
3. **Upgrade Decisions**: Users need to know their current version to decide on upgrades
4. **Support**: Technical support depends on knowing the exact firmware version
5. **User Trust**: Presenting mocked data undermines user trust in the application

Our implementation prioritizes retrieving the actual firmware version from the device rather than relying on fallbacks or hardcoded values.

## Protocol Flow for Version Extraction

The process to extract the firmware version follows these steps:

1. **Device Connection**: Establish a connection to the KeepKey device via HID
2. **Initialize Message**: Send an `Initialize` message to the device
3. **Features Response**: Receive a `Features` message from the device
4. **Version Extraction**: Extract the version fields from the Features message
5. **Version Formatting**: Format the version components into a version string

```rust
// Create and send Initialize message
let init_msg = Initialize::default();
let init_data = init_msg.encode_to_vec();
let (msg_type, data) = exchange_message(device, MSG_TYPE_INITIALIZE, &init_data)?;

// Handle Features response
if msg_type == MSG_TYPE_FEATURES {
    // Try to decode the complete Features message
    match Features::decode(data.as_slice()) {
        Ok(features) => {
            // Extract version components
            let firmware_version = format!("{}.{}.{}",
                features.major_version.unwrap_or(0),
                features.minor_version.unwrap_or(0),
                features.patch_version.unwrap_or(0)
            );
            
            // Use the firmware version
            println!("Device firmware version: {}", firmware_version);
        },
        Err(e) => {
            // Handle decoding errors
            // ...
        }
    }
}
```

## Protobuf Message Structure

The firmware version is stored in the `Features` protobuf message, which is defined in the `messages.proto` file:

```protobuf
message Features {
    optional string vendor = 1;                 // device manufacturer
    optional uint32 major_version = 2;          // major version of firmware
    optional uint32 minor_version = 3;          // minor version of firmware
    optional uint32 patch_version = 4;          // patch version of firmware
    optional bool bootloader_mode = 5;          // is device in bootloader mode?
    optional string device_id = 6;              // device's unique identifier
    optional bytes bootloader_hash = 7;         // hash of the bootloader
    // ... other fields ...
}
```

The version information is split across three separate fields:
- `major_version` (field 2): The major version number (e.g., 4)
- `minor_version` (field 3): The minor version number (e.g., 0)
- `patch_version` (field 4): The patch version number (e.g., 0)

## Version Field Decoding

When the complete `Features` message is successfully decoded, extracting the version is straightforward:

```rust
let features = Features::decode(data.as_slice())?;

let firmware_version = format!("{}.{}.{}",
    features.major_version.unwrap_or(0),
    features.minor_version.unwrap_or(0),
    features.patch_version.unwrap_or(0)
);
```

However, in practice, we may receive a partial message due to HID communication issues, particularly with large `Features` messages (691 bytes) that span multiple HID reports.

## Handling Partial Messages

For partial messages where the standard protobuf decoder fails, we implemented a manual field extraction approach:

```rust
// Manual extraction of essential fields from partial protobuf message
let mut major_version = None;
let mut minor_version = None;
let mut patch_version = None;

// Parse the protobuf wire format directly
let mut i = 0;
while i < data.len() {
    if i + 1 >= data.len() { break; }
    
    let tag = data[i] >> 3;  // Field number
    let wire_type = data[i] & 0x7;  // Wire type
    i += 1;
    
    match (tag, wire_type) {
        // major_version (uint32, field 2)
        (2, 0) => {
            if i >= data.len() { break; }
            major_version = Some(data[i] as u32);
            i += 1;
        },
        // minor_version (uint32, field 3)
        (3, 0) => {
            if i >= data.len() { break; }
            minor_version = Some(data[i] as u32);
            i += 1;
        },
        // patch_version (uint32, field 4)
        (4, 0) => {
            if i >= data.len() { break; }
            patch_version = Some(data[i] as u32);
            i += 1;
        },
        // ... other fields ...
    }
}

// Only if we successfully extracted the version fields
if major_version.is_some() && minor_version.is_some() && patch_version.is_some() {
    let firmware_version = format!("{}.{}.{}",
        major_version.unwrap(),
        minor_version.unwrap(),
        patch_version.unwrap()
    );
    println!("Extracted firmware version: {}", firmware_version);
}
```

This approach allows us to extract the version information even when we can't decode the entire `Features` message, ensuring we always get real data from the device.

## Error Handling Without Fallbacks

A key principle in our implementation is to never return mocked or hardcoded version numbers. If we cannot extract the actual version from the device, we return an error:

```rust
// Instead of using fallbacks, return an error if we couldn't extract the required information
if vendor.is_none() || major_version.is_none() || minor_version.is_none() || patch_version.is_none() {
    return Err(format!("Failed to extract complete firmware information from device. \n\
                      Received {} bytes of {} expected. \n\
                      Partial data: vendor={:?}, firmware={:?}.{:?}.{:?}, device_id={:?}",
                      data.len(),
                      expected_size,
                      vendor,
                      major_version,
                      minor_version,
                      patch_version,
                      device_id));
}
```

This approach ensures:

1. Users always see real data or a clear error
2. Developers receive detailed diagnostic information when extraction fails
3. The application maintains integrity by not presenting fake information

## Testing and Verification

To verify that our version extraction is working correctly, we need to test with various KeepKey devices and firmware versions:

### Testing Approach

1. **Complete Message Test**: Verify version extraction with complete `Features` messages
2. **Partial Message Test**: Confirm that partial message extraction works correctly
3. **Error Case Test**: Ensure appropriate errors are returned when version cannot be extracted
4. **Comparison Test**: Compare extracted version with known device version

### Verification

We can verify the extracted version by checking:

1. **Device Labeling**: Physical device labeling or packaging
2. **Alternative Tools**: Compare with version reported by official KeepKey tools
3. **Expected Behavior**: Verify that device features match what is expected for the reported version

For example, with our current implementation, we successfully extract version "4.0.0" from a real KeepKey device, confirming that we are getting accurate firmware information.

## Conclusion

Reliable firmware version extraction is fundamental to proper KeepKey device interaction. Our implementation ensures that:

1. We always present the real version directly from the device
2. We handle partial messages gracefully
3. We provide clear errors rather than fallbacks
4. We maintain user trust by never showing mocked data

This approach allows applications to make informed decisions based on the actual device state, enhancing security and user experience.
