# Transaction Build API Planning Document

## Overview
The **`/api/v2/tx/build`** endpoint creates unsigned Bitcoin transactions with comprehensive input selection, fee validation, and safety checks. This endpoint replaces missing functionality and implements server-side protections against excessive fees and malicious requests.

---
## 1. API Endpoint Design

### 1.1 Request Schema
```rust
#[derive(Debug, Deserialize, ToSchema)]
pub struct BuildTxRequest {
    pub device_id: String,
    pub recipients: Vec<TxOutput>,
    pub fee_rate: f64,                    // sat/vByte
    pub input_selection: InputSelection,
    pub max_fee_btc: Option<f64>,        // Optional override (default: 0.1 BTC max)
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TxOutput {
    pub address: String,
    pub amount: String,                  // BTC amount as string (e.g., "0.00100000")
}

#[derive(Debug, Deserialize, ToSchema)]
pub enum InputSelection {
    Auto { percent: u8 },               // 0-100% of available UTXOs
    Manual { utxos: Vec<String> },      // Specific txid:vout identifiers
    Max,                                // Send maximum (all UTXOs minus fee)
}
```

### 1.2 Response Schema
```rust
#[derive(Debug, Serialize, ToSchema)]
pub struct BuildTxResponse {
    pub success: bool,
    pub tx: Option<UnsignedTx>,
    pub error: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UnsignedTx {
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub fee_sats: u64,
    pub fee_btc: String,
    pub fee_usd: String,
    pub size_bytes: u32,
    pub fee_rate: f64,                  // Actual sat/vByte achieved
    pub total_input_sats: u64,
    pub total_output_sats: u64,
    pub change_output: Option<ChangeOutput>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TxInput {
    pub txid: String,
    pub vout: u32,
    pub amount_sats: u64,
    pub script_type: String,
    pub address_n_list: Vec<u32>,
    pub confirmations: u32,
    pub address: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChangeOutput {
    pub address: String,
    pub amount_sats: u64,
    pub address_n_list: Vec<u32>,
    pub script_type: String,
}
```

---
## 2. Safety Requirements & Validation

### 2.1 Fee Protection (CRITICAL)
```rust
const MAX_FEE_BTC: f64 = 0.1; // 0.1 BTC absolute maximum
const MAX_FEE_PERCENT: f64 = 50.0; // 50% of transaction value maximum

fn validate_fee_safety(fee_sats: u64, total_output_sats: u64, max_fee_override: Option<f64>) -> Result<(), String> {
    let fee_btc = fee_sats as f64 / 100_000_000.0;
    let max_allowed = max_fee_override.unwrap_or(MAX_FEE_BTC);
    
    // 🚨 HARD LIMIT: Never allow fees > 0.1 BTC (or override limit)
    if fee_btc > max_allowed {
        return Err(format!(
            "Fee of {} BTC exceeds maximum allowed limit of {} BTC. This appears to be an error.",
            fee_btc, max_allowed
        ));
    }
    
    // 🚨 PERCENTAGE CHECK: Fee shouldn't exceed 50% of transaction value
    if total_output_sats > 0 {
        let fee_percentage = (fee_sats as f64 / total_output_sats as f64) * 100.0;
        if fee_percentage > MAX_FEE_PERCENT {
            return Err(format!(
                "Fee of {:.1}% is excessive. Maximum allowed is {:.1}% of transaction value.",
                fee_percentage, MAX_FEE_PERCENT
            ));
        }
    }
    
    // 🚨 DUST PROTECTION: Minimum fee rate 1 sat/vByte
    // (This will be checked during coin selection)
    
    Ok(())
}
```

