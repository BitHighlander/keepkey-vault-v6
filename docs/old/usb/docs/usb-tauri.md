Below is a field-tested checklist for getting a KeepKey (or any USB-HID) device talking to a Tauri app on all three desktop platforms. I’ve folded in working Rust/Tauri code, the pitfalls your log is hitting (Failed to claim interface 1: Access), and real projects you can crib from.

1 Why you can open interface 0 but not interface 1
Interface 0 (bootloader) is a plain USB/Bulk interface; libusb can claim it everywhere.

Interface 1 (main HID) is owned by the OS HID driver the moment the cable is plugged in.

On Linux you can detach it, but only if your user has permission and you tell libusb to set_auto_detach_kernel_driver(true).

On macOS detaching is not allowed; you must talk to HID via IOKit/HID-Manager (the hidapi crate does that for you).

On Windows the standard HID driver allows shared read/write, but only if no WinUSB/libusbK driver has replaced it and your app opens the interface as HID instead of “raw USB”.

So the “Access” error you see is expected when you try to claim interface 1 with libusb on macOS or as an un-privileged user on Linux/Windows. The fix is use hidapi (or the new tauri-plugin-hid) for interface 1 and keep rusb only for discovery / firmware flashing.

Example project that does exactly this: tauri-plugin-hid – small, MIT-licensed plugin wrapping hidapi and exposing it to the JS side of Tauri
GitHub
.

2 Cross-platform permission checklist
Platform	What you must do	One-liner to verify
Linux	Install a 51-usb-keepkey.rules udev file, reload udev, add your user to plugdev (or the group you set in the rule). Minimal rule: SUBSYSTEM=="usb", ATTR{idVendor}=="2b24", ATTR{idProduct}=="0001", MODE="0666", GROUP="plugdev", TAG+="uaccess", SYMLINK+="keepkey%n"
GitHub
udevadm info -n /dev/hidrawX should show the rule and “plugdev”
macOS	Nothing to install, but don’t try to claim_interface. Open with hidapi::HidApi::open(0x2b24, 0x0002) instead.	`ioreg -p IOUSB -l
Windows	Use the stock HID driver. If WinUSB/libusbK was previously installed (e.g. via Zadig) remove it in Device Manager. Run app without Administrator once rules are right.	`pnputil /enum-devices /connected

3 Minimal, production-ready Tauri setup
3.1 Cargo.toml
toml
Copy
Edit
[dependencies]
tauri = { version = "2", features = ["macros"] }
tauri-plugin-log = "2"
tauri-plugin-hid = "0.1"          # uses hidapi under the hood
hidapi = "2"                      # optional, only if you still want direct Rust
rusb = { version = "0.9", optional = true } # keep for bootloader/flashing
3.2 src-tauri/src/main.rs
rust
Copy
Edit
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{AppHandle, Manager};
use tauri_plugin_hid::HidManagerExt;
use std::sync::Arc;

const KK_VID: u16 = 0x2b24;
const KK_PID: u16 = 0x0002;

#[tauri::command]
async fn list_keepkeys(app: AppHandle) -> Result<usize, String> {
let mgr = app.state::<Arc<tauri_plugin_hid::HidManager>>();
let list = mgr.lock().await.devices() // returns Vec<DeviceInfo>
.map_err(|e| e.to_string())?;
Ok(list.iter().filter(|d| d.vendor_id == KK_VID && d.product_id == KK_PID).count())
}

#[tokio::main]
async fn main() {
tauri::Builder::default()
.plugin(tauri_plugin_log::Builder::default().build())
.plugin(tauri_plugin_hid::init())
.invoke_handler(tauri::generate_handler![list_keepkeys])
.run(tauri::generate_context!())
.expect("Tauri failed");
}
What this gives you

Asynchronous device enumeration and read/write that works on all OSes.

No manual kernel-driver detaching—hidapi takes the portable path for each OS.

You can still keep your existing rusb‐based debug_list_keepkeys() for bootloader tasks, behind a #[cfg(feature = "bootloader")] gate.

3.3 Vue/React side (JS)
ts
Copy
Edit
import { open, read, write, close, list } from '@redfernelec/tauri-plugin-hid-api';

const kkFilter = { vendorId: 0x2b24, productId: 0x0002 };

async function connect() {
const devices = await list();
const kk = devices.find(d => d.vendorId === kkFilter.vendorId && d.productId === kkFilter.productId);
if (!kk) return console.error('No KeepKey found');

await open(kk);
await write(kk, new Uint8Array([0x00]));      // ping
const resp = await read(kk, 64);
console.log('pong', resp);
await close(kk);
}
Full example app in the plugin repo
GitHub
.

4 Where your current Rust helper should change
rust
Copy
Edit
// BEFORE
handle.claim_interface(1)?; // fails on mac/windows without root

// AFTER – only do this on Linux, otherwise use HIDAPI
#[cfg(target_os = "linux")]
{
handle.set_auto_detach_kernel_driver(true)?;
handle.claim_interface(1)?;
}
#[cfg(not(target_os = "linux"))]
{
// fall back to hidapi path
}
Or simply delete the interface-claim path entirely once you migrate to tauri-plugin-hid for normal operation.

