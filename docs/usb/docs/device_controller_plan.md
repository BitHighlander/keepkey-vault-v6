# KeepKey Device Controller Planning

## Objective

Design and implement a robust Device Controller in the Rust backend that:
- Detects all KeepKey device states (normal wallet mode, legacy bootloader, OOB, etc.) using all available USB libraries.
- Maintains and updates device state in real time.
- Emits state changes to the frontend instantly.

---

## 1. Core Requirements

- **Universal Detection:** Must detect all device types/PIDs (OOB wallet, OOB legacy bootloader, WebUSB, future types).
- **Multi-Library Support:** Use `hidapi` for HID, `rusb` for USB, and optionally `webusb` for browser-based detection.
- **Real-Time Updates:** State changes (plug/unplug, mode switch) must be sent to the frontend with zero polling.
- **Centralized State:** All device state and detection logic is managed in a single controller struct.

---

## 2. Rust Backend Structure

### 2.1. New Module: `device_controller.rs`

**Location:**  
`/projects/keepkey-desktop/src-tauri/src/device_controller.rs`

**Justification:**  
- Keeps device state logic isolated from Tauri commands and protocol definitions.
- Promotes modularity and future extensibility.

**Code Skeleton:**
```rust
// device_controller.rs

use tauri::Manager;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use crate::{FeatureInfo, PolicyInfo};

#[derive(Clone, serde::Serialize, Debug, PartialEq)]
pub enum DeviceMode {
    Disconnected,
    LegacyBootloader,
    Firmware,
    WebUsbBootloader,
}

#[derive(Clone, serde::Serialize, Debug)]
pub struct DeviceState {
    pub mode: DeviceMode,
    pub feature_info: Option<FeatureInfo>,
    pub error_message: Option<String>,
}

pub struct DeviceController {
    state: Arc<Mutex<DeviceState>>,
    // Optionally: handles to background threads, USB watchers, etc.
}

impl DeviceController {
    pub fn new() -> Self {
        // Initialize with Disconnected state
    }

    pub fn start_detection(&self, app: AppHandle) {
        // Spawns background threads to monitor all interfaces/libraries
        // On state change, emits Tauri event (see section 4)
    }

    pub fn get_state(&self) -> DeviceState {
        // Returns current state
    }
}
```

**Where to Import/Instantiate:**  
- Add `mod device_controller;` at the top of `src-tauri/src/lib.rs` (line 1–5).
- Create a global or static instance in `lib.rs` (just after imports, before Tauri commands).

---

### 2.2. Detection Logic

**Location:**  
- Inside `DeviceController::start_detection()` in `device_controller.rs`.

**Justification:**  
- Keeps all detection logic in one place.
- Allows for easy spawning of threads for each USB library.

**Code Skeleton:**
```rust
pub fn start_detection(&self, app: AppHandle) {
    // Spawn a thread for hidapi
    // Spawn a thread for rusb
    // Optionally: spawn for webusb
    // Each thread:
    //   - Loops, checks for device(s) on relevant interfaces/PIDs
    //   - If state changes, updates self.state and emits Tauri event
}
```

---

### 2.3. Tauri Command for Manual State Query

**Location:**  
- In `src-tauri/src/lib.rs`, after controller instantiation.
- Example: after line where Tauri commands are registered.

**Justification:**  
- Allows frontend to query current state at any time (e.g., on app load).

**Code:**
```rust
#[tauri::command]
fn get_device_state() -> DeviceState {
    DEVICE_CONTROLLER.get_state()
}
```

---

## 3. Real-Time State Emission

### 3.1. Tauri Event Emission

**Location:**  
- Inside `DeviceController::start_detection()` (or helper function), whenever state changes.

**Justification:**  
- Tauri events are the canonical way to send real-time updates from Rust to the frontend.

**Code:**
```rust
app.emit_all("device_state_changed", &new_state).unwrap();
```

---

## 4. Frontend Integration

- Listen for `"device_state_changed"` events.
- On event, update UI to reflect new state.
- Optionally, call `get_device_state` on load for initial state.

---

## 5. Justification for Structure

- **Modularity:** All device logic is in one file, easily testable and extensible.
- **Thread Safety:** Uses `Arc<Mutex<...>>` for state sharing across threads.
- **Performance:** Background threads avoid blocking Tauri commands/UI.
- **Real-Time:** Tauri events ensure instant updates with no polling.

---

## 6. Next Steps

- Implement `device_controller.rs` as above.
- Refactor detection code out of `lib.rs` into the controller.
- Add frontend event listener and UI updates.

---

## Summary Table of Code Insertions

| File/Section                       | Line/Location                  | Code/Struct/Function                 | Justification                                    |
|-------------------------------------|-------------------------------|--------------------------------------|--------------------------------------------------|
| `device_controller.rs`              | Top of file                   | DeviceMode, DeviceConnectionState    | Centralized state model                          |
| `device_controller.rs`              | After structs                 | DeviceController struct/impl         | Encapsulate polling and state logic              |
| `device_controller.rs`              | In polling loop               | Tauri event emission                 | Real-time frontend updates                       |
| `main.rs` or `lib.rs`               | Backend startup               | device_controller.start_detection()  | Ensure detection runs at app launch              |
| `lib.rs`                            | Tauri commands section        | get_device_state command             | On-demand state fetch for frontend               |

---

**This plan ensures a robust, extensible, and idiomatic Rust/Tauri architecture for KeepKey device state detection and real-time UI updates.**
