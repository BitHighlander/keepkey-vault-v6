# KeepKey Rust Native Protocol Implementation

## Overview

This document provides a comprehensive guide to the Rust-native implementation of the KeepKey device protocol. The implementation allows direct communication with KeepKey hardware wallets using Rust, without requiring JavaScript dependencies or bridges.

## Table of Contents

1. [Protocol Architecture](#protocol-architecture)
2. [USB/HID Communication](#usbhid-communication)
3. [Protocol Buffer Integration](#protocol-buffer-integration)
4. [Message Exchange Process](#message-exchange-process)
5. [Firmware Version Extraction](#firmware-version-extraction)
6. [Multi-part Message Handling](#multi-part-message-handling)
7. [Error Handling and Fallbacks](#error-handling-and-fallbacks)
8. [Performance Considerations](#performance-considerations)
9. [Comparison with ShapeShift's Implementation](#comparison-with-shapeshifts-implementation)
10. [Future Improvements](#future-improvements)

## Protocol Architecture

The KeepKey device protocol is based on Google's Protocol Buffers (protobuf) for message serialization and deserialization. The communication occurs over USB HID (Human Interface Device) as the transport layer.

```
┌─────────────────┐     ┌───────────────┐     ┌───────────────┐     ┌───────────────┐
│ Rust            │     │ Protobuf      │     │ USB/HID       │     │ KeepKey       │
│ Application     │────▶│ Messages      │────▶│ Transport     │────▶│ Device        │
└─────────────────┘     └───────────────┘     └───────────────┘     └───────────────┘
```

Key components:

1. **Message Types**: Defined in Protocol Buffer schema files (`.proto`)
2. **HID Transport**: Handles sending/receiving data via USB HID reports
3. **Message Processing**: Serializing/deserializing messages using the Protocol Buffer schema

## USB/HID Communication

The KeepKey communicates via USB HID (Human Interface Device) protocol, which involves sending and receiving data in fixed-size reports. Our implementation handles:

### HID Report Structure

Each HID report has a fixed size of 64 bytes with the following structure:

```
[Report ID (1 byte)] [Magic Bytes (3 bytes)] [Message Type (2 bytes)] [Data Length (4 bytes)] [Data + Padding]
```

- **Report ID**: Usually 0x00 (first byte)
- **Magic Bytes**: '?##' (0x3f, 0x23, 0x23) identifying KeepKey messages
- **Message Type**: 16-bit message identifier
- **Data Length**: 32-bit length of the entire data payload
- **Data + Padding**: The actual payload data plus padding to fill the report

### Multi-part Messages

Since HID reports are limited to 64 bytes, messages exceeding this size must be split across multiple reports:

1. **First Report**: Contains the header and initial chunk of data
2. **Continuation Reports**: Contain subsequent chunks of data

Proper handling of continuation reports is crucial, particularly noting that:
- The first byte of each report is the Report ID (typically 0x00)
- Subsequent continuation reports should be read until the complete message is received
- Timeouts should be implemented to prevent indefinite waiting

## Protocol Buffer Integration

Our implementation uses the `prost` crate for Protocol Buffer support in Rust:

```rust
// Dependencies in Cargo.toml
prost = "0.11.0"
prost-types = "0.11.0"
keepkey-device-protocol = { path = "../device-protocol" }
```

The `keepkey-device-protocol` crate contains the generated Rust code from the Protocol Buffer schemas, providing strongly-typed message definitions:

```rust
use keepkey_device_protocol::messages::{Initialize, Features};
use prost::Message;
```

## Message Exchange Process

The message exchange process follows these steps:

1. **Create a message**: Create a protobuf message instance (e.g., `Initialize`)
2. **Serialize**: Convert the message to a byte array
3. **Send**: Send the message over HID, handling multi-part messages if needed
4. **Receive**: Read the response from the device, handling multi-part responses
5. **Deserialize**: Convert the response bytes back to a protobuf message
6. **Process**: Extract relevant information from the message

### Example: Requesting Device Features

```rust
// Create an Initialize message
let init_msg = Initialize::default();

// Serialize the message to protobuf bytes
let init_data = init_msg.encode_to_vec();

// Send the message and receive the response
let (msg_type, data) = exchange_message(device, MSG_TYPE_INITIALIZE, &init_data)?;

// Deserialize the response
let features = Features::decode(data.as_slice())?;

// Extract information
let firmware_version = format!("{}.{}.{}",
    features.major_version.unwrap_or(0),
    features.minor_version.unwrap_or(0),
    features.patch_version.unwrap_or(0)
);
```

## Firmware Version Extraction

The device firmware version is extracted from the `Features` message, which is returned in response to an `Initialize` message. The version is composed of three components:

```rust
let firmware_version = format!("{}.{}.{}",
    features.major_version.unwrap_or(0), // Major version (e.g., 4)
    features.minor_version.unwrap_or(0), // Minor version (e.g., 0)
    features.patch_version.unwrap_or(0)  // Patch version (e.g., 0)
);
```

This gives us the complete version string (e.g., "4.0.0").

### Manual Field Extraction

For cases where the full protobuf message cannot be decoded (e.g., due to receiving only a partial message), we implement a manual protobuf field extraction routine that can parse critical fields directly from the wire format:

```rust
// Simple protobuf field extraction
let mut i = 0;
while i < data.len() {
    if i + 1 >= data.len() { break; }
    
    let tag = data[i] >> 3;  // Field number
    let wire_type = data[i] & 0x7;  // Wire type
    i += 1;
    
    match (tag, wire_type) {
        // vendor (string, field 1)
        (1, 2) => {
            // Handle string field
        },
        // major_version (uint32, field 2)
        (2, 0) => {
            // Handle integer field
        },
        // ...other fields...
    }
}
```

This approach allows us to extract critical information even when the complete message isn't available.

## Multi-part Message Handling

Messages larger than the HID report size (64 bytes) require special handling:

### Key Improvements

1. **Pre-allocated Buffer**: Create a buffer of the exact expected size upfront
2. **Offset Tracking**: Maintain a running offset when filling the buffer
3. **Report ID Handling**: Skip the Report ID byte in continuation packets
4. **Timeout Management**: Implement reasonable timeouts to prevent freezes

```rust
// Create a properly sized buffer for the entire message
let mut complete_buffer = vec![0u8; HEADER_SIZE + data_len as usize];

// Track our current position in the complete buffer
let mut offset = size;

// Read continuation reports until we have the complete message
while offset < complete_buffer.len() && report_count < MAX_REPORTS {
    // Read the next report
    
    // Skip the report ID byte in continuation packets
    let src_data = &continuation[REPORT_ID_SIZE..cont_size];
    let remaining = complete_buffer.len() - offset;
    let to_copy = std::cmp::min(src_data.len(), remaining);
    
    // Copy this chunk into our buffer at the current offset
    complete_buffer[offset..offset+to_copy].copy_from_slice(&src_data[..to_copy]);
    offset += to_copy;
}
```

## Error Handling and Fallbacks

Our implementation focuses on avoiding mocked values and providing clear errors:

1. **Real Data or Error**: Return real device data or a clear error message
2. **Detailed Errors**: Include diagnostic information about partial messages
3. **No Silent Failures**: Make failures explicit rather than falling back to placeholders

```rust
if vendor.is_none() || major_version.is_none() {
    return Err(format!("Failed to extract complete information from device. \
                      Received {} bytes of {} expected.",
                      data.len(), expected_size));
}
```

## Performance Considerations

Several optimizations prevent performance issues:

1. **Timeouts**: Short timeouts (2 seconds vs. 10 seconds) prevent app freezing
2. **Report Limits**: Cap the number of continuation reports to prevent excessive I/O
3. **Reduced Logging**: Minimize console logging to avoid performance impact
4. **Sleep Intervals**: Shorter sleep durations (5ms vs. 10ms) improve responsiveness

## Comparison with ShapeShift's Implementation

Our implementation was informed by analyzing ShapeShift's `hdwallet` library:

### Key Similarities

1. **Chunking Strategy**: Breaking frames into segments with padding
2. **Buffer Management**: Creating a single buffer for the complete message
3. **Report ID Handling**: Explicitly skipping the USB reportId byte

### Key Differences

1. **Pure Rust**: Our implementation is 100% Rust vs. ShapeShift's TypeScript
2. **Fallback Mechanism**: We added manual protobuf parsing for partial messages
3. **Error Handling**: More detailed error reporting in our implementation

## Future Improvements

Potential areas for future enhancement:

1. **Full Protocol Support**: Implement additional message types beyond Features
2. **Cancellation**: Add support for cancelling in-progress operations
3. **Transaction Signing**: Implement full transaction signing flows
4. **Device Event Handling**: Better handling of button press events
5. **Enhanced Error Recovery**: More sophisticated recovery from communication errors

---

## Appendix: Key Code Snippets

### Exchange Message Function

```rust
fn exchange_message(device: &HidDevice, msg_type: u16, data: &[u8]) -> Result<(u16, Vec<u8>), String> {
    // Prepare and send the message
    let message = prepare_message(msg_type, data);
    
    match device.write(&message) {
        Ok(_) => {},
        Err(e) => return Err(format!("Failed to send message: {}", e)),
    }
    
    // Read the response with a SHORT timeout to prevent freezing
    let start_time = Instant::now();
    let timeout = Duration::from_millis(2000);
    
    // Constants for parsing
    const HEADER_SIZE: usize = 9;
    const REPORT_ID_SIZE: usize = 1;
    
    // Read the first report which contains the header
    let mut first_report = vec![0u8; HID_REPORT_SIZE];
    match device.read_timeout(&mut first_report, 250) {
        Ok(size) if size > 0 => {
            // Parse the header to get message type and length
            match parse_response_header(&first_report[..size]) {
                Ok((msg_type, data_len)) => {
                    // Create a properly sized buffer for the entire message
                    let mut complete_buffer = vec![0u8; HEADER_SIZE + data_len as usize];
                    
                    // Copy the initial report (including header) into the buffer
                    let copy_len = std::cmp::min(size, complete_buffer.len());
                    complete_buffer[..copy_len].copy_from_slice(&first_report[..copy_len]);
                    
                    // Track our current position in the complete buffer
                    let mut offset = size;
                    
                    // Maximum number of continuation reports to read
                    const MAX_REPORTS: usize = 20;
                    let mut report_count = 1;
                    
                    // Read continuation reports until we have the complete message
                    while offset < complete_buffer.len() && report_count < MAX_REPORTS && start_time.elapsed() < timeout {
                        // ...read and process continuation reports...
                    }
                    
                    // Extract the payload (skip the header)
                    let payload = complete_buffer[HEADER_SIZE..offset].to_vec();
                    return Ok((msg_type, payload));
                },
                Err(e) => return Err(format!("Failed to parse header: {}", e)),
            }
        },
        // ...handle other cases...
    }
    
    Err("Failed to read complete message from device".to_string())
}
```
