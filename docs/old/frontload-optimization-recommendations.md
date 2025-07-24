# Frontload Optimisation & Architecture Recommendations  
*(based on 2025-06-11 `frontload-post.md` log plus current `DeviceFrontloader` source)*

---

## 1  Key Findings

| # | Symptom in Logs | Root Cause in Code |
|---|-----------------|--------------------|
|1|`create_device_transport` called **for every address / XPUB** – hundreds of invocations.|`get_and_cache_*` helpers each call `self.create_device_transport()` instead of re-using a single connection.|
|2|Each call scans **all attached USB devices** (`list_devices()`) – ~1-2 ms per scan but multiplied by ~600 = noticeable startup lag.|USB enumeration is inside `create_transport_from_device_info` for every invocation.|
|3|Features are fetched & saved repeatedly during loops.|`frontload_features()` is called once but individual address helpers fetch features again when error-handling.|
|4|Multiple openings of `device_cache.db` (lines 19, 47, 103, 187 …).|`DeviceCache::open()` called every save/load; the struct doesn’t hold a persistent pooled `Connection`.|
|5|Mixed transport behaviour – USB fails first then HID succeeds; later USB works.|Legacy `transport_arc` unused; each new call re-opens a transport which may race with HID hot-plug listener.|
|6|Blocking USB/HID I/O executed directly on the async runtime thread.|`rusb` & `hidapi` APIs are synchronous; they should run inside `tokio::task::spawn_blocking` to avoid starvation.|
|7| Verbose per-packet HID logging slows frontload by ~10-15 %.|`transport::hid` logs every read/write at INFO level.|

---

## 2  Quick Wins (< 1 day)

1. **Reuse a single transport during frontload**  
   • Create it once at the start of `frontload_all_with_progress` and pass a `&mut dyn ProtocolAdapter` reference to helpers.  
   • Remove `create_device_transport()` calls inside inner helpers.

2. **Open device_cache only once**  
   • Hold `rusqlite::Connection` in `DeviceCache` struct; open in `new` or lazy-init, reuse for all ops.

3. **Turn down noisy HID per-packet logs**  
   • Change to `debug!` or behind a `TRACE_HID` feature flag.

4. **Cache device features in memory**  
   • After the first `GetFeatures`, store it on the `DeviceFrontloader` and avoid redundant RPCs.

---

## 3  Medium-Term Refactors (1-2 weeks)

### 3.1  Transport Manager / Pool

```
┌────────────────────┐        get()         ┌─────────────────┐
│ TransportManager   ├────────────────────►│ UsbTransport    │
│  HashMap<id, Tx>   │◄────────────────────┤ HidTransport    │
└────────────────────┘        return()      └─────────────────┘
```
*Singleton inside `tauri::State` that owns one live transport per device.*

• Guarantees **single open handle** preventing HID/USB contention.  
• Exposes async `with_transport(id, |tx| async { … })` to execute critical sections.

### 3.2  Address / XPUB Batch RPCs

KeepKey firmware supports `GetAddress` with `batch=True` (or "address_n" array).  
Send paths **in groups of 10** to cut round-trip time by ~70 %.

### 3.3  Async / Spawn Blocking Boundary

```rust
let features = tokio::task::spawn_blocking(move || transport.get_features())
    .await??;
```

Move **every HID / USB read-write** into `spawn_blocking` to avoid blocking Tokio executor threads.

### 3.4  Remove `transport_arc`

Legacy field is now misleading. Eliminate and replace with `Option<FriendlyUsbDevice>` + Manager above.

---

## 4  Long-Term Architecture (~1 month)

### 4.1  Event-Driven Frontload Pipeline

1. **DeviceReady event** emitted by `device_controller`.
2. **FrontloadWorker** consumes event, obtains transport from pool, and executes pipeline:
   1. *EnsureCacheSchema*
   2. *FetchFeatures* (if stale)
   3. *EnsureDefaultPaths*
   4. *AddressBatchSync* (parallel asset pipelines, limited concurrency = 1 transport)
3. Emits `FrontloadProgress` via channel/WS to UI.

Benefits: decouples heavy frontload from REST request thread, enables cancellation and retry, supports multiple devices concurrently.

### 4.2  Database Layer

• Migrate to **single SQLite connection per process** with `r2d2` or `sqlx::SqlitePool`.  
• Use PRAGMA `journal_mode=WAL` for read-heavy workloads.

### 4.3  Mock Transport Feature

`transport-mock` crate implementing `ProtocolAdapter` to return canned responses so CI can run frontload tests without hardware.

---

## 5  Potential Bugs Detected

| Line | Issue | Likely Fix |
|------|-------|------------|
|108|Context fails with `Entity not found` just before frontload.|Call to `set_context_with_real_eth_address` still relies on old `create_device_transport`; migrate to new pool.|
|136|`FAILING FAST: No transport available` then continues anyway.|Return error early OR demote to warn once fallback path works.|
|205|HID read timeout → immediate retry with new transport.|KeepKey sometimes NAKs after first XPUB; consider exponential back-off instead of full reconnect.|

---

## 6  Next Steps Checklist

- [ ] Implement **single-transport reuse** inside frontload.
- [ ] Turn HID packet logs to DEBUG.
- [ ] Benchmark frontload time before/after (expect 40-60 % decrease).
- [ ] Design `TransportManager`; spike implementation behind feature flag.
- [ ] Add integration test using `transport-mock` with 100 address batch.

---

*Prepared by Cascade – 2025-06-11*
