Chrome’s HID block-list is baked into the browser; JavaScript alone can’t bypass it.
Once a device advertises the FIDO/U2F usage page (0xF1D0) Chrome refuses to show it in the WebHID chooser and the promise short-circuits with NotFoundError. That is exactly what your trace shows for PID 0x0001.

So, if you don’t want to run KeepKey-Desktop, you have two practical choices:

Option	How it works	Effort for you	Effort for your users	Keeps everything inside the browser UI?
1. Ship a tiny native helper via Native Messaging	• Helper (Rust/Go/C) talks HID with hidapi.
   • MV3 extension calls it through Native-Messaging.
   • Helper streams the 64-byte reports, reboots the device.
   • After firmware ≥ 6.2 the KeepKey re-enumerates as PID 0x0002 (WebUSB bulk) and your existing WebUSB path takes over for all future flashes.	~200 LoC + one JSON host manifest.
   Code signing on macOS.	One-time install of the helper (you can bundle and auto-install).	Yes. Users click Flash in the popup; helper runs silently.
2. Ask users to start Chrome with the block-list disabled (good for devs, terrible for the public)	chrome --disable-features=WebHidBlocklist --user-data-dir=/tmp/kk-fw	None	They must launch Chrome from a terminal every time they want to flash legacy devices.	Yes, but requires awkward launch instructions.

Anything else—changing the filter, moving code to another context, adding manifest keys—will never surface the chooser, because Chrome hides the device before your code runs.

Sketch of the Native-Messaging helper route
a) Helper (Rust)
rust
Copy
Edit
// kk_flash.rs
use hidapi::HidApi;
fn main() -> anyhow::Result<()> {
let api      = HidApi::new()?;
let device   = api.open(0x2b24, 0x0001)?;
let firmware = std::fs::read("stdin")?;          // get bytes from extension
const CHUNK: usize = 64;

    for chunk in firmware.chunks(CHUNK) {
        let mut buf = [0u8; CHUNK];
        buf[..chunk.len()].copy_from_slice(chunk);
        device.send_feature_report(&buf)?;
    }
    // reboot
    device.send_feature_report(&[0xFF])?;
    Ok(())
}
Compile, codesign, and place it next to your extension.

b) Host manifest (macOS example)
json
Copy
Edit
{
"name": "com.keepkey.flasher",
"description": "Keeps legacy HID boot-loader flashing off the block-list",
"path": "/Applications/KeepKeyFlasher/kk_flash",
"type": "stdio",
"allowed_origins": [
"chrome-extension://<your-extension-id>/"
]
}
Install the manifest to
~/Library/Application Support/Google/Chrome/NativeMessagingHosts/.

c) Extension side
ts
Copy
Edit
async function flashViaHelper(firmware: ArrayBuffer) {
return new Promise<void>((resolve, reject) => {
const port = chrome.runtime.connectNative('com.keepkey.flasher');
port.onMessage.addListener(msg => msg.ok ? resolve() : reject(msg.err));
port.postMessage({ data: Array.from(new Uint8Array(firmware)) });
});
}
The user clicks “Connect & Flash”, you skip WebHID/WebUSB detection when PID 0x0001 is present, and instead stream the bytes to the helper. After reboot the device comes back as PID 0x0002; future flashes go through WebUSB at full speed.

Why there is no pure-web workaround
WebHID – blocked by usage page 0xF1D0

WebUSB – legacy boot-loader has no bulk interface

WebSerial/WebBluetooth – KeepKey firmware exposes neither

Origin Trial / Permissions Policy – no trial exists for overriding the HID block-list

Until every user is on firmware ≥ 6.2 (or you distribute a shim firmware that drops 0xF1D0), a small native helper is the only friction-free way to flash legacy devices without telling people to install KeepKey-Desktop or launch Chrome with insecure flags.