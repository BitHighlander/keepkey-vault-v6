# KeepKey Transport Protocol History

## Why So Many Formats?

The complexity of KeepKey transport protocols stems from years of evolution across different platforms and requirements. Understanding this history helps explain why things are the way they are.

## Timeline

### 2014-2016: The Beginning (v1-v3)
- **Original Protocol**: Simple USB HID with basic framing
- **Magic bytes**: None initially, then `0x3F` prefix added
- **Why**: Followed Trezor's protocol closely

### 2017-2019: The v4 Era
- **Protocol**: `[0x3F][0x23][0x23]` magic bytes
- **Header size**: 10 bytes total
- **Key features**:
  - Report ID (0x00) for HID compatibility
  - Three-byte magic sequence for reliability
  - Big-endian encoding
- **Why**: Better error detection, clearer packet boundaries

### 2020-2022: Browser Support
- **WebUSB**: Emerged as browsers blocked HID access
- **Challenge**: Same protocol, different transport layer
- **Solution**: Kept v4 protocol for compatibility

### 2023-2024: The v5 Modernization
- **New Protocol**: Simplified to `##` (0x23 0x23)
- **Header size**: Reduced to 8 bytes
- **Why**: 
  - Cleaner implementation
  - Better protobuf integration
  - Reduced overhead

## The HID Legacy Problem

When v5 was developed, a critical decision was made: keep HID using the old v4 protocol. Why?

1. **Firmware Compatibility**: Many devices in the field still expect v4 format
2. **HID Drivers**: System HID drivers have quirks with format changes
3. **Permission Models**: HID often works when USB doesn't (especially macOS)
4. **Risk Mitigation**: Changing HID protocol risks breaking existing deployments

## Platform-Specific Quirks

### macOS
- USB requires special entitlements
- HID "just works" for most devices
- Result: HID fallback is critical

### Windows
- USB works well with proper drivers
- HID has Report ID requirements
- Result: Both protocols needed

### Linux
- USB requires udev rules
- HID also requires permissions
- Result: Complex permission handling

## Lessons Learned

1. **Document Protocol Versions**: Always version your protocols explicitly
2. **Maintain Compatibility Matrix**: Know which devices support which formats
3. **Test Across Platforms**: Each OS has unique quirks
4. **Keep Legacy Support**: Hardware in the field lasts years
5. **Format Translation**: Sometimes necessary evil for compatibility

## Future Considerations

### Protocol v6 (Hypothetical)
If we were to design a v6:
- Explicit version byte in header
- Self-describing packet format
- Unified across all transports
- Backward compatibility mode

### Best Practices Going Forward
1. Always check device firmware version
2. Implement protocol negotiation
3. Have clear fallback strategies
4. Test with real hardware, not just emulators
5. Document every magic number

## The Golden Rule

**When in doubt, check what the actual hardware expects!**

The device firmware is the source of truth. No amount of clever code can fix a protocol mismatch.

## Quick Decision Tree

```
Is it HID?
  └─ Yes: Use v4 format (0x3F 0x23 0x23)
  └─ No: 
      └─ Is it WebUSB?
          └─ Yes: Use v5 format (##)
          └─ No: Use v5 format (##) for regular USB
```

Remember: The transport layer's job is to be invisible. If developers need to think about transport details, we've failed in our abstraction. 