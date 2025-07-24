# Backend Refactor Plan

## Executive Summary

This plan outlines a systematic approach to refactor the Tauri Rust backend, focusing on:
1. Separating queue and command responsibilities
2. Removing redundant and dead code
3. Improving architecture and maintainability
4. Fixing compilation issues

## Current State Analysis

### Critical Issues
1. **Missing Modules**: `logging`, `cache`, `event_controller` are referenced but not present
2. **Monolithic commands.rs**: 4,384 lines with 53+ command handlers
3. **Circular Dependencies**: Commands ↔ Device modules
4. **Code Duplication**: ~30% duplicate logic between queue and commands
5. **Dead Code**: Entire device module disconnected from main app

### Architecture Problems
```
Current Flow:
Frontend → Tauri Command → commands.rs → device/queue.rs → keepkey_rust
                        ↘                ↗
                         (duplicate logic)
```

## Phase 1: Immediate Fixes (Day 1-2)

### 1.1 Restore Missing Modules
**Priority**: CRITICAL - Code won't compile without these

```bash
# Copy from v5 project
cp -r ../keepkey-vault-v5/projects/keepkey-vault/src-tauri/src/logging.rs src-tauri/src/
cp -r ../keepkey-vault-v5/projects/keepkey-vault/src-tauri/src/event_controller.rs src-tauri/src/
cp -r ../keepkey-vault-v5/projects/keepkey-vault/src-tauri/src/cache src-tauri/src/
```

**Modifications needed**:
- Remove v5-specific dependencies
- Update imports to match v6 structure
- Create minimal implementations if full modules are too complex

### 1.2 Fix Module Declarations
```rust
// In lib.rs, add:
mod commands;
mod device;
mod logging;
mod cache;
mod event_controller;
```

### 1.3 Remove Dead Device Module
Since the device module is not connected:
- Either connect it properly OR
- Remove it entirely and use keepkey_rust directly

## Phase 2: Queue/Command Separation (Day 3-7)

### 2.1 New Architecture
```
Proposed Flow:
Frontend → Tauri Command → Service Layer → keepkey_rust
              ↓                 ↓
         (thin wrapper)    (business logic)
```

### 2.2 Directory Structure
```
src/
├── commands/           # Thin Tauri command wrappers
│   ├── mod.rs
│   ├── device.rs      # Device management commands
│   ├── wallet.rs      # Wallet operations
│   ├── recovery.rs    # Recovery operations
│   ├── system.rs      # System utilities
│   └── preferences.rs # App preferences
├── services/          # Business logic layer
│   ├── mod.rs
│   ├── device_service.rs
│   ├── wallet_service.rs
│   ├── recovery_service.rs
│   └── cache_service.rs
├── state/            # Shared application state
│   ├── mod.rs
│   ├── device_state.rs
│   └── app_state.rs
├── utils/            # Shared utilities
│   ├── mod.rs
│   ├── parsing.rs    # Derivation paths, transactions
│   └── conversions.rs # Feature conversions
└── errors.rs         # Unified error handling
```

### 2.3 Refactoring Steps

#### Step 1: Extract Utilities
Move shared functions from commands.rs:
- `parse_derivation_path()` → `utils/parsing.rs`
- `parse_transaction_from_hex()` → `utils/parsing.rs`
- Feature conversion functions → `utils/conversions.rs`

#### Step 2: Create Service Layer
Example transformation:
```rust
// OLD: In commands.rs
#[tauri::command]
pub async fn get_device_info(
    device_id: String,
    state: State<'_, DeviceQueueManager>,
) -> Result<DeviceInfo, String> {
    // 50+ lines of business logic
}

// NEW: In commands/device.rs
#[tauri::command]
pub async fn get_device_info(
    device_id: String,
    device_service: State<'_, DeviceService>,
) -> Result<DeviceInfo, String> {
    device_service.get_info(&device_id).await
}

// NEW: In services/device_service.rs
impl DeviceService {
    pub async fn get_info(&self, device_id: &str) -> Result<DeviceInfo, Error> {
        // Business logic here
    }
}
```

