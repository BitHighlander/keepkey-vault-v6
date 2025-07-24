# Vault `/v2` Axum Architecture Overview

## Purpose
This document describes the architecture for integrating real, cached portfolio endpoints (`/v2/networks`, `/v2/balances`, `/v2/portfolio/summary`) into the Vault backend, using Axum, SQLite, and a unified state pattern. It explains how the Vault backend and keepkey-rest crate interoperate, and how the system is designed for extensibility and performance.

---

## Key Components

### 1. **AppState (Vault)**
- Central Axum state struct: `AppState` holds both the Vault's `ServerState` (for device management, etc.) and the SQLite connection (wrapped in an `Arc<tokio::sync::Mutex<_>>`).
- All Axum routers and handlers use `Arc<AppState>` as their state type.

```rust
pub struct AppState {
    pub server_state: Arc<ServerState>,
    pub indexdb: Arc<tokio::sync::Mutex<rusqlite::Connection>>,
}
```

### 2. **Trait-Based State Extraction (keepkey-rest)**
- The keepkey-rest crate defines a trait `HasDbConnection`:
  ```rust
  pub trait HasDbConnection: Send + Sync + 'static {
      fn get_db_connection(&self) -> Arc<Mutex<Connection>>;
  }
  ```
- `AppState` implements this trait, allowing keepkey-rest's generic handlers to extract the SQLite connection from any compatible state.

### 3. **Generic v2 Router and Handlers (keepkey-rest)**
- All `/v2` handlers and the router are generic over a state type `S_Actual: HasDbConnection`.
- The router is constructed as `Router<Arc<S_Actual>>`, so in Vault, it's `Router<Arc<AppState>>`.
- Example handler signature:
  ```rust
  pub async fn get_networks<S_Actual: HasDbConnection>(State(state_arc): State<Arc<S_Actual>>) -> Json<Vec<Network>>
  ```

### 4. **Router Integration (Vault)**
- The Vault backend constructs its main Axum router with `Arc<AppState>` as state.
- The `/v2` endpoints are nested using `.nest("/v2", v2_integration::vault_v2_router(app_state.clone()))`, where `vault_v2_router` returns a `Router<Arc<AppState>>`.
- This allows `/v2` endpoints to access both the SQLite connection (for cached data) and any future Vault state.

---

## Benefits
- **Type Safety:** All handlers and routers are statically checked for state compatibility.
- **Extensibility:** Any future Vault state (e.g., device session, config) can be added to `AppState` and made available to all endpoints.
- **Performance:** SQLite access is performed with `tokio::task::spawn_blocking` and a mutex, ensuring async runtime safety.
- **Separation of Concerns:** keepkey-rest remains agnostic to Vault's internal state, relying only on the `HasDbConnection` trait.
- **OpenAPI/Swagger:** All `/v2` endpoints are fully documented and visible in the Vault Swagger UI.

---

## Example Flow
1. Vault starts and constructs `AppState` with device manager and SQLite connection.
2. The main Axum router is built with `Arc<AppState>` as state.
3. `/v2` endpoints are nested and receive `Arc<AppState>` as state.
4. keepkey-rest handlers extract the SQLite connection via `HasDbConnection` and serve real cached data for the UI.

---

## Extending This Pattern
- To add new shared resources, add fields to `AppState` and update the trait(s) as needed.
- To add new `/v2` endpoints, implement additional handlers in keepkey-rest using the same trait-based pattern.
- For live device data, extend `AppState` and the trait to expose device sessions or other state.

---

## File Locations
- `vault/src-tauri/src/server/mod.rs` - Defines `AppState`, router, and main server logic.
- `vault/src-tauri/src/server/v2_integration.rs` - Wraps keepkey-rest's v2 router for Vault.
- `keepkey-rest/src/routes/v2.rs` - Defines `/v2` handlers, trait, and generic router.

---

## Summary
This architecture enables a robust, extensible, and type-safe integration of real portfolio data endpoints into the Vault backend, leveraging Rust's trait system and Axum's generic routers for clean separation and future growth.
