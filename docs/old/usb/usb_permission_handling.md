# USB Permission Handling and HID Fallback

## Overview

When connecting to KeepKey devices, especially legacy bootloader devices, USB permission errors can occur. This document describes the automatic fallback mechanism that uses HID API when USB permissions fail.

## The Problem

Legacy bootloader KeepKey devices and out-of-box (OOB) devices often encounter USB permission errors:
```
Failed to initialize USB transport for device: Access denied (insufficient permissions)
```

This typically happens because:
- The user lacks permissions to access raw USB devices
- udev rules are not properly configured on Linux
- The device is in bootloader mode which may have different permission requirements

## The Solution: Automatic HID Fallback

The KeepKey Desktop application now implements an automatic fallback mechanism:

1. **Primary Attempt**: Try to connect using USB transport (libusb)
2. **Permission Check**: If USB fails with permission errors, detect this condition
3. **Fallback**: Automatically retry using HID API (hidapi)
4. **Success**: HID often has better OS-level permissions and can succeed where USB fails

## Implementation Details

### Transport Layer

Two transport implementations are available:
- `UsbTransport`: Uses libusb for direct USB communication
- `HidTransport`: Uses hidapi for HID-level communication

Both implement the same `Transport` trait, making them interchangeable.

### Features Module

The `get_device_features_with_fallback()` function:
```rust
pub fn get_device_features_with_fallback(target_device: &FriendlyUsbDevice) -> Result<DeviceFeatures>
```

This function:
1. First attempts USB transport via `get_device_features_for_device()`
2. Checks for permission-related errors
3. Falls back to HID transport via `get_device_features_via_hid()` if needed
4. Returns device features regardless of which transport succeeded

### Error Detection

The following error patterns trigger HID fallback:
- "Access denied"
- "insufficient permissions"
- "LIBUSB_ERROR_ACCESS"

## Platform-Specific Notes

### Linux

On Linux, you can avoid permission issues by setting up udev rules:

```bash
# Create udev rule for KeepKey
echo 'SUBSYSTEM=="usb", ATTR{idVendor}=="2b24", MODE="0666", GROUP="plugdev"' | sudo tee /etc/udev/rules.d/51-keepkey.rules
echo 'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="2b24", MODE="0666", GROUP="plugdev"' | sudo tee -a /etc/udev/rules.d/51-keepkey.rules

# Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# Add user to plugdev group
sudo usermod -aG plugdev $USER
```

### macOS

On macOS, HID devices typically work without additional permissions. The HID fallback ensures compatibility even if direct USB access is restricted.

### Windows

Windows generally allows HID access without elevated permissions, making the HID fallback particularly useful.

## Update Commands

When a legacy bootloader device is detected, users should update their firmware. The update process:

1. **Detection**: The application detects bootloader mode via the `bootloader_mode` flag
2. **Notification**: Users are prompted to update firmware
3. **Update Flow**: Guide users through the firmware update process

Example update detection:
```rust
if device_features.bootloader_mode {
    // Device is in bootloader mode
    if device_features.version == "0.0.0" || device_features.version.contains("Legacy") {
        // Legacy bootloader detected - prompt for update
    }
}
```

## Debugging

Enable debug logging to see transport fallback in action:
```
[INFO] Getting features for device with fallback: KeepKey (9323130311747323E300F100)
[WARN] USB permission denied for device 9323130311747323E300F100, trying HID fallback
[INFO] Attempting HID connection for device: 9323130311747323E300F100
[INFO] Successfully got features via HID for device 9323130311747323E300F100
```

## Benefits

1. **Improved User Experience**: Users don't need to configure permissions manually
2. **Broader Compatibility**: Works on systems with restrictive USB permissions
3. **Automatic**: No user intervention required
4. **Transparent**: Same API for both transport methods

## Future Improvements

1. **Smart Transport Selection**: Remember which transport worked for each device
2. **Parallel Attempts**: Try both transports simultaneously for faster connection
3. **Permission Helper**: Provide OS-specific instructions when both transports fail

---

*Last updated: 2024-12-31* 