#### Step 3: Consolidate Queue Operations
- Remove `device/queue.rs` entirely
- Move queue functionality to `services/device_service.rs`
- Use keepkey_rust's DeviceQueue directly

## Phase 3: Remove Redundant Code (Day 8-10)

### 3.1 Identified Redundancies

| Redundancy | Location | Action |
|------------|----------|---------|
| Queue management | commands.rs + device/queue.rs | Consolidate in service |
| PIN flow handling | Multiple locations | Single implementation in wallet_service |
| Device status evaluation | Duplicated 3x | Extract to device_state |
| Response caching | commands + queue | Use cache service |
| Error conversions | Throughout | Unified error type |

### 3.2 Code to Remove
- [ ] `device/queue.rs` - Replace with service layer
- [ ] Duplicate utility functions
- [ ] Redundant state management
- [ ] Unused imports and dead code
- [ ] Command forwarding boilerplate

### 3.3 Consolidation Examples

**Before**: 3 different PIN flow implementations
```rust
// In commands.rs
fn handle_pin_flow_commands() { /* implementation 1 */ }

// In device/queue.rs  
fn manage_pin_flow() { /* implementation 2 */ }

// In device/system_operations.rs
fn pin_operations() { /* implementation 3 */ }
```

**After**: Single implementation
```rust
// In services/wallet_service.rs
impl WalletService {
    pub async fn handle_pin_flow(&self, ...) { /* single implementation */ }
}
```

## Phase 4: Clean Architecture (Day 11-14)

### 4.1 Dependency Injection
```rust
// App setup in lib.rs
.setup(|app| {
    let device_service = DeviceService::new();
    let wallet_service = WalletService::new();
    let cache_service = CacheService::new();
    
    app.manage(device_service);
    app.manage(wallet_service);
    app.manage(cache_service);
    
    Ok(())
})
```

### 4.2 Error Handling
```rust
// errors.rs
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Device error: {0}")]
    Device(#[from] DeviceError),
    
    #[error("Wallet error: {0}")]
    Wallet(#[from] WalletError),
    
    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),
}

// Automatic conversion in commands
#[tauri::command]
pub async fn some_command() -> Result<String, AppError> {
    // Errors automatically converted
}
```

### 4.3 Testing Strategy
```
src/
├── services/
│   ├── device_service.rs
│   └── device_service_test.rs  # Unit tests
└── tests/
    └── integration/            # Integration tests
```

## Implementation Timeline

| Phase | Duration | Deliverables |
|-------|----------|-------------|
| Phase 1 | 2 days | Compilable code with missing modules |
| Phase 2 | 5 days | Separated queue/commands |
| Phase 3 | 3 days | No redundant code |
| Phase 4 | 4 days | Clean architecture |

Total: 14 days

## Success Metrics

1. **Code Quality**
   - No file > 500 lines
   - No function > 50 lines
   - Zero circular dependencies
   - < 5% code duplication

2. **Architecture**
   - Clear separation of concerns
   - Testable service layer
   - Unified error handling
   - Consistent patterns

3. **Performance**
   - No regression in response times
   - Reduced memory usage
   - Better error recovery

## Migration Strategy

1. **Branch Strategy**
   - Work on `backend-refactor` branch
   - Daily commits with clear messages
   - PR reviews for each phase

2. **Testing Protocol**
   - Manual testing after each phase
   - Regression testing for critical paths
   - Performance benchmarking

3. **Rollback Plan**
   - Keep original code available
   - Feature flags for gradual migration
   - Ability to switch implementations

## Next Steps

1. **Immediate Action**: Copy missing modules from v5
2. **Quick Win**: Extract utilities to shared module
3. **Focus Area**: Start with device commands (most complex)
4. **Validation**: Ensure each refactor maintains functionality

## Notes

- The device module appears to be from an older version - evaluate if needed
- Consider using workspace dependencies for shared code
- Plan for frontend changes if API contracts change
- Document new architecture for team