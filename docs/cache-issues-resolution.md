# Cache Issues Resolution

## Problem Summary
The KeepKey vault had persistent database caching issues causing:
- **"No device found in cache"** errors despite data existing in database
- **Repeated frontload loops** ("Missing 24 out of 24 required addresses")
- **Startup failures** with race conditions between frontload completion and API calls
- **USD values showing $0.00** due to stale portfolio summary cache

## Root Causes Identified

### 1. Memory Cache vs Database Mismatch
**Issue**: `get_device_id()` only checked memory cache, ignoring database
```rust
// ❌ BEFORE: Only checked memory, returned None even when DB had data
pub fn get_device_id(&self) -> Option<String> {
    let cache = self.memory_cache.read().unwrap();
    cache.device_id.clone()  // Always None after restart!
}
```

**Impact**: API endpoints failed with "No device found in cache" immediately after frontload

### 2. Portfolio Summary Cache Not Invalidated
**Issue**: `save_balances()` updated individual balances but didn't clear portfolio summary cache
```rust
// ❌ BEFORE: Missing portfolio summary cache invalidation
// Balances: ✅ $302.86
// Summary:  ❌ $0.00 (stale)
```

### 3. Race Condition in Device Ready Signal
**Issue**: "Device ready" sent before memory cache was populated
- ✅ Frontload completes successfully
- ✅ Device context set in server
- 🚀 "Device ready" signal sent
- ❌ Frontend calls API → memory cache empty → "No device found"

### 4. Excessive Startup Time
**Issue**: Cache timeout was 10 minutes instead of 1 hour
- System was re-fetching balances every 10 minutes
- Should be 1 hour for "instant startup"

## Comprehensive Fixes Implemented

### ✅ Fix 1: Database Fallback for get_device_id()
**Files**: `device_cache.rs` (both vault and kkcli)
```rust
// ✅ AFTER: Database fallback when memory cache empty
pub fn get_device_id(&self) -> Option<String> {
    // Try memory cache first (fast)
    {
        let cache = self.memory_cache.read().unwrap();
        if let Some(device_id) = &cache.device_id {
            return Some(device_id.clone());
        }
    }
    
    // Fallback to database (reliable)
    match self.get_first_device_from_db() {
        Ok(Some(device_id)) => {
            info!("💾 Using database fallback for device ID: {}", device_id);
            
            // Populate memory cache for next time
            {
                let mut cache = self.memory_cache.write().unwrap();
                cache.device_id = Some(device_id.clone());
            }
            
            Some(device_id)
        },
        _ => None
    }
}
```

### ✅ Fix 2: Portfolio Summary Cache Invalidation
**Files**: `device_cache.rs` (both vault and kkcli)
```rust
// ✅ AFTER: Clear portfolio summary when balances update
pub async fn save_balances(&self, device_id: &str, balances: &[Balance]) -> Result<()> {
    // ... save balances ...
    
    // 🚨 FIXED: Clear portfolio summary cache when balances update
    db.execute(
        "DELETE FROM portfolio_summaries WHERE device_id = ?1",
        params![device_id],
    )?;
    
    info!("💾 Saved {} balances and cleared stale portfolio summary", balances.len());
    Ok(())
}
```

### ✅ Fix 3: Extended Cache Timeout to 1 Hour
**Files**: `device_cache.rs` (both vault and kkcli)
```rust
// ✅ AFTER: 1 hour cache timeout for instant startup
pub async fn balances_need_refresh(&self, device_id: &str) -> Result<bool> {
    let one_hour_ago = chrono::Utc::now().timestamp() - 3600; // 1 hour (was 600 = 10 min)
    
    let count: i64 = db.query_row(
        "SELECT COUNT(*) FROM cached_balances 
         WHERE device_id = ?1 AND last_updated > ?2",
        params![device_id, one_hour_ago],
        |row| row.get(0),
    )?;
    
    let needs_refresh = count == 0;
    info!("💾 Cache check: {} fresh balances (< 1h old) → need refresh: {}", 
        count, needs_refresh);
    Ok(needs_refresh)
}
```

