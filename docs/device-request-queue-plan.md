# KeepKey Device-Request Queue — Design Plan

This document proposes an **in-memory, single-owner queue** that serialises **all** requests to a KeepKey device, records rich performance metrics, and provides per-device response caching.

---
## 1. Problem Statement
* Parallel tasks (features fetch, front-load, HTTP endpoints, etc.) open their own transports → races & "Entity not found / busy" errors.
* Missing central visibility into latency.
* Repeated identical calls waste device bandwidth.

## 2. High-Level Architecture
```
┌────────────────────┐      mpsc::Sender<DeviceCmd>
│  HTTP Endpoints    │──┐
├────────────────────┤  │
│  Front-loader      │──┤   awaits oneshot response
└────────────────────┘  │
                         ▼
                ┌───────────────────────┐  holds one open transport
                │  DeviceWorker (task)  │──────────────────┐
                └───────────────────────┘                  │ USB/HID
                       ▲    ▲    ▲                       ▼
               in-mem cache │    metrics registry   KeepKey
```

### 2.1 Components
| Component | Responsibility |
|-----------|----------------|
| **DeviceCmd enum** | Variants for each device operation, e.g. `SendRaw(Message)`, `GetFeatures`, `GetAddress(Path)`, etc. Each variant carries a `oneshot::Sender<Result<…>>`. |
| **DeviceWorker** | *Single task* per device. Consumes `DeviceCmd`s from an `mpsc` queue, executes them in order with the shared `ProtocolAdapter`. |
| **Cache (HashMap)** | Map `<CacheKey, CachedValue>` where `CacheKey` = `(device_id, op_tag, params_hash)`. TTL or LRU optional. |
| **Metrics** | Captured per request: `queue_wait_ms`, `device_rtt_ms`, `total_ms`. Stored in an in-memory registry (e.g. `metrics` crate) and optionally exported via `/metrics`. |

## 3. API Surface
```rust
pub struct DeviceQueueHandle {
    sender: mpsc::Sender<DeviceCmd>,
}

impl DeviceQueueHandle {
    pub async fn get_features(&self) -> Result<Features>;  // wraps Cmd + await
    pub async fn get_eth_address(&self, path: Vec<u32>) -> Result<String>;
    // … more helpers …
}
```
*Callers never create transports; they only use `DeviceQueueHandle`.*

## 4. Detailed Flow
1. **Caller** constructs helper like `handle.get_features().await`.
2. Helper serialises params → `CacheKey`.
3. If present in cache and **fresh** ⇒ return immediately (`metrics.cache_hit += 1`).
4. Else enqueue `DeviceCmd` with start time, plus oneshot.
5. In `DeviceWorker` loop:
   1. Pop `cmd`.
   2. Record `queue_wait` (now-cmd.enqueued_at).
   3. Execute against transport; measure `device_rtt`.
   4. Send result via oneshot.
   5. Insert into cache with timestamp.
6. Caller receives result; total latency = now-start.

## 5. Cache Strategy
* **Scope**: per-device.  `HashMap<String /*device_id*/, LruCache<CacheKey, (Instant, Value)>>`.
* **Eviction**: size-bounded (e.g. 256 entries) or TTL (e.g. 30 s) — whichever first.
* **Key hashing** must include path / parameters; identical calls hit cache.
* **Mutable ops** (sign tx, reset device) **bypass and purge** cache.

## 6. Metrics
| Metric | Description | Type |
|--------|-------------|------|
| `dq_queue_wait_ms` | Time from enqueue → start. | histogram |
| `dq_device_rtt_ms` | Time spent talking to device. | histogram |
| `dq_total_ms` | End-to-end time. | histogram |
| `dq_queue_depth` | Current queued commands. | gauge |
| `dq_cache_hit_ratio` | hits / (hits+misses) | gauge |

Implementation: use [metrics] crate with Prometheus exporter via existing Axum router (`/metrics`).

## 7. Error Handling & Re-connect
* Any IO error ⇒ close transport, try HID fallback, retry **once**.
* If still failing ⇒ bubble error; cache **not** updated.

## 8. Integration Steps
1. **Create crate `device_queue`** (or module in `src-tauri`).
2. Implement `DeviceCmd`, `DeviceWorker`, `DeviceQueueHandle`.
3. On device-detect event in `DeviceController`, spawn a `DeviceWorker` for that device and store the `DeviceQueueHandle` in `DEVICE_REGISTRY` entry.
4. Refactor existing code: replace direct `create_device_transport()` calls with the queue API.
5. Wire metrics exporter.
6. Remove legacy per-call transport builders after migration.

## 9. Future Extensions
* Support **batch** commands (e.g. fetch multiple XPUBs per connection).
* Persist cache to disk on shutdown for faster cold-start.
* Add tracing spans for richer profiling.

---
*Prepared: 2025-06-11*