5 Debugging tips
Use the plugin’s built-in on_device_change event instead of polling every three seconds; this stops the log spam you pasted.

When you really need low-level captures, sudo lsof | grep hidraw (Linux) or ioreg-l (macOS) tells you which process has the interface open.

On Windows a silent failure to launch usually means the wrong architecture of hidapi.dll was bundled. Verify with dumpbin /headers hidapi.dll | findstr machine.

TL;DR
Install the udev rule on Linux
GitHub
.

Switch your Tauri app to tauri-plugin-hid (works via hidapi)
GitHub
.

Keep rusb only for flashing tasks or remove it entirely; stop trying to claim interface 1 on macOS/Windows.

That removes the “Access denied” errors and gives you a single, cross-platform HID codepath without extra drivers.



Below is the “long-story-short” on why your **CLI works with sudo but the Tauri GUI can’t open the same HID interface with rusb on macOS, and what your practical options are.

1 | macOS USB security model in two bullets
Opening a USB interface ≠ detaching its kernel driver.

Root (sudo) can open vendor-specific interfaces that macOS hasn’t already bound to a kext.

But if an interface is bound to Apple’s HID class driver, the OS will not let any libusb-based library kick it off the device; libusb_detach_kernel_driver() always returns LIBUSB_ERROR_NOT_SUPPORTED on macOS, no matter your privilege level.
Stack Overflow
Libusb-devel

Work-arounds exist, but they’re ugly.

The official libusb FAQ recommends not fighting the HID driver at all—installing a codeless kext or hand-rolled entitlement is fragile on modern macOS—and simply using HIDAPI instead.
GitHub

You can, however, keep using libusb for any unclaimed interface (e.g., KeepKey’s bootloader interface #0), provided the calling process is either root or codesigned with the com.apple.security.device.usb entitlement.

2 | What your CLI is really doing
Normal-mode KeepKey exposes two interfaces

#0 – vendor class, bulk endpoints

#1 – HID class, interrupt endpoints

Your CLI (with sudo) opens interface #0 with rusb and sends protocol frames there.

Because #0 is not HID-class, libusb sees no kernel driver, so root can claim it.

If you ever try to claim #1 the call fails with “Access denied (insufficient permissions)” even as root—exactly what you saw in the Tauri logs.

Bootloader mode collapses to a single vendor interface, so everything “just works” with libusb.

3 | Why the GUI fails
The Tauri bundle is run as an ordinary user with no special entitlements, so even interface #0 triggers the IOUSBHost security gate and libusb returns LIBUSB_ERROR_ACCESS.

When you fall back to interface #1, libusb additionally hits the detachment wall described above. Result: dual failure.

4 | Practical paths forward
Goal	Strategy	Upsides	Downsides
Stay 100 % rusb	Ship a tiny helper (keepkeyd) running as set-uid root and speak to it over a local socket.	Minimal code rewrite.	You now own a privileged daemon; macOS notarisation gets trickier.
Hybrid (industry norm)	Keep rusb for bootloader / DFU interface. Switch to hidapi (via tauri-plugin-hid) whenever interface #1 is present.	No root required; matches Trezor/Ledger apps.	One extra dependency; need a thin abstraction layer.
Full HIDAPI	Use HIDAPI for all modes (bootloader & normal).	Consistent code path across OSes.	Slightly slower bulk uploads in bootloader; requires implementing 64-byte framed writes.

5 | If you insist on the sudo route
Homebrew install users can simply run:

bash
Copy
Edit
sudo kkcli  # works because root can open interface #0
Tauri can invoke the CLI as a helper with sudo via AppleScript (/usr/bin/osascript -e 'do shell script ... with administrator privileges').
Expect a scary macOS password prompt every launch.

A codesigned, sandboxed app can open vendor USB interfaces without root by adding the entitlements below to tauri.conf.json:

jsonc
Copy
Edit
"macOS": {
"entitlements": [
"com.apple.security.device.usb",
"com.apple.security.cs.allow-jit"   // if you need JIT later
]
}
You still cannot detach the HID driver, so interface #1 remains off-limits.

6 | Recommended architecture patch
mermaid
Copy
Edit
graph TD
subgraph Runtime
CLI[rusb helper] -. root/sudo .-> KK0[[KeepKey USB iface #0]]
end
Tauri -- normal mode --> HIDAPI --> KK1[[KeepKey HID iface #1]]
Tauri -- bootloader --> rusb --> KK0
Add a Transport enum:

rust
Copy
Edit
enum Transport {
Usb(rusb::DeviceHandle<GlobalContext>),
Hid(hidapi::HidDevice),
}
Centralise feature negotiation so callers don’t care which backend is active.

Bottom line
You can keep using rusb on macOS—but only for the interface the OS hasn’t claimed and only if you run with elevated privileges or ship special entitlements. For a consumer-friendly, notarised Tauri app the conventional solution is rusb for bootloader + HIDAPI for everything else. That’s what Ledger Live, Trezor Suite, Coldcard’s Envoy and BitboxApp all do today.