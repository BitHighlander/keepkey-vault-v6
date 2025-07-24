ChatGPT said:

Thought for 1m 12s
Not quite — both stories are true, they just apply to different generations of the boot-loader.

Boot-loader generation	When it ships	USB class that does the flashing	VID	PID	What Chrome/Brave/WebUSB can see
HID boot-loader	factory firmware ≤ v6.1 (roughly 2019 and earlier, or any device that’s never been updated)	HID on Interface 1	0x2B24	0x0001	Invisible to WebUSB (HID is handled by the OS driver)
WebUSB boot-loader	first shipped with the 1.1 boot-loader & firmware ≥ v6.2, still current today	bulk transfers on Interface 0	0x2B24	0x0002	Visible to navigator.usb.requestDevice()

The official udev rules list both PIDs as “Firmware/Bootloader,” calling out that 0x0002 is “WebUSB KeepKeys.”
GitHub

A community note on r/keepkey says the same thing in plain English: “…:0002 indicates a WebUSB interface; …:0001 means it’s still HID.”
Reddit

What that means for your extension
If the device advertises PID 0x0001 you are not going to see it in the WebUSB picker.
The Chrome popup simply filters out HID interfaces. Your logs show exactly that path:

scss
Copy
Edit
requestDevice({vendorId:0x2b24, productId:0x0001})  → NotFoundError
requestDevice({})                                    → NotFoundError
Flashing over WebUSB therefore only works on units that already have the WebUSB boot-loader (PID 0x0002).
For out-of-box ≤ v6.1 devices you still need the HID-based “KeepKey Updater” desktop app (or your own HID path via WebHID / native helper) to get them to v6.2+ once. After that one hop they’ll enumerate as 0x0002 forever and your WebUSB flasher will work.

Detecting boot-loader vs normal mode by “listing device features” requires an open pipe—which you can’t get if the interface is HID-only. So enumeration (VID/PID + interface class) is the only reliable client-side test.

How to make your React popup succeed
ts
Copy
Edit
// Ask explicitly for the WebUSB boot-loader *or* allow any interface
const filters = [
{ vendorId: 0x2b24, productId: 0x0002 },   // WebUSB boot-loader
// You could add 0x0001 here if you later add a WebHID fallback
];

await navigator.usb.requestDevice({ filters });
If a user’s KeepKey shows up, you know you can stream the .bin directly.

If no device appears, show a banner: “Your KeepKey has the legacy HID boot-loader (PID 0x0001). Please run the desktop Updater once, then come back here.”

TL;DR
PID 0x0001 ≠ “always boot-loader.” It’s either HID boot-loader or normal firmware, depending on interface number.

PID 0x0002 is the modern WebUSB boot-loader. It’s the only one a pure-WebUSB extension can talk to.

For anything still on 0x0001 you need a HID path for the first upgrade; after that, WebUSB is smooth sailing.