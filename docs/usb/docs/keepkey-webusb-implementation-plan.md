# KeepKey WebUSB Implementation Plan for Tauri/Rust

## Quick Implementation Strategy

Based on the analysis, we have **two parallel approaches** to implement WebUSB support:

### Approach 1: Frontend WebUSB → Rust Backend Bridge
**Fastest to implement, highest compatibility**

```typescript
// Frontend: Direct WebUSB Access
const connectKeepKeyWebUSB = async () => {
  if (!navigator.usb) {
    throw new Error('WebUSB not supported in this browser');
  }
  
  try {
    // Request WebUSB device access
    const device = await navigator.usb.requestDevice({
      filters: [
        { vendorId: 0x2b24, productId: 0x0002 }, // WebUSB KeepKey
        { vendorId: 0x2b24, productId: 0x0001 }  // HID KeepKey (limited)
      ]
    });
    
    await device.open();
    await device.selectConfiguration(1);
    await device.claimInterface(0);
    
    // Bridge to Rust backend for protocol handling
    return await window.tauri.invoke('handle_webusb_device', {
      deviceInfo: {
        vendorId: device.vendorId,
        productId: device.productId,
        serialNumber: device.serialNumber
      }
    });
  } catch (error) {
    // Fallback to HID via Rust backend
    return await window.tauri.invoke('get_features_hid_fallback');
  }
};
```

### Approach 2: Pure Rust WebUSB Transport
**Better integration, requires more research**

```rust
// Add to Cargo.toml
[dependencies]
rusb = { version = "0.9", features = ["vendored"] }
tokio = { version = "1", features = ["full"] }
futures = "0.3"

// Rust WebUSB implementation
use rusb::{Device, DeviceHandle, GlobalContext};

pub struct WebUSBTransport {
    handle: DeviceHandle<GlobalContext>,
    endpoint_in: u8,
    endpoint_out: u8,
    debug_interface: Option<u8>,
}

impl WebUSBTransport {
    pub async fn connect(vid: u16, pid: u16) -> Result<Self, TransportError> {
        let devices = rusb::devices()?;
        
        for device in devices.iter() {
            let desc = device.device_descriptor()?;
            
            if desc.vendor_id() == vid && desc.product_id() == pid {
                let mut handle = device.open()?;
                
                // For WebUSB devices (PID 0x0002), try to claim interface 0
                match handle.claim_interface(0) {
                    Ok(_) => {
                        println!("✅ WebUSB interface 0 claimed");
                        
                        // Try to claim debug interface 1 (optional)
                        let debug_interface = match handle.claim_interface(1) {
                            Ok(_) => {
                                println!("✅ Debug interface 1 claimed");
                                Some(1)
                            }
                            Err(_) => {
                                println!("⚠️ Debug interface not available");
                                None
                            }
                        };
                        
                        return Ok(WebUSBTransport {
                            handle,
                            endpoint_in: 0x81,   // IN endpoint
                            endpoint_out: 0x01,  // OUT endpoint
                            debug_interface,
                        });
                    }
                    Err(rusb::Error::Access) => {
                        return Err(TransportError::DeviceAlreadyClaimed);
                    }
                    Err(e) => {
                        return Err(TransportError::USBError(e));
                    }
                }
            }
        }
        
        Err(TransportError::DeviceNotFound)
    }
}
```

## Immediate Implementation Steps

### Step 1: Modify Current Transport Layer (30 minutes)

