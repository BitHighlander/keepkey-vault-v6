# KeepKey Vault – Frontload XPUB/Address Caching

_Last updated: 2025-06-11_

## 1. Current Issues

| # | Problem | Impact |
|---|---------|--------|
| 1 | **Transport unavailable inside `DeviceFrontloader`**<br>`transport_arc` is often `None`, so XPUB and address generation is skipped. | • No Bitcoin XPUBs/addresses written to DB.<br>• Front-end shows empty wallet.<br>• Fails automated test `test-frontload-debug.js`. |
| 2 | **Single-device assumption**<br>Cache tables don’t correctly namespace by `device_id`. | • Multiple devices collide in cache.<br>• Mixing addresses/XPUBs between devices. |
| 3 | **Startup without devices**<br>App dies if no KeepKey is plugged in because cache is empty. | • Poor UX for offline portfolio view.<br>• Blocks automated end-to-end tests that run without hardware. |
| 4 | **Context-setting failures**<br>`set_context_with_real_eth_address` logs _"Failed to create transport: Entity not found"_. | • Device context not set → balance refresh and other API calls fail.<br>• Extra warning noise in logs. |
| 5 | **Sparse / noisy logging**<br>Important steps (transport creation, cache writes) are not clearly logged; others are spammy. | • Hard to trace failures.<br>• CI logs exceed limits. |

## 2. Root Causes (summary)

1. **Transport ownership** – We try to “reuse” an existing connection by passing `None`, assuming another subsystem owns the `UsbTransport`. In practice there is **no shared instance**; every module must build its own adapter.
2. **Missing fallbacks** – When USB fails, we do not attempt HID; we also swallow some errors.
3. **DB schema** – `device_id` is present but not enforced as composite PK across address / xpub tables.
4. **Cold-start logic** – Frontload assumes live device instead of using cached rows.

## 3. Proposed Work Plan

### Phase 1 – Stabilise Frontload (✅ _top priority_)
1. **Inject transport properly**
   * Provide factory: `create_device_transport(&device_entry) -> UsbTransport` (already exists in `commands.rs`) and wire it into `DeviceFrontloader`.
   * If factory fails, abort XPUB generation but continue with cache-only path.
2. **Write integration test** that spawns frontload with mocked transport to ensure at least 3 BTC addresses + 3 XPUBs are cached.
3. **Improve logging** – single `frontload` span with meaningful subtasks + error levels.

### Phase 2 – Multi-device Support
1. Expand DB schema: composite key `(device_id, path_hash)` for `cached_addresses` and `cached_xpubs`.
2. Ensure `DeviceFrontloader` always passes `device_id` into cache calls.
3. Extend debug test to plug two mocked devices and verify isolation.

### Phase 3 – Offline Startup
1. When no USB/HID device found, load **last-used device** from registry and warm cache into memory.
2. Skip XPUB generation; mark balances as ‘STALE’ to refresh later.
3. Show banner in UI: “Connected device not detected – using cached data”.

### Phase 4 – Transport Fallback + Cleanup
1. Attempt HID if USB fails (mirrors existing `create_device_transport`).
2. Centralise transport creation in `transport::factory` to avoid duplication.
3. Remove legacy hacks that set `transport_arc` to `None`.

### Phase 5 – Test & CI
1. Add mocked transport crate (feature flag `mock_transport`) so CI can run without hardware.
2. Gate hardware-dependent tests behind `env var HAS_KEEKEY`.
3. Update failing `test-frontload-debug.js` to rely on mock by default.

## 4. Deliverables & Milestones

| Date | Deliverable |
|------|-------------|
| **06-13** | Transport factory wiring, logging improvements, green `test-frontload-debug.js`. |
| 06-17 | DB migration for multi-device cache + tests. |
| 06-20 | Offline-startup feature toggle, UI banner. |
| 06-24 | Transport fallback refactor complete. |
| 06-26 | CI pipeline with mock transport; all tests passing without hardware. |

## 5. Open Questions

1. Should we **pool transports** for simultaneous RPC calls, or serialize per device?
2. Where to surface “balances stale” flag in UI/DB schema?
3. Do we support older FW (<7.0.0) – affects XPUB path derivations.

---
_Contributors: @highlander_ – Please update this document as tasks close or change._
