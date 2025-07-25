use serde::{Deserialize, Serialize};

// ========== Device Registry Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRecord {
    pub device_id: String,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub label: Option<String>,
    pub firmware_variant: Option<String>,
    pub firmware_version: Option<String>,
    pub bootloader_mode: bool,
    pub initialized: bool,
    pub pin_protection: bool,
    pub passphrase_protection: bool,
    pub first_seen: i64,
    pub last_seen: i64,
    pub features: Option<String>,
    
    // Setup tracking fields
    pub serial_number: Option<String>,
    pub setup_complete: bool,
    pub setup_step_completed: u8,
    pub eth_address: Option<String>,
    pub setup_started_at: Option<i64>,
    pub setup_completed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConnection {
    pub id: i64,
    pub device_id: String,
    pub connected_at: i64,
    pub disconnected_at: Option<i64>,
    pub session_data: Option<String>,
}

// ========== Portfolio Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioBalance {
    pub id: i64,
    pub device_id: String,
    pub pubkey: String,
    pub caip: String,
    pub network_id: String,
    pub ticker: String,
    pub address: Option<String>,
    pub balance: String,
    pub balance_usd: String,
    pub price_usd: String,
    pub balance_type: String, // 'balance', 'staking', 'delegation', etc.
    pub name: Option<String>,
    pub icon: Option<String>,
    pub precision: Option<i32>,
    pub contract: Option<String>,
    pub validator: Option<String>,
    pub unbonding_end: Option<i64>,
    pub rewards_available: Option<String>,
    pub last_updated: i64,
    pub last_block_height: Option<i64>,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioBalanceInput {
    pub device_id: String,
    pub pubkey: String,
    pub caip: String,
    pub network_id: String,
    pub ticker: String,
    pub address: Option<String>,
    pub balance: String,
    pub balance_usd: String,
    pub price_usd: String,
    pub balance_type: String,
    pub name: Option<String>,
    pub icon: Option<String>,
    pub precision: Option<i32>,
    pub contract: Option<String>,
    pub validator: Option<String>,
    pub unbonding_end: Option<i64>,
    pub rewards_available: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioDashboard {
    pub id: i64,
    pub device_id: String,
    pub total_value_usd: String,
    pub networks_json: String,
    pub assets_json: String,
    pub total_assets: i32,
    pub total_networks: i32,
    pub last_24h_change_usd: Option<String>,
    pub last_24h_change_percent: Option<String>,
    pub is_combined: bool,
    pub included_devices: Option<String>,
    pub last_updated: i64,
}

// ========== Asset Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: i64,
    pub caip: String,
    pub network_id: String,
    pub chain_id: Option<String>,
    pub symbol: String,
    pub name: String,
    pub asset_type: Option<String>,
    pub is_native: bool,
    pub contract_address: Option<String>,
    pub token_id: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub decimals: Option<i32>,
    pub precision: Option<i32>,
    pub network_name: Option<String>,
    pub native_asset_caip: Option<String>,
    pub explorer: Option<String>,
    pub explorer_address_link: Option<String>,
    pub explorer_tx_link: Option<String>,
    pub coin_gecko_id: Option<String>,
    pub chain_reference: Option<String>,
    pub tags: Option<String>,
    pub source: String,
    pub is_verified: bool,
    pub created_at: i64,
    pub last_updated: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    pub id: i64,
    pub network_id: String,
    pub name: String,
    pub short_name: Option<String>,
    pub chain_id: Option<String>,
    pub network_type: Option<String>,
    pub native_asset_caip: String,
    pub native_symbol: String,
    pub rpc_urls: Option<String>,
    pub ws_urls: Option<String>,
    pub explorer_url: Option<String>,
    pub explorer_api_url: Option<String>,
    pub explorer_api_key_required: bool,
    pub supports_eip1559: bool,
    pub supports_memo: bool,
    pub supports_tokens: bool,
    pub fee_asset_caip: Option<String>,
    pub min_fee: Option<String>,
    pub tags: Option<String>,
    pub is_testnet: bool,
    pub is_active: bool,
    pub created_at: i64,
    pub last_updated: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivationPath {
    pub id: i64,
    pub path_id: String,
    pub note: Option<String>,
    pub blockchain: String,
    pub symbol: String,
    pub networks: String, // JSON array
    pub script_type: Option<String>,
    pub address_n_list: String, // JSON array
    pub address_n_list_master: String, // JSON array
    pub curve: String,
    pub show_display: bool,
    pub is_default: bool,
    pub tags: Option<String>,
    pub version: i32,
    pub created_at: i64,
    pub last_updated: i64,
}

// ========== Wallet Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletXpub {
    pub id: i64,
    pub device_id: String,
    pub path: String,
    pub label: String,
    pub caip: String,
    pub pubkey: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletXpubInput {
    pub device_id: String,
    pub path: String,
    pub label: String,
    pub caip: String,
    pub pubkey: String,
}

// ========== Cache Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPubkey {
    pub id: i64,
    pub device_id: String,
    pub derivation_path: String,
    pub coin_name: String,
    pub script_type: Option<String>,
    pub xpub: Option<String>,
    pub address: Option<String>,
    pub chain_code: Option<Vec<u8>>,
    pub public_key: Option<Vec<u8>>,
    pub cached_at: i64,
    pub last_used: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub device_id: String,
    pub label: Option<String>,
    pub firmware_version: Option<String>,
    pub initialized: Option<bool>,
    pub frontload_status: Option<String>,
    pub frontload_progress: i32,
    pub last_frontload: Option<i64>,
    pub error_message: Option<String>,
}

// ========== Transaction Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionCache {
    pub id: i64,
    pub device_id: String,
    pub txid: String,
    pub caip: String,
    pub transaction_type: String,
    pub amount: String,
    pub amount_usd: Option<String>,
    pub fee: Option<String>,
    pub fee_usd: Option<String>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub timestamp: i64,
    pub block_height: Option<i64>,
    pub status: Option<String>,
    pub metadata_json: Option<String>,
}

// ========== Meta/Preferences Types ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preference {
    pub key: String,
    pub value: String,
}

// ========== Setup Flow Types ==========

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupStep {
    DeviceConnection = 0,
    VerifyBootloader = 1,
    VerifyFirmware = 2,
    SetupWallet = 3,
    Complete = 4,
}

impl From<u8> for SetupStep {
    fn from(value: u8) -> Self {
        match value {
            0 => SetupStep::DeviceConnection,
            1 => SetupStep::VerifyBootloader,
            2 => SetupStep::VerifyFirmware,
            3 => SetupStep::SetupWallet,
            4 => SetupStep::Complete,
            _ => SetupStep::DeviceConnection,
        }
    }
}

impl From<SetupStep> for u8 {
    fn from(step: SetupStep) -> Self {
        step as u8
    }
} 