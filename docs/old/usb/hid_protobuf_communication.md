# KeepKey HID Protocol and Protobuf Communication

## Overview

This document details the low-level communication between the KeepKey Desktop application and KeepKey hardware devices. It focuses specifically on the USB/HID transport layer and Protocol Buffer message handling.

## Table of Contents

1. [HID Communication Basics](#hid-communication-basics)
2. [Message Framing](#message-framing)
3. [Multi-part Messages](#multi-part-messages)
4. [Protobuf Wire Format](#protobuf-wire-format)
5. [Message Types](#message-types)
6. [Handling Partial Messages](#handling-partial-messages)
7. [Performance Optimizations](#performance-optimizations)

## HID Communication Basics

The KeepKey device uses USB HID (Human Interface Device) as its communication protocol. This choice provides broad compatibility across operating systems without requiring custom drivers.

### Device Identification

KeepKey devices can be identified by their USB Vendor ID (VID) and Product ID (PID):

```
VID: 0x2B24 (ShapeShift)
PID: 0x0001 (Legacy/OOB mode)
PID: 0x0002 (Firmware mode)
```

### Report Structure

HID communication uses fixed-size reports (64 bytes for KeepKey). Each report follows this structure:

```
Byte 0: Report ID (typically 0x00)
Bytes 1-63: Report Data
```

## Message Framing

KeepKey messages are framed within HID reports using a specific header structure:

```
┌───────────┬───────────┬───────────┬─────────────┬────────────┬──────────┐
│ Report ID │ Magic 1   │ Magic 2   │ Magic 3     │ Msg Type   │ Length   │
│ (1 byte)  │ (1 byte)  │ (1 byte)  │ (1 byte)    │ (2 bytes)  │ (4 bytes)│
├───────────┼───────────┼───────────┼─────────────┼────────────┼──────────┤
│ 0x00      │ 0x3F ('?')│ 0x23 ('#')│ 0x23 ('#')  │ BE uint16  │ BE uint32│
└───────────┴───────────┴───────────┴─────────────┴────────────┴──────────┘
```

Followed by the message payload and optional padding to fill the report.

### Header Parsing

```rust
// Parse a response from the KeepKey device
fn parse_response_header(data: &[u8]) -> Result<(u16, u32), String> {
    if data.len() < 9 {  // Need at least 9 bytes for header
        return Err("Response too short".to_string());
    }
    
    // Check magic bytes
    if data[0] != 0x3f || data[1] != 0x23 || data[2] != 0x23 {
        return Err(format!("Invalid magic bytes: {:02x} {:02x} {:02x}", 
                          data[0], data[1], data[2]));
    }
    
    // Extract message type
    let msg_type = BigEndian::read_u16(&data[3..5]);
    
    // Extract data length
    let data_len = BigEndian::read_u32(&data[5..9]);
    
    Ok((msg_type, data_len))
}
```

## Multi-part Messages

Many KeepKey messages (especially `Features` responses) exceed the 64-byte HID report size. These must be split across multiple reports:

### First Report

The first report contains:
- The complete header (9 bytes)
- The initial chunk of the payload (up to 55 bytes)

### Continuation Reports

Subsequent reports contain:
- Report ID (byte 0)
- Continuation of payload data (bytes 1-63)

### Handling Multi-part Messages

The optimal approach is to:

1. Read the first report and extract the expected total message length
2. Allocate a buffer of the exact required size
3. Read continuation reports until the buffer is filled
4. Process the complete message

```rust
// Create a properly sized buffer for the entire message
let mut complete_buffer = vec![0u8; HEADER_SIZE + data_len as usize];

// Copy the initial report (including header) into the buffer
let copy_len = std::cmp::min(size, complete_buffer.len());
complete_buffer[..copy_len].copy_from_slice(&first_report[..copy_len]);

// Track our current position in the complete buffer
let mut offset = size;

// Read continuation reports until we have the complete message
while offset < complete_buffer.len() && report_count < MAX_REPORTS {
    let mut continuation = vec![0u8; HID_REPORT_SIZE];
    match device.read_timeout(&mut continuation, 100) {
        Ok(cont_size) if cont_size > 0 => {
            // Skip the report ID byte in continuation packets
            let src_data = &continuation[REPORT_ID_SIZE..cont_size];
            let remaining = complete_buffer.len() - offset;
            let to_copy = std::cmp::min(src_data.len(), remaining);
            
            // Copy this chunk into our buffer at the current offset
            complete_buffer[offset..offset+to_copy].copy_from_slice(&src_data[..to_copy]);
            offset += to_copy;
        },
        // ...handle other cases...
    }
}
```

## Protobuf Wire Format

Protocol Buffers use a binary wire format to encode messages. Understanding this format is critical for manual parsing when needed.

### Field Encoding

Each field in a protobuf message is encoded as:

```
(field_number << 3) | wire_type
```

Where:
- `field_number` is the field's tag in the `.proto` file
- `wire_type` identifies the encoding:
  - 0: Varint (int32, int64, uint32, uint64, bool, enum)
  - 1: 64-bit (fixed64, sfixed64, double)
  - 2: Length-delimited (string, bytes, embedded messages)
  - 5: 32-bit (fixed32, sfixed32, float)

### Manual Field Extraction

For partial messages, we can extract fields directly from the wire format:

```rust
let mut i = 0;
while i < data.len() {
    if i + 1 >= data.len() { break; }
    
    let tag = data[i] >> 3;  // Field number
    let wire_type = data[i] & 0x7;  // Wire type
    i += 1;
    
    match (tag, wire_type) {
        // vendor (string, field 1)
        (1, 2) => {
            if i >= data.len() { break; }
            let len = data[i] as usize;
            i += 1;
            if i + len > data.len() { break; }
            
            vendor = Some(String::from_utf8_lossy(&data[i..i+len]).to_string());
            i += len;
        },
        // device_id (string, field 10)
        (10, 2) => {
            // Similar string handling...
        },
        // major_version (uint32, field 2)
        (2, 0) => {
            if i >= data.len() { break; }
            major_version = Some(data[i] as u32);
            i += 1;
        },
        // ...other fields...
    }
}
```

## Message Types

The KeepKey protocol defines message types in its `.proto` files. Key message types include:

### Initialize / Features Exchange

The primary handshake for device communication:

1. Send `Initialize` message (type 0)
2. Receive `Features` message (type 17)

```rust
// Create an Initialize message
let init_msg = Initialize::default();

// Serialize and send it
let init_data = init_msg.encode_to_vec();
let (msg_type, data) = exchange_message(device, MSG_TYPE_INITIALIZE, &init_data)?;

// Parse the Features response
if msg_type == MSG_TYPE_FEATURES {
    let features = Features::decode(data.as_slice())?;
    
    // Extract information
    let firmware_version = format!("{}.{}.{}",
        features.major_version.unwrap_or(0),
        features.minor_version.unwrap_or(0),
        features.patch_version.unwrap_or(0)
    );
}
```

## Handling Partial Messages

In practical implementations, we may not always receive the complete message due to:
- USB timing issues
- Device disconnection
- Buffer limitations

Our approach is to:

1. Set reasonable timeouts (2 seconds)
2. Limit the number of continuation reports (20 max)
3. Extract what information we can from partial messages
4. Return clear errors rather than mocked data

### Example: Features Message

The `Features` message from a KeepKey device can be up to 691 bytes, which requires 12 HID reports:
- 1 initial report with header (64 bytes)
- 11 continuation reports (64 bytes each)

If we only receive 660 bytes (11 reports), we can still extract critical fields like:
- Vendor
- Device ID
- Firmware version

## Performance Optimizations

Several optimizations were implemented to prevent performance issues:

1. **Short Timeouts**: Use 2-second timeouts instead of 10 seconds
2. **Report Limits**: Cap at 20 continuation reports
3. **Efficient Buffer Management**: Pre-allocate a single buffer of the exact size needed
4. **Minimal Logging**: Reduce console logging
5. **Non-Blocking Reads**: Use shorter read timeouts (100ms) within the overall timeout
6. **Sleep Management**: Use 5ms sleeps to balance responsiveness vs. CPU usage

### Example: Exchange Message Function

```rust
fn exchange_message(device: &HidDevice, msg_type: u16, data: &[u8]) -> Result<(u16, Vec<u8>), String> {
    // Prepare and send the message
    let message = prepare_message(msg_type, data);
    
    match device.write(&message) {
        Ok(_) => {},
        Err(e) => return Err(format!("Failed to send message: {}", e)),
    }
    
    // Read with a SHORT timeout to prevent freezing
    let start_time = Instant::now();
    let timeout = Duration::from_millis(2000);
    
    // Constants
    const HEADER_SIZE: usize = 9;
    const REPORT_ID_SIZE: usize = 1;
    
    // First report handling...
    
    // Continuation report handling with optimizations...
    
    // Extract the payload and return...
}
```

---

## Conclusion

Effective communication with KeepKey devices requires careful handling of both the HID transport layer and Protocol Buffer messages. The multi-part message handling is particularly critical for correctly processing larger messages like the `Features` response.

Our implementation prioritizes:
- Correctness: Getting real data from the device
- Performance: Preventing application freezes
- Robustness: Handling partial messages when possible
- Error clarity: Providing meaningful errors rather than mocked data

By understanding and implementing these patterns, we've created a reliable Rust-native implementation for KeepKey device communication.
