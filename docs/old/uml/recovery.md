sequenceDiagram
participant User
participant Frontend
participant Backend
participant Device
participant USB

    Note over User,USB: Recovery Process with Disconnection Handling
    
    User->>Frontend: Start Recovery
    Frontend->>Backend: start_device_recovery(device_id="39323130114737323E001F00")
    Backend->>Device: RecoveryDevice message
    Device->>Backend: PinMatrixRequest
    
    User->>Frontend: Enter PIN
    Frontend->>Backend: send_recovery_pin_response()
    Backend->>Device: PinMatrixAck
    Device->>Backend: CharacterRequest (word 0, char 0)
    
    Note over Frontend: Recovery locked with original ID
    Frontend->>Frontend: isRecoveryLocked = true<br/>originalDeviceId = "39323130114737323E001F00"
    
    User->>Frontend: Enter characters
    Frontend->>Backend: send_recovery_character()
    Backend->>Device: CharacterAck
    
    Note over USB: Device disconnects unexpectedly
    USB-->>Backend: Device disconnected
    Backend->>Backend: Check is_device_in_recovery_flow()<br/>Result: true - preserve queue
    Note over Backend: Queue NOT cleaned up
    
    USB->>Backend: New device connected<br/>ID: "keepkey_2b24_0002_bus0_addr5"
    Backend->>Backend: are_devices_potentially_same()?<br/>Result: true
    Backend->>Backend: add_recovery_device_alias()
    Backend->>Frontend: device:recovery-reconnected event
    
    Note over Backend: Alias mapping:<br/>"keepkey_2b24_0002_bus0_addr5" → "39323130114737323E001F00"
    
    Frontend->>Frontend: Show "Device reconnected" message
    
    User->>Frontend: Continue entering characters
    Frontend->>Backend: send_recovery_character()<br/>(uses original session ID)
    Backend->>Backend: get_canonical_device_id()<br/>Resolves to original ID
    Backend->>Device: CharacterAck<br/>(via reconnected transport)
    Device->>Backend: CharacterRequest/Success
    
    Note over User,USB: Recovery continues seamlessly