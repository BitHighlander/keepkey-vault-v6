# Development Philosophy & Rules

## 🚩 "Fail Fast" Philosophy

When approaching a large, complex task:

### 1. Quickly identify the component most likely to fail
Ask: "What assumption, technology, or feature, if proven incorrect, invalidates the entire approach?"

**Example**: Before building a complex WebUSB integration:
```bash
# Don't start with the full implementation
# First test: Can we even connect to the device?
echo "Testing basic USB connection..."
node -e "navigator.usb.requestDevice({filters: []}).then(console.log).catch(console.error)"
```

### 2. Test that specific critical part first
Conduct a small, targeted experiment or prototype to verify your assumptions early.

**Example**: Testing device communication before building UI:
```rust
// Don't build the entire queue system first
// Start with: Can we send/receive a single message?
#[test]
fn test_basic_device_communication() {
    let device = connect_device()?;
    let response = device.ping()?;
    assert!(response.is_ok());
}
```

### 3. Fail intentionally and rapidly
If failure occurs, that's valuable—it's an immediate signal to rethink and avoid deeper investment.

**Example**: Testing performance assumptions:
```rust
// Before optimizing for 1000 devices, test with 10
fn stress_test_device_queue() {
    let start = Instant::now();
    handle_10_devices();
    assert!(start.elapsed() < Duration::from_secs(1), "Too slow for 10 devices!");
}
```

### 4. Pivot based on early failures
Quickly reconsider your approach, exploring alternative methods if initial tests fail.

**Example**: 
```markdown
Initial approach: WebUSB for all platforms
Early test failed: Linux permission issues
Pivot: Use native HID for Linux, WebUSB for others
```

## 🚩 "Fail Forward" Philosophy

Build tools and services in a way that are useful later.

### 1. Don't write one-off scripts, write composable tools

**Bad Example**:
```bash
# One-off script that's useless after running
python3 -c "import json; data=json.load(open('devices.json')); print(data['device1']['info'])"
```

**Good Example**:
```rust
// Reusable device inspector tool
#[tauri::command]
pub fn inspect_device(device_id: &str, field: Option<&str>) -> Result<Value> {
    let device = get_device(device_id)?;
    match field {
        Some(f) => device.get_field(f),
        None => Ok(device.to_json())
    }
}
```

### 2. Build CLI tools that compose well

**Example**: Device management CLI
```bash
# Individual tools that work together
keepkey list                    # List all devices
keepkey inspect <id>            # Inspect specific device
keepkey test <id> --ping        # Test device connection
keepkey queue <id> --status     # Check queue status

# Composable
keepkey list | grep "bootloader" | xargs -I {} keepkey update {}
```

### 3. Failed experiments should leave useful artifacts

**Example**: Building a caching system
```rust
// Even if the full cache fails, these remain useful:
pub trait CacheStrategy {
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    fn set(&self, key: &str, value: Vec<u8>);
}

pub struct MemoryCache { ... }
pub struct DiskCache { ... }
pub struct RedisCache { ... }

// If Redis fails, we still have Memory and Disk implementations
```

## 🚩 Make the Requirements Less Dumb

### 1. Question all requirements

**Example**: "We need to support 100 concurrent devices"
- Question: How many devices does a user actually have?
- Reality check: 99% of users have 1-2 devices
- Simplification: Optimize for 5 devices, make 100 possible but not optimized

### 2. Delete the Part or Process

**Example**: Device queue system
```rust
// Original: Complex queue with priorities, scheduling, persistence
// Question: Do we need all this?
// Deleted: Priorities (all requests are equal)
// Deleted: Persistence (requests are short-lived)
// Deleted: Complex scheduling (FIFO is fine)
// Result: 80% less code, 90% of functionality
```

### 3. Simplify and Optimize (in that order)

**Bad Order** (optimize first):
```rust
// Optimized complex system
pub struct OptimizedComplexQueue {
    priority_heap: BinaryHeap<Request>,
    scheduler: Box<dyn Scheduler>,
    persistence: Box<dyn Storage>,
    cache: Arc<Mutex<LruCache>>,
    // 500 lines of optimization
}
```

**Good Order** (simplify first):
```rust
// Step 1: Simplify
pub struct SimpleQueue {
    requests: VecDeque<Request>,
}

// Step 2: Then optimize only if needed
pub struct SimpleQueue {
    requests: VecDeque<Request>,
    batch_size: usize, // Only optimization added after profiling
}
```

## Practical Examples in Our Codebase

### Fail Fast Example: USB Device Detection
```rust
// Before building complex device management:
#[test]
fn fail_fast_usb_detection() {
    // Critical assumption: We can detect devices
    let devices = list_usb_devices();
    assert!(!devices.is_empty(), "FAIL FAST: No USB devices detected!");
}
```

### Fail Forward Example: Command System
```rust
// Instead of one-off command handlers, build reusable service layer
pub struct DeviceService {
    // Useful even if current commands change
}

impl DeviceService {
    pub async fn execute(&self, request: Request) -> Result<Response> {
        // Reusable for CLI, API, or future interfaces
    }
}
```

### Less Dumb Requirements Example: Caching
```rust
// Original requirement: "Cache everything forever"
// Questioned: Why? Most data changes frequently
// Simplified: Cache only immutable data (addresses, xpubs)
// Result: 90% less cache complexity
```

## Rules for This Project

1. **Test critical assumptions first**
   - Can we connect to the device?
   - Can we handle concurrent requests?
   - Does the performance meet requirements?

2. **Build reusable components**
   - Services over scripts
   - Composable tools over monoliths
   - Interfaces over implementations

3. **Question every requirement**
   - "Do we really need this?"
   - "What happens if we don't do this?"
   - "Can we ship without it?"

4. **Delete aggressively**
   - If you're not adding things back, you haven't deleted enough
   - Start with the minimum viable functionality

5. **Simplify before optimizing**
   - Get it working simply first
   - Profile to find actual bottlenecks
   - Optimize only what matters

## Current Application to Backend Refactor

1. **Fail Fast Tests**:
   - Can we compile without the missing modules?
   - Can we separate queue from commands without breaking functionality?
   - Will the service layer pattern work with Tauri?

2. **Fail Forward Approach**:
   - Build service layer that's useful regardless of Tauri
   - Create CLI tools for testing that remain useful
   - Design APIs that work for multiple frontends

3. **Less Dumb Requirements**:
   - Question: "Do we need 53 separate commands?"
   - Delete: Remove duplicate queue/command logic
   - Simplify: One service method, multiple thin wrappers