### 2.2 Input Validation
```rust
fn validate_build_request(req: &BuildTxRequest) -> Result<(), String> {
    // Device ID validation
    if req.device_id.trim().is_empty() {
        return Err("Device ID is required".to_string());
    }
    
    // Recipients validation
    if req.recipients.is_empty() {
        return Err("At least one recipient is required".to_string());
    }
    
    for (i, recipient) in req.recipients.iter().enumerate() {
        // Bitcoin address validation
        if !is_valid_bitcoin_address(&recipient.address) {
            return Err(format!("Invalid Bitcoin address at recipient {}", i + 1));
        }
        
        // Amount validation
        let amount = parse_btc_amount(&recipient.amount)?;
        if amount <= 0.0 {
            return Err(format!("Invalid amount at recipient {}: must be > 0", i + 1));
        }
        
        // Dust limit check (546 sats for P2PKH)
        let amount_sats = (amount * 100_000_000.0) as u64;
        if amount_sats < 546 {
            return Err(format!("Amount too small at recipient {}: below dust limit", i + 1));
        }
    }
    
    // Fee rate validation
    if req.fee_rate < 1.0 {
        return Err("Fee rate must be at least 1 sat/vByte".to_string());
    }
    if req.fee_rate > 1000.0 {
        return Err("Fee rate of {} sat/vByte is unreasonably high".to_string());
    }
    
    Ok(())
}
```

---
## 3. UTXO Selection Algorithms

### 3.1 Auto Selection (Percentage-based)
```rust
fn select_utxos_auto(utxos: &[UTXO], percent: u8) -> Vec<UTXO> {
    if percent == 0 || utxos.is_empty() {
        return vec![];
    }
    
    // Sort UTXOs by ascending age/value (as per planning doc)
    let mut sorted_utxos = utxos.to_vec();
    sorted_utxos.sort_by(|a, b| {
        // Primary: More confirmations first (older/safer)
        match b.confirmations.cmp(&a.confirmations) {
            std::cmp::Ordering::Equal => {
                // Secondary: Smaller amounts first (privacy/efficiency)
                a.amount_sats.cmp(&b.amount_sats)
            }
            other => other,
        }
    });
    
    // Select percentage of UTXOs
    let count = ((sorted_utxos.len() as f64 * percent as f64) / 100.0).ceil() as usize;
    let count = count.min(sorted_utxos.len()).max(1); // At least 1, at most all
    
    sorted_utxos.into_iter().take(count).collect()
}
```

### 3.2 Manual Selection
```rust
fn select_utxos_manual(utxos: &[UTXO], selected_outpoints: &[String]) -> Result<Vec<UTXO>, String> {
    let mut selected = Vec::new();
    
    for outpoint in selected_outpoints {
        let parts: Vec<&str> = outpoint.split(':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid outpoint format: {}", outpoint));
        }
        
        let txid = parts[0];
        let vout: u32 = parts[1].parse()
            .map_err(|_| format!("Invalid vout in outpoint: {}", outpoint))?;
        
        match utxos.iter().find(|u| u.txid == txid && u.vout == vout) {
            Some(utxo) => selected.push(utxo.clone()),
            None => return Err(format!("UTXO not found: {}", outpoint)),
        }
    }
    
    if selected.is_empty() {
        return Err("No valid UTXOs selected".to_string());
    }
    
    Ok(selected)
}
```

### 3.3 Max Send Selection
```rust
fn select_utxos_max(utxos: &[UTXO]) -> Vec<UTXO> {
    // For max send, use all available UTXOs
    // Coin selection will determine optimal subset
    utxos.to_vec()
}
```

---
## 4. Coin Selection Implementation

### 4.1 Using coinselect Library (Rust equivalent)
```rust
use bitcoin::util::amount::Amount;

fn build_transaction(
    available_utxos: &[UTXO],
    recipients: &[TxOutput],
    fee_rate: f64,
    is_max_send: bool,
) -> Result<CoinSelectionResult, String> {
    
    // Convert inputs to coinselect format
    let inputs: Vec<_> = available_utxos.iter().map(|utxo| {
        CoinSelectInput {
            txid: utxo.txid.clone(),
            vout: utxo.vout,
            value: utxo.amount_sats,
            script_type: utxo.script_type.clone(),
        }
    }).collect();
    
    // Convert outputs to coinselect format
    let outputs: Vec<_> = recipients.iter().map(|recipient| {
        let amount_sats = (parse_btc_amount(&recipient.amount).unwrap() * 100_000_000.0) as u64;
        CoinSelectOutput {
            address: recipient.address.clone(),
            value: amount_sats,
        }
    }).collect();
    
    // Perform coin selection
    let result = if is_max_send {
        coinselect_split(&inputs, &outputs, fee_rate)?
    } else {
        coinselect(&inputs, &outputs, fee_rate)?
    };
    
    // Validate result
    if result.inputs.is_empty() {
        return Err("No suitable UTXOs found for transaction".to_string());
    }
    
    if result.outputs.is_empty() {
        return Err("Failed to create valid outputs".to_string());
    }
    
    // Validate fee
    validate_fee_safety(result.fee, result.total_output_value, None)?;
    
    Ok(result)
}
```

