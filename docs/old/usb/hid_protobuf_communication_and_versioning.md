# KeepKey HID & Protobuf Communication for Device Interaction

This document outlines the communication flow between KeepKey Desktop and the KeepKey hardware device, focusing on how device information, particularly the version number, is retrieved. It covers the HID transport layer, Protobuf message serialization, and the relevant Rust code implementation.

## Core Technologies

1.  **USB HID (Human Interface Device):** Used as the low-level transport protocol for sending and receiving data packets (reports) to/from the KeepKey device. Each report is typically 64 bytes.
2.  **Protobuf (Protocol Buffers):** Used for defining the structure of messages exchanged between the host (KeepKey Desktop) and the device. This allows for language-agnostic, efficient serialization and deserialization of data.

## Protobuf Definitions

The protobuf definitions are primarily split into two files located in `projects/keepkey-desktop/src-tauri/device-protocol/`:

*   **`types.proto`:** Defines common data structures and enums used across various messages. Examples include:
    *   `CoinType`: Describes properties of a supported cryptocurrency (name, shortcut, address types, etc.).
    *   `PolicyType`: Describes device policies (name, enabled status).
    *   Various enums for failure types, script types, button requests, etc.

*   **`messages.proto`:** Defines the actual request and response messages. Key messages for device initialization and feature retrieval include:
    *   `Initialize` (Message Type 0): A message sent from the host to the device to initiate communication. It expects a `Features` message in response.
    *   `GetFeatures` (Message Type 55): An alternative request for device features, also expecting a `Features` message.
    *   `Features` (Message Type 17): A crucial response message from the device, providing a wealth of information about its current state, capabilities, and identity.

### The `Features` Message

Defined in `messages.proto` (lines 215-246), this message is central to understanding the device's state. Key fields include:

*   `vendor` (string): Manufacturer name.
*   `major_version`, `minor_version`, `patch_version` (uint32): Firmware version numbers.
*   `bootloader_mode` (bool): True if the device is in bootloader mode.
*   `device_id` (string): Unique identifier for the device.
*   `pin_protection` (bool): Is PIN protection enabled?
*   `passphrase_protection` (bool): Is passphrase protection enabled?
*   `language` (string): Current device language.
*   `label` (string): User-defined device label.
*   `coins` (repeated `CoinType`): List of supported cryptocurrencies. **Note:** This field is a primary suspect for causing the `Features` message to become large.
*   `initialized` (bool): Does the device contain a seed?
*   `revision` (bytes): SCM revision of the firmware.
*   `bootloader_hash` (bytes): Hash of the bootloader.
*   `policies` (repeated `PolicyType`): List of applied device policies.
*   `model` (string): Hardware model of the device.
*   `firmware_hash` (bytes): Hash of the current firmware.

## HID Communication Layer & Rust Implementation

The Rust code responsible for HID communication and protobuf message handling is primarily located in `projects/keepkey-desktop/src-tauri/src/device/backends/hid.rs`.

### HID Message Framing

KeepKey messages sent over HID are wrapped in a custom frame within the 64-byte HID reports:

*   **Report ID:** 1 byte (usually 0, defined as `REPORT_ID`).
*   **Magic Bytes:** 3 bytes (`0x3f`, `0x23`, `0x23`) to identify KeepKey messages.
*   **Message Type:** 2 bytes (Big Endian), indicating the type of protobuf message (e.g., `MSG_TYPE_INITIALIZE`, `MSG_TYPE_FEATURES`). These correspond to the enum values in `messages.proto`.
*   **Data Length:** 4 bytes (Big Endian), indicating the total length of the serialized protobuf message payload that follows.
*   **Payload:** The actual serialized protobuf message data.
*   **Padding:** If the payload is smaller than the remaining space in the HID report, it's padded.

This header totals 1 + 3 + 2 + 4 = 10 bytes, but the initial `REPORT_ID` is handled by the HID system, and the first magic byte `0x3f` is often considered part of a 9-byte logical header for parsing (`?#<type_u16><len_u32>`). This leaves approximately 64 - 1 - 9 = 54 or 64 - 9 = 55 bytes for the protobuf payload *within a single HID report*.

### Key Functions in `hid.rs`

1.  **`prepare_message(msg_type: u16, data: &[u8]) -> Vec<u8>`:**
    *   Constructs a 64-byte HID report ready to be sent.
    *   It prepends the magic bytes, message type, data length, and the first chunk of the protobuf payload (`data`).
    *   **Limitation:** If `data` (the protobuf payload) is larger than ~55 bytes, only the first part is included in this single prepared report. The current implementation does not seem to segment larger payloads across multiple `prepare_message` calls for sending.