### ✅ Fix 4: Respect Cache Logic in Frontload
**Files**: `frontload.rs` (both vault and kkcli)
```rust
// ✅ AFTER: Actually use cache result instead of always refreshing
let needs_refresh = match self.cache.balances_need_refresh(device_id).await {
    Ok(needs) => {
        info!("{}: Balances need refresh: {}", tag, needs);
        needs // ✅ FIXED: Actually use the cache result
    }
    Err(e) => {
        warn!("{}: Error checking refresh status, forcing refresh: {}", tag, e);
        true
    }
};

if needs_refresh {
    info!("{}: Refreshing balances from Pioneer API...", tag);
    // ... refresh logic ...
} else {
    info!("{}: ⚡ Balances are fresh (< 1h old) - skipping Pioneer API call for instant startup!", tag);
}
```

### ✅ Fix 5: Database Debug Endpoint
**Files**: `v2_endpoints.rs`, `device_cache.rs`
```rust
// ✅ NEW: Debug endpoint to inspect raw database contents
GET /api/v2/debug/database

// Returns:
{
  "devices": [...],           // All devices in database
  "cached_addresses": [...],  // All cached addresses (limit 100)
  "cached_balances": [...],   // All cached balances  
  "portfolio_summaries": [...], // All portfolio summaries
  "memory_cache_device_id": "...", // Current memory cache state
  "summary": {
    "devices_count": 1,
    "addresses_count": 40,
    "balances_count": 3,
    "summaries_count": 0
  }
}
```

## Testing Tools Created

### 1. Cache Mismatch Debug Tool
**File**: `tests/debug-cache-mismatch.js`
- Traces memory cache vs database state
- Tests API endpoints after sync
- Provides fix suggestions

### 2. Startup Optimization Test
**File**: `tests/test-startup-optimization.js`  
- Verifies 1-hour cache timeout
- Tests instant startup behavior
- Validates cache respect logic

### 3. USD Value Fix Test
**File**: `tests/test-usd-fix.js`
- Verifies portfolio summary cache invalidation
- Tests balance vs summary consistency

## Expected Behavior After Fixes

### 🚀 First Run (Cold Start)
1. Device detected → frontload triggered
2. Missing addresses populated (up to 60s)
3. Balances fetched from Pioneer API
4. Device ready signal sent
5. Frontend loads instantly with data

### ⚡ Subsequent Runs (Warm Start)
1. Device detected → frontload triggered  
2. Cache check: "Balances are fresh (< 1h old)"
3. Skip Pioneer API call
4. Device ready signal sent immediately
5. **Instant startup** (< 3 seconds)

### 💰 USD Values
1. Individual balances: ✅ Correct USD values
2. Portfolio summary: ✅ Correct total USD value
3. Cache invalidation: ✅ Summary updates when balances change

### 🔧 No More "No Device Found" Errors
1. API endpoints use database fallback
2. Memory cache populated automatically
3. Race conditions eliminated

## Validation Commands

```bash
# Test cache debug tools
chmod +x tests/debug-cache-mismatch.js
node tests/debug-cache-mismatch.js

# Test startup optimization  
chmod +x tests/test-startup-optimization.js
node tests/test-startup-optimization.js

# Test USD value fix
chmod +x tests/test-usd-fix.js
node tests/test-usd-fix.js

# Inspect raw database
curl http://localhost:1646/api/v2/debug/database | jq .summary
```

## Key Architecture Improvements

1. **Database as Source of Truth**: API endpoints no longer require memory cache
2. **Automatic Cache Recovery**: Memory cache auto-populates from database
3. **Proper Cache Invalidation**: Portfolio summaries cleared when balances update
4. **Instant Startup**: 1-hour cache timeout prevents excessive API calls
5. **Comprehensive Debugging**: New tools to trace cache issues

This resolves the fundamental "simple database persistence" problem that was causing startup failures and data inconsistencies. 