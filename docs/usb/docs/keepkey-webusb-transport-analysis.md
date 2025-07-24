# KeepKey WebUSB Transport Protocol Analysis

## Executive Summary

After extensive investigation and implementation attempts, we've discovered critical insights about KeepKey's dual transport system and protocol requirements. This document captures all findings for future reference.

## Key Discoveries

### 1. KeepKey Has TWO Different PIDs for Different Transport Methods

```
VID: 0x2B24 (KeepKey)
PID: 0x0001 = HID mode (limited functionality)
PID: 0x0002 = WebUSB mode (full API access)
```

### 2. Transport Priority in Production Code

From production KeepKey Desktop (`walletUtils.ts`):
```typescript
// PRODUCTION PRIORITY:
// 1. FIRST: WebUSB (full functionality)
// 2. SECOND: HID (fallback, limited)

const webUsbDevice = await webUsbAdapter.getDevice()
if (webUsbDevice) {
  return webUsbDevice; // ✅ Full API access
}

// Fallback to HID only if WebUSB fails
const hidDevice = await hidAdapter.getDevice()
```

**Critical Finding**: Production code **REQUIRES WebUSB for full functionality**. HID is only a fallback with severe limitations.

## Protocol Analysis

### HID Transport (PID 0x0001) - Limited
- **Endpoint**: Interrupt transfers via HID
- **Access**: `hidapi::open(0x2B24, 0x0001)`
- **Limitations**: 
  - Cannot access debug interface
  - Limited to basic operations
  - Missing advanced protocol features
- **Message Format**: Standard HID reports (64 bytes)

### WebUSB Transport (PID 0x0002) - Full API
- **Endpoint**: Bulk transfers, endpoint 1 (IN/OUT)
- **Access**: `rusb` with interface claiming
- **Capabilities**:
  - Full device API access
  - Debug interface support (interface 1)
  - Complete protocol implementation
- **Message Format**: hdwallet segmented protocol

## hdwallet Protocol Deep Dive

### Message Structure
```
Outgoing: [0x23, 0x23, msgType(2), length(4), data]
Incoming: [0x3f, 0x23, 0x23, msgType(2), length(4), data]
```

**Key Insight**: The `0x3f` is **added by the USB layer**, not the application layer.

### Segmentation Protocol
```rust
// Message broken into 64-byte segments:
// Segment format: [size_byte(1), data(63)]
// size_byte = always 63 (even for partial segments)

const SEGMENT_SIZE: usize = 63; // Data per segment
const CHUNK_SIZE: usize = 64;   // Total chunk size
```

### WebUSB Implementation (from hdwallet)
```typescript
// WebUSB endpoint configuration
endpoint_out = 1  // Bulk OUT
endpoint_in = 1   // Bulk IN (not 0x81!)

// Interface claiming
await device.claimInterface(0)  // Main interface
await device.claimInterface(1)  // Debug interface (optional)

// Transfer operations
await device.transferOut(1, data)  // Write to endpoint 1
await device.transferIn(1, 64)     // Read from endpoint 1
```

## Our Implementation Issues

### 1. Endpoint Configuration Problems
```rust
// ❌ WRONG: We tried 0x81 (standard IN endpoint)
let endpoint_in = 0x81;

// ✅ CORRECT: hdwallet uses endpoint 1 for both directions
let endpoint_in = 1;
let endpoint_out = 1;
```

### 2. Segmentation Issues
```rust
// ❌ WRONG: We treated segments inconsistently
// ❌ WRONG: We didn't handle multi-segment reconstruction properly

// ✅ CORRECT: hdwallet approach
segment[0] = 63;  // Always 63, regardless of actual data length
// Copy data and pad with zeros if needed
```

### 3. Timeout Problems
Our implementation experienced "Operation timed out" errors because:
- Wrong endpoint configuration
- Incorrect message segmentation
- Improper interface setup

### 4. USB Layer vs Application Layer Confusion
```rust
// ❌ WRONG: We added 0x3f in application layer
message.push(0x3f);  // USB layer handles this!

// ✅ CORRECT: Application sends pure hdwallet format
message.push(0x23);  // hdwallet magic
message.push(0x23);
```

## Working Implementation Reference

### From hdwallet-keepkey-nodewebusb
```typescript
class TransportDelegate {
  async writeChunk(buf: Uint8Array, debugLink?: boolean): Promise<void> {
    const result = await this.usbDevice.transferOut(
      debugLink ? 2 : 1,  // endpoint 1 for normal, 2 for debug
      buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength)
    );
  }

  async readChunk(debugLink?: boolean): Promise<Uint8Array> {
    const result = await this.usbDevice.transferIn(
      debugLink ? 2 : 1,  // endpoint 1 for normal, 2 for debug
      keepkey.SEGMENT_SIZE + 1  // 64 bytes
    );
  }
}
```

