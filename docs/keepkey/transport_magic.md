# KeepKey Transport Protocol Magic 🪄

## Overview

This document captures all the "magic" details and gotchas about KeepKey transport protocols. These details are critical for correct implementation but easy to get wrong. 

**⚠️ WARNING**: Getting any of these details wrong will result in "invalid tag value: 0" or similar protobuf decoding errors that are very hard to debug!

## The Three Transport Types

1. **USB (via rusb)** - Direct USB communication with full control
2. **HID (via hidapi)** - Human Interface Device protocol, often better for permissions
3. **WebUSB** - Browser-based USB access (limited by security model)

## Protocol Message Format

### High-Level Protocol (All Transports)

All KeepKey messages follow this protobuf-encoded format:

```
[Magic Header] [Message Type] [Data Length] [Protobuf Data]
```

- **Magic Header**: `##` (0x23 0x23) - 2 bytes
- **Message Type**: Big-endian u16 - 2 bytes  
- **Data Length**: Big-endian u32 - 4 bytes
- **Protobuf Data**: Variable length encoded protobuf message

Total header size: 8 bytes

### ⚠️ Critical Gotcha #1: Double Encoding

The v5 `Message::encode()` already adds the `##` header! Don't add it again in the transport layer.

```rust
// DON'T DO THIS - Double encoding!
let mut buf = vec![];
buf.push(b'#');
buf.push(b'#');
// ... then call Message::encode()

// DO THIS - Message::encode() handles it
let mut buf = Vec::with_capacity(msg.encoded_len());
msg.encode(&mut buf)?;
```

## USB Transport (rusb)

### Packet Format

USB uses 64-byte packets with this format:

**First packet:**
```
[?][Message with protocol header][Padding to 64 bytes]
```

**Continuation packets:**
```
[?][Data continuation][Padding to 64 bytes]
```

- Always starts with `?` (0x3F) prefix
- 63 bytes available for data per packet
- Packets are always exactly 64 bytes (padded with zeros)

### Example USB Write

```rust
// First packet
packet[0] = b'?';  // USB packet marker
packet[1..].copy_from_slice(&message_data);  // Already has ## header from encode()

// Continuation packets  
packet[0] = b'?';
packet[1..64].copy_from_slice(&remaining_data);
```

## HID Transport (hidapi)

### ⚠️ Critical Gotcha #2: HID Uses Legacy Format!

HID transport uses the **OLD v4 protocol format**, not the v5 format!

**v4 Format (what HID expects):**
```
[Report ID][0x3F][0x23][0x23][Message Type][Data Length][Data]
```

- **Report ID**: 0x00 - 1 byte
- **Magic bytes**: `[0x3F, 0x23, 0x23]` - 3 bytes (NOTE: Different from v5!)
- **Message Type**: Big-endian u16 - 2 bytes
- **Data Length**: Big-endian u32 - 4 bytes  
- **Data**: Variable length

Total v4 header size: 10 bytes (vs 8 in v5)

### HID Packet Format

**First packet:**
```
[0x00][0x3F][0x23][0x23][Type][Length][Data up to 54 bytes]
```

**Continuation packets:**
```
[?][Data continuation up to 63 bytes]
```

### ⚠️ Critical Gotcha #3: Format Translation

Since v5 uses `##` format but HID expects v4 format, you must translate:

```rust
// Incoming v5 format: [#][#][type][length][data]
// Convert to v4 format for HID: [0][0x3F][0x23][0x23][type][length][data]

// Extract from v5 format (skip ##)
let msg_type = &msg[2..4];
let msg_length = &msg[4..8];  
let msg_data = &msg[8..];

// Build v4 packet
packet[0] = 0x00;      // Report ID
packet[1] = 0x3F;      // Magic 1
packet[2] = 0x23;      // Magic 2  
packet[3] = 0x23;      // Magic 3
packet[4..6].copy_from_slice(msg_type);
packet[6..10].copy_from_slice(msg_length);
// ... then data
```

### HID Read Translation

When reading, translate back from v4 to v5 format:

```rust
// Read v4 format: [0x3F][0x23][0x23][type][length][data]
// Convert to v5 format: [#][#][type][length][data]

buf.push(b'#');
buf.push(b'#');
buf.extend_from_slice(&packet[3..9]); // type + length
buf.extend_from_slice(&data);         // actual data
```

## WebUSB Transport

### Browser Security Model

WebUSB has additional restrictions:

1. **No HID access** - Browsers block HID class devices
2. **User permission required** - Must request device access
3. **Configuration limitations** - Cannot change USB configurations
4. **Endpoint restrictions** - Limited to specific endpoint types

### WebUSB Packet Format

WebUSB typically uses the same format as USB transport:

```javascript
// First packet
const packet = new Uint8Array(64);
packet[0] = 0x3F; // '?' prefix
// Copy message data (includes ## header)

// Send via WebUSB
await device.transferOut(ENDPOINT, packet);
```

### ⚠️ Critical Gotcha #4: Browser Buffering

Browsers may buffer USB transfers. Always:
- Flush after writing
- Handle fragmented reads
- Implement proper timeouts

## Common Message Types

| Message | Type ID | Direction | Description |
|---------|---------|-----------|-------------|
| Initialize | 0 | To Device | First message to establish connection |
| Features | 17 | From Device | Device capabilities and state |
| Ping | 1 | To Device | Keep-alive / test message |
| Success | 2 | From Device | Operation completed |
| Failure | 3 | From Device | Operation failed |

## Debugging Transport Issues

### 1. "Invalid tag value: 0" Error

This means protobuf decoding failed. Common causes:
- Wrong magic bytes (using `##` instead of `[0x3F, 0x23, 0x23]` for HID)
- Missing or incorrect header translation
- Incomplete message (didn't read all continuation packets)
- Double-encoded headers

### 2. Device Not Responding

- Check Report ID (must be 0x00 for HID)
- Verify magic bytes match expected format
- Ensure complete message is sent (all chunks)
- Check USB permissions

### 3. Partial Messages

Always read continuation packets:

```rust
while bytes_received < expected_length {
    // Read next packet
    // Skip continuation marker if present
    // Append data
}
```

## Implementation Checklist

When implementing a transport:

- [ ] Correct magic bytes for the transport type
- [ ] Proper header size (8 bytes for v5, 10 for v4/HID)
- [ ] Report ID handling (HID only)
- [ ] Continuation packet markers (`?` prefix)
- [ ] Complete message assembly (multiple packets)
- [ ] Format translation if needed (v5 ↔ v4)
- [ ] Proper error handling for incomplete messages
- [ ] Timeout handling for reads

## Test Your Implementation

1. **Initialize Test**: Send Initialize (type 0), expect Features (type 17)
2. **Large Message Test**: Send/receive messages larger than 64 bytes
3. **Error Test**: Send invalid message, expect Failure response
4. **Timeout Test**: Ensure timeouts work correctly

## References

- [v4 Implementation](../../../keepkey-desktop-v4/src-tauri/src/device/backends/hid.rs) - Working HID reference
- [USB HID Specification](https://www.usb.org/hid) - Official USB HID docs
- [WebUSB API](https://wicg.github.io/webusb/) - Browser WebUSB specification

---

**Remember**: When in doubt, check packet dumps! The device is very particular about format. 