---
## 5. Change Address Management

### 5.1 Change Address Generation
```rust
async fn get_change_address(
    device_cache: &DeviceCache,
    device_id: &str,
    script_type: &str,
) -> Result<ChangeAddress, String> {
    
    // Get next change address index from cache/database
    let change_index = device_cache.get_next_change_index(device_id, script_type).await?;
    
    // Derive change address path
    let (purpose, coin_type) = match script_type {
        "p2pkh" => (44, 0),      // m/44'/0'/0'/1/index
        "p2sh-p2wpkh" => (49, 0), // m/49'/0'/0'/1/index  
        "p2wpkh" => (84, 0),     // m/84'/0'/0'/1/index
        _ => return Err(format!("Unsupported script type: {}", script_type)),
    };
    
    let derivation_path = vec![
        purpose + 0x80000000,    // Purpose (hardened)
        coin_type + 0x80000000,  // Coin type (hardened)
        0x80000000,              // Account 0 (hardened)
        1,                       // Change = 1
        change_index,            // Address index
    ];
    
    // Get change address from cache or generate
    let change_address = match device_cache.get_cached_address("Bitcoin", script_type, &derivation_path) {
        Some(cached) => cached.address,
        None => {
            // Generate new change address via device
            generate_change_address_on_device(device_id, &derivation_path, script_type).await?
        }
    };
    
    Ok(ChangeAddress {
        address: change_address,
        derivation_path,
        script_type: script_type.to_string(),
        index: change_index,
    })
}
```

---
## 6. Security & Privacy Considerations

### 6.1 Privacy Warnings
```rust
fn generate_privacy_warnings(inputs: &[TxInput], outputs: &[TxOutput]) -> Vec<String> {
    let mut warnings = Vec::new();
    
    // Check for mixed script types
    let script_types: HashSet<_> = inputs.iter().map(|i| &i.script_type).collect();
    if script_types.len() > 1 {
        warnings.push("⚠️ Mixing script types may reduce privacy".to_string());
    }
    
    // Check for low confirmations
    let low_conf_count = inputs.iter().filter(|i| i.confirmations < 3).count();
    if low_conf_count > 0 {
        warnings.push(format!("⚠️ {} input(s) have low confirmations", low_conf_count));
    }
    
    // Check for round number detection
    for output in outputs {
        let amount_sats = parse_btc_amount(&output.amount).unwrap() * 100_000_000.0;
        if is_round_number(amount_sats as u64) {
            warnings.push("⚠️ Round number amounts may reduce privacy".to_string());
            break;
        }
    }
    
    warnings
}

fn is_round_number(sats: u64) -> bool {
    // Check for common round numbers that stand out
    let btc = sats as f64 / 100_000_000.0;
    btc.fract() == 0.0 || // Whole BTC amounts
    sats % 1_000_000 == 0 || // Whole mBTC amounts
    sats % 100_000 == 0 // 0.001 BTC increments
}
```

