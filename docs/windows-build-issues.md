# Windows Build Issues & Solutions

This document serves as the master reference for Windows-specific build and runtime issues with the KeepKey Bitcoin-Only Vault application.

## Overview

Windows presents unique challenges for hardware wallet applications due to its driver architecture, permission models, and device enumeration quirks. This document catalogs all known Windows issues and their solutions.

## 🚨 Critical Issues (Fixed in this PR)

### 1. Windows FIDO/U2F Blocklist Issue

**Problem**: Windows 10 (1903+) CTAP-HID filter automatically claims KeepKey devices, blocking USB access.

**Symptoms**:
- "Feature object false" errors in logs
- "Access denied" errors when trying to communicate with device
- Device appears disconnected when physically connected

**Root Cause**: Early KeepKey bootloaders advertised FIDO usage page (0xF1D0) for Chrome U2F compatibility, causing Windows to treat them as security keys.

**Solution**: Force HID transport on Windows to bypass FIDO filter entirely.

**Documentation**: [`docs/keepkey/windows_fido_blocklist_fix.md`](keepkey/windows_fido_blocklist_fix.md)

### 2. Windows HID Parameter Error (0x00000057)

**Problem**: Windows HID API rejects packets with incorrect format, causing ERROR_INVALID_PARAMETER.

**Symptoms**:
- HID write fails with error code 0x00000057
- "The parameter is incorrect" errors
- Device detection works but communication fails

**Root Cause**: Windows HID API expects specific packet format without report ID prefix.

**Solution**: Platform-specific packet formatting for Windows vs other platforms.

**Documentation**: [`docs/keepkey/windows_fido_blocklist_fix.md`](keepkey/windows_fido_blocklist_fix.md) (combined fix)

## 🔧 Additional Windows Issues

### 3. Serial Number Inconsistency

**Problem**: Windows may not consistently provide device serial numbers after reconnection.

**Symptoms**:
- Empty serial numbers (`""`) reported by Windows
- Device identification failures
- UI state inconsistencies

**Solution**: Fallback ID generation using VID/PID/bus/address when serial unavailable.

**Documentation**: [`docs/keepkey/windows_hid_quirks.md`](keepkey/windows_hid_quirks.md)

### 4. HID Device Detection Quirks

**Problem**: Windows HID enumeration can be inconsistent, especially after device reconnection.

**Symptoms**:
- Devices not found immediately after reconnection
- Intermittent detection failures
- Need to unplug/replug device

**Solution**: Robust device enumeration with fallback strategies and retry logic.

**Documentation**: [`docs/keepkey/windows_hid_quirks.md`](keepkey/windows_hid_quirks.md)

## 🎯 Build System Considerations

### Windows Development Dependencies

The Windows build requires specific dependencies that may not be available by default:

```bash
# Windows development typically requires:
# - Windows SDK
# - Visual Studio Build Tools
# - Rust (with MSVC toolchain)
# - Node.js/Bun for frontend

# Note: 'make' is not available on Windows by default
# Use direct commands instead:
cd projects/vault-v2
bun run tauri dev
```

### Windows Makefile Alternative

Since Windows doesn't have `make` by default, we provide PowerShell equivalents:

| Makefile Command | Windows PowerShell Equivalent |
|------------------|-------------------------------|
| `make vault` | `cd projects/vault-v2; bun install; bun run tauri dev` |
| `make vault-build` | `cd projects/vault-v2; bun install; bun run tauri build` |
| `make clean` | Manual deletion of `target/`, `node_modules/`, `dist/` directories |

### Windows-Specific Tauri Configuration

The application includes Windows-specific configuration in `tauri.conf.json`:

```json
{
  "bundle": {
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": "",
      "tsp": false,
      "wix": {
        "language": "en-US"
      },
      "nsis": {
        "displayLanguageSelector": false,
        "installMode": "currentUser"
      }
    }
  }
}
```

## 🧪 Testing on Windows

### Verification Steps

1. **Device Detection**:
   ```powershell
   # Check device manager for KeepKey
   Get-PnpDevice -FriendlyName '*KeepKey*'
   ```

2. **HID Enumeration**:
   ```powershell
   # List HID devices
   Get-WmiObject Win32_PointingDevice
   ```

