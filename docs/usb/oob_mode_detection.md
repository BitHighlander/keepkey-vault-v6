# Detecting the Difference Between OOB Wallet Mode and OOB Bootloader Mode on KeepKey

## Overview

When a KeepKey device is first connected out-of-box (OOB), it can appear in either wallet (firmware) mode or bootloader mode. Both modes may use the same USB PID (0x0001), making it non-trivial to distinguish between them using only USB descriptors. Accurate detection is critical for providing the correct user experience and guiding users through setup or recovery flows.

This document summarizes the methods and heuristics implemented to reliably detect the difference between OOB Wallet Mode and OOB Bootloader Mode.

---

## Detection Steps

### 1. Query Device Features via HID
- Send an `Initialize` message to the device using HID.
- Receive the Features protobuf response from the device.

### 2. Analyze Features Response

#### Key Indicators:
- **`bootloader_mode` flag:**
  - `true` → Device is in bootloader mode
  - `false` → Device is in firmware (wallet) mode
- **`firmware_version` string:**
  - "Legacy Bootloader" or similar → Bootloader mode
  - Any other version (e.g., "6.4.2") → Wallet mode
- **`initialized` flag:**
  - `false` is expected in bootloader mode
  - `true` is expected in wallet mode (but may be `false` for new/uninitialized wallets)
- **Features response length:**
  - Bootloader responses are typically short (e.g., 21 bytes)
  - Wallet mode responses are longer (usually > 50 bytes)
- **Data pattern heuristics:**
  - Specific byte values (e.g., byte 16 set to 1) can indicate bootloader mode

### 3. Decision Logic (Heuristics)

The detection logic combines the above indicators for robust mode identification:

- **If** `bootloader_mode` is `true`, **then** device is in Legacy Bootloader mode.
- **Else if** `firmware_version` is "Legacy Bootloader", **then** device is in Legacy Bootloader mode.
- **Else if** `initialized` is `false` **and** response is short, **then** device is likely in Bootloader mode.
- **Else** device is in OOB Wallet (firmware) mode.

### 4. Logging and Debugging

- Detailed debug logs are emitted for:
  - Raw Features response bytes
  - All parsed fields (`bootloader_mode`, `firmware_version`, `initialized`, etc.)
  - The result of each heuristic check
- This allows for easy troubleshooting and future refinement of the detection logic.

---

## Example Debug Output

```
DEBUG: Features data length: 21
DEBUG: First 20 bytes of features: [10, 11, 107, 101, 101, 112, 107, 101, 121, 46, 99, 111, 109, 16, 1, 24, 0, 32, 3, 40]
DEBUG: Short response (21), likely bootloader
Feature flags:
  bootloader_mode: true
  firmware_version: Some("Legacy Bootloader")
  initialized: false
  device_id: Some("9323130311747323E300F100")
  vendor: Some("KeepKey")
Device identified as Legacy Bootloader based on bootloader_mode flag
```

---

## Summary Table

| Indicator                | OOB Wallet Mode        | OOB Bootloader Mode        |
|-------------------------|------------------------|---------------------------|
| `bootloader_mode`       | false                  | true                      |
| `firmware_version`      | e.g., "6.4.2"          | "Legacy Bootloader"       |
| `initialized`           | true (or false for new)| false                     |
| Features length         | > 50 bytes             | ~21 bytes                 |
| Data pattern            | N/A                    | Byte 16 == 1              |

---

## Future Improvements

- Implement full protobuf parsing for more robust detection
- Add automated tests for all known OOB device states
- Continue to update heuristics as new firmware/bootloader versions are released

---

## References
- [KeepKey Device Controller Implementation](./device_controller_implementation.md)
- [KeepKey Protocol Documentation](https://github.com/keepkey/device-protocol)

---

*Last updated: 2025-05-29*
