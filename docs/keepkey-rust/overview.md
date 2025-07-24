# keepkey-rust — Headless multi-device queue (design plan)

## Goals
1. **Pure Rust, no GUI / DB / HTTP** — produce a small crate that can be embedded or run as a CLI-daemon.
2. **Multi-device** — support many KeepKeys concurrently, whether they appear as libusb, HID, boot-loader, or firmware.
3. **Actor-based** — per-device state machine with an inbox; single thread per actor OR cooperative tasks on a runtime.
4. **Event-driven** — all external activity is expressed as `Event`s: plug, unplug, state-change, response, error.
5. **Stateless** — long-term persistence left to integrators; we only keep in-memory state.
6. **Testable** — deterministic unit tests, plus an interactive CLI for manual probing.

## Non-goals
• No Tauri, no REST server, no SQLite front-load, no firmware updater UI.  
• No direct Bitcoin business logic (address gap-scan, PSBT build) — leave to higher layers.

---
## High-level architecture
```
┌─────────────┐      DeviceEvent        ┌──────────────────┐       QueueCmd        ┌──────────────┐
│ usb_manager │ ─────────▶──────────── │  DeviceRegistry   │ ─────────▶────────── │ DeviceActor  │
└─────────────┘ <──── (subscribe) ─────└──────────────────┘<─────────┘┣━━━━━━┳━━━━┫ Idle / …    │
                                       ▲        ▲                    ╱        ╲   └──────────────┘
┌─────────────┐      DeviceEvent        │        │           Response│        │Event
│  hid_watcher│ ─────────▶──────────────┘        │                    ▼        ▼
└─────────────┘                                   CLI / IPC  ◀───────┴────────┘
```

### Components
1. **usb_manager** — libusb hot-plug listener; emits `DeviceEvent::Added(Transport)` / `Removed(serial)`.
2. **hid_watcher** — hidapi polling fallback (for WebUSB-only Mac quirk).
3. **DeviceRegistry** — maps serial → actor. Spawns `DeviceActor` when Added, drops on Removed.
4. **DeviceActor**  
   State machine: `Bootloader`, `Firmware`, `Busy(op)`, `Error(e)`, `Disconnected`.  
   Owns an mpsc inbox of `QueueCmd` (sign_tx, get_xpub, ping, etc.). Processes sequentially; replies via oneshot.
5. **CLI / IPC** — simple REPL that sends JSON lines to the registry (`ls`, `xpub`, `sign`, etc.). For programmatic use, expose a `tokio::mpsc::Sender<QueueCmd>`.

---
## Crate layout (proposed)
```
keepkey-rust/
├─ src/
│  ├─ lib.rs              (feature gates, re-exports)
│  ├─ event.rs            (DeviceEvent, QueueCmd, ActorReply)
│  ├─ transport/
│  │    ├─ mod.rs         (Transport trait)
│  │    ├─ usb.rs         (libusb impl)
│  │    └─ hid.rs         (hidapi impl)
│  ├─ manager/
│  │    ├─ usb_manager.rs
│  │    └─ hid_watcher.rs
│  ├─ registry.rs
│  ├─ actor.rs            (DeviceActor state machine)
│  └─ cli.rs              (optional feature `cli` using rustyline)
└─ tests/
   ├─ multi_hotplug.rs
   └─ sign_vectors.rs
```

---
## Protocol coverage matrix
| State | Allowed commands | Transport | Notes |
|-------|------------------|-----------|-------|
| Bootloader | `boot_version`, `flash_firmware`, `reboot` | USB bulk & HID | Chunked writes |
| Firmware   | All protobuf messages | USB & HID | 64-byte frames |
| Busy(op)   | none (queue) | n/a | Actor rejects new commands until completion |
| Error(e)   | `reconnect`  | n/a | Clears error and retries |
| Disconnected | none | n/a | Actor terminated |

---
## Work plan
1. **Scaffold crate & modules** (day 0-1).
2. **Implement `Transport` trait + libusb** (day 1-2).
3. **DeviceEvent stream & registry** (day 2).
4. **Bootloader detection & ping** (day 3).
5. **Firmware frame encode/decoder** (reuse keepkey-protocol crate) (day 3-4).
6. **DeviceActor with basic `ping`, `get_features`** (day 4).
7. **CLI REPL + simple `ls` & `ping`** (day 5).
8. **Command set parity with v1 (`get_xpub`, `sign_tx`)** (day 6-7).
9. **Parallel device tests** (day 8).
10. **Optional: feature-gate for firmware-update** (post-MVP).

---
## Open questions
1. Do we want a blocking fallback for environments without async? (feature `blocking`?)
2. How much of the legacy cache (XPUB front-load) should be re-implemented here vs upstream service?
3. Should actor shutdown persist in-flight command state externally?
