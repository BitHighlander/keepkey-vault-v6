# Windows FIDO/U2F Blocklist Fix for KeepKey

## Problem Description

Windows 10 (build 1903+) includes a CTAP-HID filter driver that automatically claims any USB device with a FIDO usage page (0xF1D0). Early KeepKey bootloaders advertised this page for Chrome U2F compatibility, causing Windows to block direct USB access with "Access denied" errors.

This results in:
- "Feature object false" errors in logs
- `evaluate_device_status` receiving `None` features
- Device appearing disconnected when it's actually connected

## Complete Solution Implemented

We've implemented a **two-part permanent fix** that addresses both the FIDO blocklist and Windows HID communication issues:

### 1. FIDO Blocklist Bypass (`device_queue.rs`)
- **Forces HID transport on Windows** for all KeepKey devices
- **Bypasses the FIDO filter** completely
- **Works for all device PIDs** (0x0001, 0x0002, etc.)
- **Doesn't affect macOS/Linux** behavior

### 2. Windows HID Packet Format Fix (`transport/hid.rs`)
- **Platform-specific HID packet format** for Windows vs other OS
- **Fixes Windows error 0x00000057** (ERROR_INVALID_PARAMETER)
- **Windows**: Send protocol directly without report ID prefix
- **Non-Windows**: Keep original format with report ID for compatibility

## Code Changes

### FIDO Blocklist Fix

In `projects/keepkey-rust/device_queue.rs`:
```rust
// WINDOWS FIDO BLOCKLIST FIX: Always use HID on Windows to avoid FIDO filter driver
#[cfg(target_os = "windows")]
{
    info!("🪟 Windows detected - forcing HID transport to avoid FIDO/U2F blocklist issues");
    info!("   📝 Windows CTAP-HID filter blocks USB access for KeepKey devices");
    info!("   ✅ HID transport bypasses this restriction and works reliably");
    return Ok(TransportType::HidOnly);
}
```

### Windows HID Packet Format Fix

In `projects/keepkey-rust/transport/hid.rs`:
```rust
// Windows HID fix: Different packet format for Windows
#[cfg(target_os = "windows")]
{
    // Windows: Start directly with protocol without report ID
    first_packet[0] = 0x3f;
    first_packet[1] = 0x23;
    first_packet[2] = 0x23;
    first_packet[3..5].copy_from_slice(msg_type);
    first_packet[5..9].copy_from_slice(msg_length);
    
    info!("🪟 Windows HID: Using direct protocol format (no report ID)");
}

// Non-Windows: Use original format with report ID
#[cfg(not(target_os = "windows"))]
{
    first_packet[0] = REPORT_ID;  // 0x00 - Required for other platforms
    first_packet[1] = 0x3f;
    first_packet[2] = 0x23;
    first_packet[3] = 0x23;
    first_packet[4..6].copy_from_slice(msg_type);
    first_packet[6..10].copy_from_slice(msg_length);
}
```

## Why This Works

### FIDO Blocklist Bypass
1. **HID bypasses FIDO filter**: Windows allows HID access without elevated permissions
2. **Same protocol support**: Both USB and HID transports implement the same KeepKey protocol
3. **No driver changes needed**: Uses built-in Windows HID drivers
4. **Automatic fallback**: If HID fails, error is properly reported

### Windows HID Parameter Fix
1. **Correct packet format**: Windows HID expects specific packet structure
2. **No report ID prefix**: Windows HID API handles report ID internally
3. **Maintains compatibility**: Other platforms keep working format unchanged
4. **Resolves 0x00000057 error**: "Invalid parameter" error eliminated

## Testing

To verify the complete fix:

1. **Connect a KeepKey on Windows**
2. **Check logs for FIDO bypass**: "🪟 Windows detected - forcing HID transport"
3. **Check logs for HID format**: "🪟 Windows HID: Using direct protocol format"
4. **Verify device features retrieved**: No "feature object false" errors
5. **Confirm no Access denied errors**: Device communication successful

Example successful logs:
```
🪟 Windows detected - forcing HID transport to avoid FIDO/U2F blocklist issues
🪟 Windows HID: Using direct protocol format (no report ID)
📡 Got device features: Unlabeled v4.0.0 (932313031174732313008100)
```

## Windows Build Issues Resolved

This fix addresses multiple Windows-specific build and runtime issues:

### 1. Out-of-Box (OOB) Experience
- **Before**: "Feature object false" errors on fresh Windows installs
- **After**: Device works immediately without driver installation

### 2. FIDO/U2F Conflicts
- **Before**: Windows CTAP-HID filter blocks USB access
- **After**: HID transport bypasses filter completely

### 3. HID Communication Errors
- **Before**: Windows error 0x00000057 (Invalid parameter) during HID write
- **After**: Correct packet format eliminates parameter errors

### 4. Permission Requirements
- **Before**: Requires admin rights or driver modification (Zadig/WinUSB)
- **After**: Works with standard user permissions using built-in HID drivers

### 5. Driver Dependencies
- **Before**: Needs third-party USB drivers or registry modifications
- **After**: Uses Windows built-in HID drivers, no external dependencies

## Cross-Platform Compatibility

### ✅ **macOS Preserved**
- **HID transport**: Uses original packet format with report ID
- **Transport selection**: Maintains existing logic for device detection
- **IOKit compatibility**: No changes to macOS-specific HID behavior
- **Zero impact**: All macOS code paths unchanged

### ✅ **Linux Preserved**  
- **HID transport**: Uses original packet format with report ID
- **Transport selection**: Maintains existing logic with USB preference
- **udev rules**: Continue to work as before for USB access
- **Zero impact**: All Linux code paths unchanged

### ✅ **Windows Fixed**
- **FIDO bypass**: Forces HID to avoid Windows CTAP-HID filter
- **HID format**: Uses Windows-specific packet format
- **Driver-free**: Works with built-in Windows HID drivers
- **Complete solution**: Addresses both detection and communication issues

## Alternative Solutions (Not Recommended)

1. **Zadig/WinUSB**: Requires admin rights and driver replacement
2. **Native messaging helper**: Extra process complexity
3. **Registry edits**: System-wide changes affecting other apps
4. **FIDO exclusion lists**: Would affect legitimate security keys

## Performance Impact

- **Negligible overhead**: Platform detection at compile time
- **Same protocol**: No performance difference in communication
- **Reduced complexity**: Eliminates driver installation requirements
- **Better reliability**: Reduces Windows-specific edge cases

## References

- [Windows CTAP-HID Filter Documentation](https://docs.microsoft.com/en-us/windows-hardware/design/device-experiences/windows-hello)
- [FIDO Alliance CTAP Specification](https://fidoalliance.org/specs/fido-v2.0-ps-20190130/fido-client-to-authenticator-protocol-v2.0-ps-20190130.html)
- [Windows HID API Documentation](https://docs.microsoft.com/en-us/windows-hardware/drivers/hid/)
- [KeepKey USB/HID Transport Documentation](docs/usb/docs/usb-overview.md)
- [Windows HID Quirks Documentation](docs/keepkey/windows_hid_quirks.md)

## Future Considerations

1. **WebUSB Support**: Consider adding WebUSB transport for browser compatibility
2. **Device Capability Detection**: Auto-detect which features require USB vs HID
3. **Performance Optimization**: Cache transport type per device for faster reconnection
4. **Advanced Diagnostics**: Add Windows-specific diagnostic tools for troubleshooting

---

*Implementation completed: 2024-01-XX*  
*Tested on: Windows 10 1903+, Windows 11*  
*Compatible with: All KeepKey device PIDs (0x0001, 0x0002)* 