```rust
// Update lib.rs
fn attempt_direct_connection() -> Result<Option<FeatureInfo>, String> {
    // FIRST: Try WebUSB (PID 0x0002)
    match attempt_webusb_connection() {
        Ok(Some(features)) => {
            println!("✅ WebUSB connection successful");
            return Ok(Some(features));
        }
        Err(e) => {
            println!("⚠️ WebUSB failed: {}, trying HID", e);
        }
        Ok(None) => {
            println!("⚠️ No WebUSB device found, trying HID");
        }
    }
    
    // SECOND: Fallback to HID (PID 0x0001)
    attempt_hidapi_connection()
}

fn attempt_webusb_connection() -> Result<Option<FeatureInfo>, String> {
    // Try to find WebUSB device (PID 0x0002)
    let context = rusb::Context::new().map_err(|e| e.to_string())?;
    let devices = context.devices().map_err(|e| e.to_string())?;
    
    for device in devices.iter() {
        let desc = device.device_descriptor().map_err(|e| e.to_string())?;
        
        if desc.vendor_id() == KEEPKEY_VID && desc.product_id() == 0x0002 {
            println!("Found WebUSB KeepKey device (PID 0x0002)");
            
            match device.open() {
                Ok(mut handle) => {
                    // Try to claim interface 0 (main communication)
                    match handle.claim_interface(0) {
                        Ok(_) => {
                            println!("✅ WebUSB interface claimed successfully");
                            
                            // TODO: Implement WebUSB communication protocol
                            // For now, return basic info to prove connection
                            return Ok(Some(FeatureInfo {
                                vendor: Some("KeepKey".to_string()),
                                device_id: Some("WebUSB".to_string()),
                                firmware_version: Some("WebUSB-Connected".to_string()),
                                bootloader_mode: false,
                                bootloader_hash: None,
                                bootloader_version: None,
                                revision: None,
                                initialized: true,
                                pin_protection: false,
                                passphrase_protection: false,
                                policies: vec![],
                            }));
                        }
                        Err(rusb::Error::Access) => {
                            return Err("WebUSB device already claimed by another application".to_string());
                        }
                        Err(e) => {
                            return Err(format!("Failed to claim WebUSB interface: {}", e));
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to open WebUSB device: {}", e);
                    continue;
                }
            }
        }
    }
    
    Ok(None) // No WebUSB device found
}
```

### Step 2: Update Constants (5 minutes)

```rust
// Update constants in lib.rs
const KEEPKEY_VID: u16 = 0x2B24;
const KEEPKEY_PID_HID: u16 = 0x0001;     // HID mode (fallback)
const KEEPKEY_PID_WEBUSB: u16 = 0x0002;  // WebUSB mode (primary)
```

### Step 3: Add Frontend WebUSB Support (15 minutes)

```html
<!-- Add to your HTML -->
<script>
async function connectKeepKeyAdvanced() {
  // Method 1: Try WebUSB directly from frontend
  if (navigator.usb) {
    try {
      const device = await navigator.usb.requestDevice({
        filters: [{ vendorId: 0x2b24 }]
      });
      
      console.log('WebUSB device selected:', device);
      
      // Send device info to Rust backend
      const result = await window.tauri.invoke('handle_webusb_device', {
        productId: device.productId,
        serialNumber: device.serialNumber
      });
      
      console.log('Backend result:', result);
      return result;
    } catch (e) {
      console.log('WebUSB failed, falling back to HID:', e);
    }
  }
  
  // Method 2: Fallback to current HID implementation
  return await window.tauri.invoke('get_features');
}
</script>
```

### Step 4: Test the Changes (10 minutes)

```bash
cd projects/keepkey-desktop-v4
cargo build
bun run tauri dev
```

## Expected Results

After implementing Step 1, you should see:

**With WebUSB device (PID 0x0002):**
```
Found WebUSB KeepKey device (PID 0x0002)
✅ WebUSB interface claimed successfully
✅ WebUSB connection successful
```

**With HID device (PID 0x0001):**
```
⚠️ No WebUSB device found, trying HID
Successfully opened KeepKey device with hidapi
```

**With device already claimed:**
```
Found WebUSB KeepKey device (PID 0x0002)
⚠️ WebUSB failed: WebUSB device already claimed by another application, trying HID
```

## Full Protocol Implementation (Phase 2)

Once basic detection works, implement the full WebUSB protocol:

```rust
impl WebUSBTransport {
    async fn send_message(&mut self, msg_type: u16, data: &[u8]) -> Result<Vec<u8>, TransportError> {
        // Implement WebUSB bulk transfer protocol
        // Similar to HID but using transferOut/transferIn instead of interrupt
        
        let message = self.prepare_webusb_message(msg_type, data)?;
        
        // Send via bulk endpoint
        let written = self.handle.write_bulk(self.endpoint_out, &message, Duration::from_secs(5))?;
        
        // Read response via bulk endpoint
        let mut response = vec![0u8; 1024];
        let read = self.handle.read_bulk(self.endpoint_in, &mut response, Duration::from_secs(5))?;
        
        response.truncate(read);
        self.parse_webusb_response(&response)
    }
}
```

## Testing Strategy

1. **Device Detection**: Verify both PID 0x0001 and 0x0002 are detected
2. **Transport Priority**: Confirm WebUSB is tried first
3. **Fallback Logic**: Ensure HID fallback works when WebUSB fails
4. **Error Handling**: Test device already claimed scenarios
5. **Frontend Integration**: Test WebUSB from both frontend and backend

This approach gives us **immediate improvement** with **minimal risk** to existing functionality. 