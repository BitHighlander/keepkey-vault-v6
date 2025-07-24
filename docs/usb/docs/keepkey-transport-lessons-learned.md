# KeepKey Transport: Lessons Learned & Action Items

## TL;DR - What We Learned

1. **KeepKey requires WebUSB for full functionality** - HID is just a limited fallback
2. **Production code prioritizes WebUSB first**, then falls back to HID
3. **Different PIDs = Different capabilities**: 0x0002 (WebUSB full), 0x0001 (HID limited)
4. **Rust WebUSB is complex** - endpoint configuration, segmentation, kernel drivers
5. **Browser WebUSB "just works"** - should leverage existing hdwallet implementation

## Critical Production Finding

**From KeepKey Desktop codebase analysis**:
```typescript
// walletUtils.ts - PRODUCTION PRIORITY ORDER:
const webUsbDevice = await webUsbAdapter.getDevice()  // ✅ FIRST: Full API
const hidDevice = await hidAdapter.getDevice()        // ⚠️  FALLBACK: Limited
```

**Implication**: Any serious KeepKey application **must support WebUSB** for complete functionality.

## Protocol Complexity Reality Check

### What We Expected (Simple)
```rust
// "Just use rusb instead of hidapi, should be similar"
rusb::open(0x2B24, 0x0002).write(message).read(response)
```

### What We Found (Complex)
```rust
// 1. Interface claiming with kernel driver management
handle.set_auto_detach_kernel_driver(true)?;
handle.claim_interface(0)?;

// 2. Specific endpoint configuration (not standard)
let endpoint = 1; // NOT 0x81 like normal USB devices

// 3. hdwallet segmentation protocol
let segments = break_into_63_byte_chunks_with_size_prefix(message);

// 4. Multi-chunk message reconstruction
let response = reconstruct_from_segments_with_usb_layer_magic();

// 5. Cross-platform kernel driver differences
// 6. Permission management (udev rules, admin rights)
```

## Rust vs Browser WebUSB Comparison

| Aspect | Browser WebUSB | Rust rusb |
|--------|---------------|-----------|
| **Setup** | `navigator.usb.requestDevice()` | Interface claiming, kernel drivers |
| **Endpoints** | `transferOut(1, data)` | `write_bulk(1, data, timeout)` |
| **Permissions** | Browser handles | Manual udev rules / admin |
| **Segmentation** | hdwallet lib handles | Manual implementation required |
| **Error Handling** | High-level errors | Low-level USB errors |
| **Development Time** | Hours (existing lib) | Days/weeks (from scratch) |

## Implementation Strategies Ranked

### 1. 🥇 Frontend WebUSB Bridge (Recommended)
```typescript
// Frontend: Use browser WebUSB + existing hdwallet
// Backend: Receive parsed data via Tauri commands
// Pros: Leverages working code, cross-platform, fast
// Cons: Requires frontend complexity
```

### 2. 🥈 Rust WebUSB (Fixed Implementation)
```rust
// Fix our current approach with correct:
// - Endpoint configuration (endpoint 1)
// - hdwallet segmentation
// - Interface management
// Pros: Pure Rust, backend-only
// Cons: Complex, platform-specific issues
```

### 3. 🥉 HID Only (Limited Functionality)
```rust
// Accept reduced functionality
// Clearly communicate limitations to users
// Pros: Simple, works now
// Cons: Missing critical features
```

## Specific Technical Fixes Required

If continuing with Rust WebUSB approach:

### 1. Endpoint Configuration
```rust
// ❌ Current (wrong)
let endpoint_in = 0x81;

// ✅ Required (correct)
let endpoint_in = 1;
let endpoint_out = 1;
```

### 2. Segmentation Fix
```rust
// ✅ Required segmentation
for chunk in message.chunks(63) {
    let mut segment = vec![0u8; 64];
    segment[0] = 63; // Always 63, even for partial chunks
    segment[1..1+chunk.len()].copy_from_slice(chunk);
    // Send segment
}
```

### 3. Message Format Fix
```rust
// ❌ Don't add 0x3f (USB layer adds this)
// ✅ Send pure hdwallet format
let message = [0x23, 0x23, msg_type_bytes, length_bytes, data];
```

## Error Patterns & Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| "Operation timed out" | Wrong endpoint | Use endpoint 1, not 0x81 |
| "Invalid parameter" | Interface not claimed | Claim interface 0 first |
| "Device already claimed" | Another app open | Close other KeepKey apps |
| "Access denied" | Permissions | Run as admin or setup udev rules |

## Deployment Considerations

### Development
```bash
# macOS: May need admin for USB access
sudo ./keepkey-app

# Linux: Need udev rules
echo 'SUBSYSTEM=="usb", ATTR{idVendor}=="2b24", MODE="0666"' > /etc/udev/rules.d/51-keepkey.rules
```

### Production
- **Frontend WebUSB**: No special permissions (browser handles)
- **Rust WebUSB**: Requires installer to setup permissions
- **HID**: Works out of box (but limited functionality)

## Recommendation for Next Steps

### Immediate (Next Sprint)
1. **Implement Frontend WebUSB Bridge**
   - Use existing hdwallet libraries
   - Add Tauri commands to receive parsed device data
   - Fastest path to full functionality

### Medium Term (Future Sprint)
2. **Fix Rust WebUSB Implementation**
   - Apply specific technical fixes identified
   - Use as backup/alternative to frontend approach
   - Better for security-focused deployments

### Long Term
3. **Hybrid Approach**
   - Frontend WebUSB for full functionality
   - Rust WebUSB for security-critical operations
   - HID as last resort with clear limitations

## Files to Reference

**Working Implementations**:
- `projects/hdwallet/packages/hdwallet-keepkey-nodewebusb/src/transport.ts`
- `projects/hdwallet/packages/hdwallet-keepkey/src/transport.ts`
- `projects/keepkey-desktop/packages/keepkey-desktop/src/helpers/kk-state-controller/walletUtils.ts`

**Our Implementation**:
- `projects/keepkey-desktop-v4/src-tauri/src/lib.rs` (current state)

**Documentation**:
- `docs/keepkey-webusb-requirements.md` (high-level analysis)
- `docs/keepkey-webusb-implementation-plan.md` (implementation options)
- `docs/keepkey-webusb-transport-analysis.md` (technical deep dive)

## Key Takeaway

**WebUSB is not optional for KeepKey** - it's a requirement for full functionality. The path of least resistance is leveraging the existing browser WebUSB implementation rather than reimplementing the complex protocol in Rust from scratch. 