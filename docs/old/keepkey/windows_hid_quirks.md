### Windows HID Device Detection Quirks & Solutions for KeepKey

1.  **Inconsistent Serial Numbers on Reconnect**:
    *   **Problem**: Windows may not consistently provide a device's serial number immediately after it's reconnected. Sometimes the serial number might be temporarily unavailable, or it might even be reported as an empty string (`""`).
    *   **Impact**: This can lead to failures in uniquely identifying the device, causing issues with feature fetching, state management, and UI updates (e.g., onboarding dialogs not closing).
    *   **Solution in `usb_manager.rs`**:
        *   Prioritize using a non-empty serial number if available.
        *   If the serial number is `None` or `Some("")` (empty string):
            *   For KeepKey devices: Fall back to a more stable unique ID combining Vendor ID (VID), Product ID (PID), bus number, and device address (e.g., `keepkey_VID_PID_busX_addrY`). This ensures a consistent, non-empty identifier.
            *   For other devices: Fall back to a bus/address identifier.
        *   Log when fallback IDs are generated due to missing or empty serials to aid diagnostics.

2.  **HID Transport Robustness (`transport/hid.rs`)**:
    *   **Problem**: If the exact serial number isn't found (due to the above inconsistencies), the HID transport layer might fail to connect to the KeepKey.
    *   **Solution**:
        *   When `HidTransport::new_for_device` is called:
            *   First, attempt to find and open the KeepKey device using the exact serial number provided.
            *   If an exact match fails (or if no serial number was provided):
                *   Enumerate all connected HID devices.
                *   Identify all KeepKey devices by their VID and PID.
                *   Attempt to open the *first available* KeepKey device from this list.
            *   Log all available HID devices and the connection attempts (both exact match and fallback) for easier debugging.

3.  **General Best Practices**:
    *   **Logging**: Implement detailed logging throughout the device detection and connection process (both in `usb_manager.rs` and `transport/hid.rs`). This is crucial for diagnosing subtle timing issues or platform-specific behaviors on Windows.
    *   **Unique ID Stability**: Ensure that the `unique_id` (or `system_id`) generated for each device is as stable as possible across disconnections and reconnections. While bus number and address can change if the device is plugged into a different USB port, they provide a reasonable fallback when serial numbers are unreliable.