### 6.2 Rate Limiting & Abuse Prevention
```rust
#[derive(Debug)]
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

impl RateLimiter {
    const MAX_REQUESTS_PER_MINUTE: usize = 10;
    const MAX_HIGH_FEE_REQUESTS_PER_HOUR: usize = 3;
    
    pub fn check_rate_limit(&self, device_id: &str, fee_btc: f64) -> Result<(), String> {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();
        let device_requests = requests.entry(device_id.to_string()).or_insert(Vec::new());
        
        // Clean old requests (older than 1 hour)
        device_requests.retain(|&time| now.duration_since(time).as_secs() < 3600);
        
        // Check general rate limit (10 requests per minute)
        let recent_requests = device_requests.iter()
            .filter(|&&time| now.duration_since(time).as_secs() < 60)
            .count();
            
        if recent_requests >= Self::MAX_REQUESTS_PER_MINUTE {
            return Err("Rate limit exceeded: too many build requests".to_string());
        }
        
        // Check high-fee request limit (if fee > 0.01 BTC)
        if fee_btc > 0.01 {
            let high_fee_requests = device_requests.iter()
                .filter(|&&time| now.duration_since(time).as_secs() < 3600)
                .count();
                
            if high_fee_requests >= Self::MAX_HIGH_FEE_REQUESTS_PER_HOUR {
                return Err("Rate limit exceeded: too many high-fee requests".to_string());
            }
        }
        
        device_requests.push(now);
        Ok(())
    }
}
```

---
## 7. Error Handling & User Feedback

### 7.1 Error Categories
```rust
#[derive(Debug, thiserror::Error)]
pub enum BuildTxError {
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Insufficient funds: need {needed} BTC, have {available} BTC")]
    InsufficientFunds { needed: f64, available: f64 },
    
    #[error("Fee too high: {fee} BTC exceeds limit of {limit} BTC")]
    FeeTooHigh { fee: f64, limit: f64 },
    
    #[error("No suitable UTXOs found for transaction")]
    NoSuitableUtxos,
    
    #[error("Device error: {0}")]
    Device(String),
    
    #[error("Network error: {0}")]
    Network(String),
}
```

### 7.2 User-Friendly Error Messages
```rust
fn format_user_error(error: &BuildTxError) -> String {
    match error {
        BuildTxError::FeeTooHigh { fee, limit } => {
            format!(
                "Transaction fee of {} BTC seems unusually high. Maximum allowed is {} BTC. \
                Please check your fee rate setting.",
                fee, limit
            )
        }
        BuildTxError::InsufficientFunds { needed, available } => {
            format!(
                "Insufficient funds. You need {} BTC but only have {} BTC available. \
                Try reducing the amount or selecting more UTXOs.",
                needed, available
            )
        }
        BuildTxError::NoSuitableUtxos => {
            "No suitable UTXOs found. Try selecting more UTXOs or reducing the amount.".to_string()
        }
        _ => error.to_string(),
    }
}
```

---
## 8. Implementation Phases

### Phase 1: Core Functionality
- [x] Basic endpoint structure (`/api/v2/tx/build`)
- [x] Input validation and safety checks
- [x] Auto UTXO selection (percentage-based)
- [x] Fee validation (0.1 BTC hard limit)
- [x] Change address generation

### Phase 2: Advanced Features  
- [ ] Manual UTXO selection
- [ ] Custom fee rate validation
- [ ] Privacy warnings system
- [ ] Rate limiting implementation

### Phase 3: Production Hardening
- [ ] Comprehensive error handling
- [ ] Audit logging for high-fee transactions
- [ ] Performance optimization
- [ ] Integration tests

---
## 9. Testing Strategy

### 9.1 Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fee_validation_excessive_fee() {
        let result = validate_fee_safety(
            15_000_000, // 0.15 BTC fee
            1_000_000,  // 0.01 BTC output
            None
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds maximum allowed limit"));
    }
    
    #[test]
    fn test_utxo_selection_percentage() {
        let utxos = create_test_utxos(10);
        let selected = select_utxos_auto(&utxos, 50);
        assert_eq!(selected.len(), 5);
    }
    
    #[test]
    fn test_dust_limit_validation() {
        let request = BuildTxRequest {
            recipients: vec![TxOutput {
                address: "bc1qtest".to_string(),
                amount: "0.00000100".to_string(), // Below dust limit
            }],
            // ... other fields
        };
        assert!(validate_build_request(&request).is_err());
    }
}
```

### 9.2 Integration Tests
- Test with real Pioneer API for fee rates
- Test with actual device XPUB data
- Test edge cases (dust amounts, high fees, insufficient funds)
- Test privacy warning generation

---
*Prepared: 2025-01-09*  
*Safety Priority: FAIL FAST on excessive fees, protect users from accidents* 