3. **Application Logs**:
   - Look for "🪟 Windows detected" messages
   - Verify no "Access denied" errors
   - Confirm successful device feature retrieval

### Windows Testing Matrix

| Windows Version | Status | Notes |
|----------------|--------|-------|
| Windows 10 1903+ | ✅ Supported | FIDO filter present, fix required |
| Windows 11 | ✅ Supported | Same FIDO filter behavior |
| Windows 10 <1903 | ⚠️ Limited testing | FIDO filter not present |

## 🚀 Out-of-Box Experience

### Before Fixes

1. User connects KeepKey to Windows
2. Windows FIDO filter claims device
3. Application shows "feature object false" errors
4. User needs admin rights or driver installation
5. Complex troubleshooting required

### After Fixes

1. User connects KeepKey to Windows
2. Application automatically uses HID transport
3. Device detected and communicates successfully
4. No additional drivers or admin rights required
5. Works immediately out-of-box

## 📚 Related Documentation

### Windows-Specific Documents

- [`docs/keepkey/windows_fido_blocklist_fix.md`](keepkey/windows_fido_blocklist_fix.md) - Complete FIDO/HID fix
- [`docs/keepkey/windows_hid_quirks.md`](keepkey/windows_hid_quirks.md) - HID detection quirks
- [`docs/usb/usb_permission_handling.md`](usb/usb_permission_handling.md) - Permission handling strategies

### Cross-Platform Documents

- [`docs/usb/docs/usb-overview.md`](usb/docs/usb-overview.md) - USB/HID overview for all platforms
- [`docs/usb/docs/USB-expanded.md`](usb/docs/USB-expanded.md) - Detailed USB troubleshooting
- [`docs/linux-build-targets.md`](linux-build-targets.md) - Linux build considerations

### Protocol Documentation

- [`docs/keepkey/transport_history.md`](keepkey/transport_history.md) - Transport protocol evolution
- [`docs/usb/docs/keepkey-transport-lessons-learned.md`](usb/docs/keepkey-transport-lessons-learned.md) - Transport implementation lessons

## 🔄 Troubleshooting Guide

### Common Windows Issues

| Error | Likely Cause | Solution |
|-------|--------------|----------|
| "Feature object false" | FIDO filter blocking USB | Upgrade to version with FIDO fix |
| "Access denied" | Permission/driver issue | Use HID transport (automatic in latest version) |
| "Invalid parameter" (0x57) | HID packet format issue | Upgrade to version with HID packet fix |
| Device not detected | Serial number quirk | Application handles this automatically |

### Emergency Fallbacks

If the automatic fixes don't work:

1. **Restart Device**: Unplug and reconnect KeepKey
2. **Different USB Port**: Try another USB port
3. **Restart Application**: Close and reopen the vault
4. **System Restart**: Restart Windows to clear stuck drivers
5. **Manual Driver**: Last resort - install WinUSB driver with Zadig

## 🎯 Future Windows Enhancements

### Planned Improvements

1. **Advanced Diagnostics**: Windows-specific diagnostic tools
2. **Performance Optimization**: Platform-specific optimizations
3. **Enhanced Logging**: Better Windows error reporting
4. **Driver Validation**: Automatic driver configuration validation

### Investigation Areas

1. **Windows Store Distribution**: UWP packaging considerations
2. **ARM64 Support**: Windows on ARM compatibility
3. **WebUSB Integration**: Browser transport for Windows
4. **Hardware Security Module**: Windows hardware security integration

## 📞 Support and Resources

### Getting Help

For Windows-specific issues:

1. Check this documentation first
2. Review application logs for Windows-specific messages
3. Try the troubleshooting steps above
4. Open GitHub issue with Windows version and error details

### Development Resources

- [Windows Hardware Dev Center](https://docs.microsoft.com/en-us/windows-hardware/)
- [Windows HID API Documentation](https://docs.microsoft.com/en-us/windows-hardware/drivers/hid/)
- [Windows USB Driver Development](https://docs.microsoft.com/en-us/windows-hardware/drivers/usbcon/)
- [Tauri Windows Configuration](https://tauri.app/v1/guides/building/windows)

---

*Last updated: 2024-01-XX*  
*Covers: Windows 10 1903+, Windows 11*  
*Status: Issues resolved in current release* 