2.  **`parse_response(data: &[u8]) -> Result<(u16, Vec<u8>), String>`:**
    *   Takes a raw 64-byte HID report received from the device (`data`).
    *   Validates magic bytes.
    *   Extracts the `msg_type` (e.g., `MSG_TYPE_FEATURES`).
    *   Extracts the `data_len` (the *declared total length* of the protobuf message).
    *   Extracts the payload chunk present *in this single report*. It calculates `actual_data_len = std::cmp::min(data_len, response_buffer.len() - 9)`, effectively limiting the returned payload to what fits in one report (max ~55 bytes).
    *   **Critical Limitation:** This function does not attempt to reassemble a full protobuf message if `data_len` indicates the message spans multiple HID reports. It returns only the partial payload from the current report.

3.  **`exchange_message(device: &HidDevice, msg_type: u16, data: &[u8]) -> Result<(u16, Vec<u8>), String>`:**
    *   The core request/response function.
    *   Calls `prepare_message` to create the outgoing HID report.
    *   Sends the report using `device.write()`.
    *   Reads a *single* HID report from the device using `device.read_timeout()` into a 64-byte buffer.
    *   Calls `parse_response` on this single received report.
    *   **Critical Limitation:** Like `parse_response`, this function inherently handles only the first HID report of a potentially multi-report response. It doesn't loop to collect all parts of a large message.

4.  **`get_features_from_device(device: &HidDevice) -> Result<FeatureInfo, String>`:**
    *   Sends an `Initialize` message to the device by calling `exchange_message(device, MSG_TYPE_INITIALIZE, &[])`.
    *   Receives the (potentially partial) response payload from `exchange_message`.
    *   Attempts to decode this payload using `Features::decode(&data[..])`.
        *   **This is where the "buffer underflow" error occurs** if the `Features` message is large (e.g., due to many `coins`) and `exchange_message` only returned the first truncated part.
    *   If decoding is successful, it populates a `FeatureInfo` struct (defined in `crate::lib.rs`) with data from the parsed `proto_features`.
    *   The firmware version string is constructed using `major_version`, `minor_version`, `patch_version`, and `bootloader_mode` from `proto_features`.

5.  **`try_connect_and_get_features(vid: u16, pid: u16) -> Result<Option<(FeatureInfo, String)>, String>`:**
    *   The entry point often used by the application.
    *   Attempts to open the HID device using VID/PID.
    *   Calls `get_features_from_device` to retrieve and parse features.
    *   Returns the `FeatureInfo` (containing version info) and the device path string.

## Retrieving Device Version in KeepKey Desktop

The process for getting the device version number generally follows these steps:

1.  **Device Detection:** The main application (e.g., in `src/lib.rs` or Tauri commands) scans for connected USB HID devices that match KeepKey's VID/PID.
2.  **Feature Retrieval:** For a detected KeepKey device, `try_connect_and_get_features` is called.
3.  **Initialization Message:** This function, via `get_features_from_device` and `exchange_message`, sends an `Initialize` protobuf message to the KeepKey.
4.  **Features Response:** The KeepKey responds with a `Features` protobuf message.
    *   **Current Issue:** If this `Features` message is larger than ~55 bytes, the device sends it in multiple HID reports. However, `exchange_message` only reads and processes the *first* report.
5.  **Decoding and Version Extraction:**
    *   `get_features_from_device` attempts to decode the (potentially truncated) payload from the first report into a `Features` protobuf object.
    *   If truncated, this decoding fails with a "buffer underflow" error, often when trying to parse a repeated field like `coins` that's cut off.
    *   If (hypothetically) the message were small enough or if multi-report handling was implemented, the `major_version`, `minor_version`, `patch_version`, and `bootloader_mode` fields would be extracted from the decoded `Features` object.
    *   This version information is then packaged into the `FeatureInfo` struct and returned to the caller in KeepKey Desktop.

## Summary of Current "Buffer Underflow" Issue

The primary reason for the "buffer underflow" error during `Features` message decoding is:

*   The `Features` protobuf message, especially when populated with a list of `CoinType` objects in its `coins` field, can easily exceed the ~55 byte payload capacity of a single HID report.
*   The KeepKey device firmware correctly splits these larger messages across multiple HID reports.
*   The current Rust code in `exchange_message` and `parse_response` in `hid.rs` is **not designed to reassemble these multi-report messages**. It reads only the first HID report of the response and attempts to decode its payload as if it were the complete message.
*   When `prost` (the protobuf library) tries to decode this truncated buffer, it expects more data (as indicated by the protobuf encoding itself, especially for repeated fields or fields further down the structure) but hits the end of the buffer prematurely, leading to the underflow error.

To resolve this, `exchange_message` needs to be modified to read the `Data Length` field from the first report's header and then loop, reading subsequent HID reports and concatenating their payloads, until the complete message (as specified by `Data Length`) is received before attempting to decode it.
