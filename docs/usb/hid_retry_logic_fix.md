# HID Retry Logic Fix for macOS "Device Already In Use" Issue

## Problem Description

On macOS, KeepKey devices connected via HID would sometimes fail to open with the error:

```
🔒 KeepKey Device Already In Use

The KeepKey device (serial: 932313031174732313008100) is currently being used by another application.

Technical details: hidapi error: hid_open_path: failed to open IOHIDDevice from mach entry: (0xE00002C5) (iokit/common) exclusive access and device already open
```

This error occurred even when no other applications were using the device. The issue is caused by macOS HID subsystem temporarily claiming devices, which can result in a "false positive" exclusive access error.

## Root Cause

The problem stems from macOS's HID (Human Interface Device) handling:

1. **Temporary Device Claims**: macOS can temporarily claim HID devices for various system operations
2. **IOKit Exclusive Access**: The IOKit framework enforces exclusive access to HID devices
3. **Race Conditions**: When multiple applications try to access HID devices simultaneously, the system can get into a state where devices appear "claimed" even when they're not
4. **No Retry Logic**: The original implementation immediately failed on the first access attempt

## Solution: Exponential Backoff Retry Logic

The fix implements retry logic with exponential backoff in all HID transport implementations:

### Key Features

1. **Retry Attempts**: Up to 5 attempts to open each device
2. **Exponential Backoff**: Delays between attempts: 100ms, 200ms, 400ms, 800ms, 1600ms
3. **Smart Error Detection**: Only retries for access-related errors
4. **Detailed Logging**: Comprehensive logging for debugging
5. **Improved Error Messages**: Better user guidance when all attempts fail

### Implementation Details

The retry logic is implemented in the `try_open_device_with_retry` function:

```rust
fn try_open_device_with_retry(device_info: &hidapi::DeviceInfo, api: &HidApi, max_attempts: u32) -> Result<HidDevice> {
    let mut last_error = None;
    
    for attempt in 1..=max_attempts {
        match device_info.open_device(api) {
            Ok(device) => {
                if attempt > 1 {
                    info!("✅ Successfully opened device after {} attempts", attempt);
                }
                return Ok(device);
            }
            Err(e) => {
                let error_msg = e.to_string().to_lowercase();
                let is_access_error = error_msg.contains("access") || 
                                     error_msg.contains("permission") || 
                                     error_msg.contains("in use") || 
                                     error_msg.contains("busy") ||
                                     error_msg.contains("claimed") || 
                                     error_msg.contains("cannot open") ||
                                     error_msg.contains("exclusive access");
                
                if is_access_error && attempt < max_attempts {
                    // Exponential backoff: 100ms, 200ms, 400ms, 800ms, 1600ms
                    let delay_ms = 100 * (1u64 << (attempt - 1).min(4));
                    warn!("⚠️ Device access attempt {} failed (will retry in {}ms): {}", attempt, delay_ms, e);
                    
                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                    last_error = Some(e);
                    continue;
                } else {
                    last_error = Some(e);
                    break;
                }
            }
        }
    }
    
    // Handle final failure with detailed error message
    // ... (error handling code)
}
```

## Files Modified

The retry logic has been implemented in all HID transport files:

1. **`projects/keepkey-rust/transport/hid.rs`** - Core keepkey-rust implementation
2. **`projects/vault/src-tauri/src/transport/hid.rs`** - Vault v1 implementation
3. **`projects/kkcli/src/transport/hid.rs`** - CLI tool implementation

## Error Detection

The retry logic specifically handles these error patterns:

- `"access"` - General access denied errors
- `"permission"` - Permission denied errors
- `"in use"` - Device in use errors
- `"busy"` - Device busy errors
- `"claimed"` - Device claimed errors
- `"cannot open"` - Generic open failures
- `"exclusive access"` - IOKit exclusive access errors

## Benefits

1. **Improved Reliability**: Handles temporary macOS HID claiming issues
2. **Better User Experience**: Reduces frustrating "device in use" errors
3. **Automatic Recovery**: No user intervention required for temporary issues
4. **Detailed Diagnostics**: Better error messages with troubleshooting steps
5. **Cross-Platform Consistency**: Same retry logic across all platforms

## Testing

A test script is provided to verify the fix:

```bash
./skills/test-hid-retry-logic.sh
```

This script:
- Tests HID device detection with retry logic
- Provides system HID device information
- Offers troubleshooting guidance
- Validates the fix is working correctly

## Example Usage

With the fix, when a temporary access issue occurs, you'll see:

```
⚠️ Device access attempt 1 failed (will retry in 100ms): hidapi error: hid_open_path: failed to open IOHIDDevice from mach entry: (0xE00002C5) (iokit/common) exclusive access and device already open
⚠️ Device access attempt 2 failed (will retry in 200ms): hidapi error: hid_open_path: failed to open IOHIDDevice from mach entry: (0xE00002C5) (iokit/common) exclusive access and device already open
✅ Successfully opened device after 3 attempts
```

## Fallback Behavior

If all retry attempts fail, the user receives a comprehensive error message:

```
🔒 KeepKey Device Access Failed After 5 Attempts

The KeepKey device (serial: 932313031174732313008100) could not be opened after 5 retry attempts.
This typically indicates the device is being used by another application.

Common causes:
• KeepKey Desktop app is running
• KeepKey Bridge is running
• Another wallet application is connected
• Previous connection wasn't properly closed
• macOS system temporarily has the device claimed

Solutions:
1. Close KeepKey Desktop app completely
2. Close any other wallet applications (MetaMask, etc.)
3. Unplug and reconnect your KeepKey device
4. Try a different USB port
5. Restart your computer if the issue persists

Technical details: hidapi error (after 5 attempts)
```

## Performance Impact

The retry logic has minimal performance impact:

- **Success Case**: No additional delay (immediate success)
- **Temporary Failure**: Small delays (100ms-1600ms total) that resolve the issue
- **Permanent Failure**: Maximum 3.1 seconds delay before final error (acceptable UX)

## Platform Compatibility

The fix is safe and beneficial across all platforms:

- **macOS**: Addresses the primary "exclusive access" issue
- **Linux**: Helps with occasional permission/access issues
- **Windows**: Provides consistent error handling

## Future Improvements

Potential enhancements:

1. **Adaptive Retry Count**: Adjust retry attempts based on platform
2. **Device-Specific Tuning**: Different retry parameters for different device types
3. **Metrics Collection**: Track retry success rates for optimization
4. **Background Retry**: Non-blocking retry attempts for better UX

## Troubleshooting

If you continue to see device access issues after this fix:

1. **Check Running Applications**: Ensure no other wallet apps are running
2. **System Restart**: Restart your computer to clear any stuck HID state
3. **USB Port Change**: Try a different USB port
4. **Cable Replacement**: Try a different USB cable
5. **Contact Support**: If issues persist, contact support with detailed logs

## Technical Notes

- The retry logic uses exponential backoff to avoid overwhelming the system
- Only access-related errors trigger retries (other errors fail immediately)
- Thread sleep is used (acceptable for HID operations which are infrequent)
- Logging provides clear visibility into retry behavior
- Error messages guide users toward effective solutions 