### From hdwallet-keepkey main transport
```typescript
// Message preparation
const header = Buffer.concat([
  Buffer.from("##", "ascii"),      // 0x23, 0x23
  Buffer.allocUnsafe(2),           // msg type
  Buffer.allocUnsafe(4),           // length
]);

// Segmentation
private breakMessageIntoChunks(message: Buffer): Buffer[] {
  const chunks: Buffer[] = [];
  
  for (let i = 0; i < message.length; i += SEGMENT_SIZE) {
    const chunk = Buffer.alloc(SEGMENT_SIZE + 1);
    chunk[0] = SEGMENT_SIZE; // Always 63
    message.copy(chunk, 1, i, i + SEGMENT_SIZE);
    chunks.push(chunk);
  }
  
  return chunks;
}
```

## Rust Implementation Challenges

### 1. Interface Claiming Differences
```rust
// rusb interface claiming is more strict than browser WebUSB
handle.claim_interface(0)?;  // May fail if kernel driver attached
handle.set_auto_detach_kernel_driver(true)?;  // Required on Linux
```

### 2. Endpoint Direction Handling
```rust
// Browser WebUSB: transferOut(1, data) / transferIn(1, size)
// rusb: write_bulk(1, data) / read_bulk(1, buffer)
// Both use endpoint 1, but rusb requires explicit direction handling
```

### 3. Error Handling Differences
```rust
// rusb errors are more detailed but require different handling
match handle.write_bulk(1, data, timeout) {
    Err(rusb::Error::Timeout) => // Handle timeout
    Err(rusb::Error::Access) => // Handle permission
    Err(rusb::Error::NotFound) => // Handle device disconnect
}
```

## Kernel Driver Issues

### macOS Specifics
```bash
# KeepKey devices may have kernel drivers attached
# Must detach before claiming interface
handle.set_auto_detach_kernel_driver(true)
```

### Linux Specifics
```bash
# May require udev rules for permission
# /etc/udev/rules.d/51-keepkey.rules
SUBSYSTEM=="usb", ATTR{idVendor}=="2b24", MODE="0666", GROUP="plugdev"
```

## Message Flow Analysis

### Working HID Flow (Limited)
```
App → hidapi → Kernel HID → Device
     [64-byte reports]    [Basic protocol]
```

### Required WebUSB Flow (Full)
```
App → rusb → libusb → USB bulk → Device
     [Segmented hdwallet]     [Full protocol]
```

## Recommendations for Future Implementation

### 1. Use Node.js WebUSB Bridge
```typescript
// Frontend: Direct browser WebUSB
// Backend: Receive parsed data via IPC
// Advantage: Leverages working hdwallet implementation
```

### 2. Fix Rust WebUSB Implementation
```rust
// Address the specific issues we identified:
// - Correct endpoint configuration (endpoint 1)
// - Proper segmentation (63-byte data + size prefix)
// - USB layer vs application layer separation
// - Interface claiming with auto-detach
```

### 3. Hybrid Approach
```rust
// 1. Attempt WebUSB (full functionality)
// 2. Fallback to HID (basic operations only)
// 3. UI indicates capability level to user
```

## Test Cases That Should Work

### 1. Device Detection
```rust
// Should find device at PID 0x0002 (WebUSB)
// Should be able to claim interface 0
// Should be able to open device handle
```

### 2. Initialize Message
```rust
// Message: [0x23, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
// Segmented: [63, 0x23, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00...pad]
// Should get Features response back
```

### 3. Features Response
```rust
// Should receive: [0x3f, 0x23, 0x23, 0x11, 0x00, length..., protobuf_data]
// Should be able to parse firmware version, bootloader hash, etc.
```

## Error Patterns We Observed

### 1. "Operation timed out" on write_bulk
- **Cause**: Wrong endpoint or interface not properly claimed
- **Solution**: Use endpoint 1, ensure interface 0 claimed

### 2. "Invalid parameter" on read_bulk  
- **Cause**: Device not ready or wrong endpoint direction
- **Solution**: Proper endpoint configuration and device state management

### 3. "Device already claimed"
- **Cause**: Another application has the device open
- **Solution**: Proper device cleanup and exclusive access

## Conclusion

WebUSB is **required** for full KeepKey functionality, but implementing it correctly in Rust requires:

1. **Exact endpoint configuration** (endpoint 1 for both directions)
2. **Proper hdwallet segmentation** (63-byte data + size prefix)
3. **Correct USB layer handling** (0x3f added by USB, not app)
4. **Interface management** (claim interface 0, optional interface 1)
5. **Kernel driver handling** (auto-detach for cross-platform support)

The production KeepKey Desktop relies on browser WebUSB API which handles many low-level details automatically. Implementing equivalent functionality in Rust requires careful attention to these USB protocol specifics.

## Files Referenced

- `projects/hdwallet/packages/hdwallet-keepkey-nodewebusb/src/transport.ts`
- `projects/hdwallet/packages/hdwallet-keepkey/src/transport.ts`
- `projects/keepkey-desktop/packages/keepkey-desktop/src/helpers/kk-state-controller/walletUtils.ts`
- Our implementation: `projects/keepkey-desktop-v4/src-tauri/src/lib.rs` 