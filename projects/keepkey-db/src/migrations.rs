use crate::errors::Result;
use rusqlite::Connection;

/// Initialize the database schema
pub fn apply_migrations(conn: &Connection) -> Result<()> {
    // Enable WAL mode and foreign keys
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    
    log::info!("Creating database schema...");
    
    // Create all tables at once
    conn.execute_batch(FULL_SCHEMA)?;
    
    log::info!("Database schema created successfully");
    Ok(())
}

// Complete database schema - all tables, indexes, views, and triggers
const FULL_SCHEMA: &str = r#"
-- KeepKey Database Schema v6
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys = ON;

-- Core accounts table for wallet information
CREATE TABLE IF NOT EXISTS accounts (
    id           INTEGER PRIMARY KEY,
    wallet_fp    TEXT NOT NULL,      -- 4-byte fingerprint (hex)
    kind         TEXT NOT NULL,      -- 'keepkey' | 'digital'
    xpub         TEXT NOT NULL,
    label        TEXT,
    added_ts     INTEGER NOT NULL    -- epoch seconds
);

-- Addresses table for derived addresses
CREATE TABLE IF NOT EXISTS addresses (
    id           INTEGER PRIMARY KEY,
    account_id   INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    address      TEXT NOT NULL UNIQUE,
    deriv_path   TEXT NOT NULL,      -- "m/84'/0'/0'/0/15"
    first_seen   INTEGER             -- block height
);

-- Transactions table
CREATE TABLE IF NOT EXISTS txs (
    txid         TEXT PRIMARY KEY,
    account_id   INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    block_height INTEGER,
    direction    INTEGER NOT NULL,   -- +sats (recv) / -sats (send)
    amount       INTEGER NOT NULL,   -- satoshis (always positive)
    fee          INTEGER,            -- satoshis
    timestamp    INTEGER             -- tx time per node
);

-- Devices table with setup tracking
CREATE TABLE IF NOT EXISTS devices (
    device_id    TEXT PRIMARY KEY,   -- Unique device ID
    vendor       TEXT,
    model        TEXT,
    label        TEXT,
    firmware_variant TEXT,
    firmware_version TEXT,
    bootloader_mode BOOLEAN,
    initialized  BOOLEAN,
    pin_protection BOOLEAN,
    passphrase_protection BOOLEAN,
    first_seen   INTEGER NOT NULL,   -- epoch seconds
    last_seen    INTEGER NOT NULL,   -- epoch seconds
    features     TEXT,               -- JSON blob of full features
    
    -- Setup tracking columns
    serial_number TEXT,              -- Device serial number
    setup_complete BOOLEAN DEFAULT FALSE, -- Whether mandatory setup is complete
    setup_step_completed INTEGER DEFAULT 0, -- Last completed setup step (0-4)
    eth_address TEXT,                -- Cached Ethereum address after setup
    setup_started_at INTEGER,        -- Timestamp when setup began
    setup_completed_at INTEGER       -- Timestamp when setup finished
);

-- Device connections table for tracking connection history
CREATE TABLE IF NOT EXISTS device_connections (
    id           INTEGER PRIMARY KEY,
    device_id    TEXT NOT NULL REFERENCES devices(device_id),
    connected_at INTEGER NOT NULL,   -- epoch seconds
    disconnected_at INTEGER,         -- epoch seconds, NULL if still connected
    session_data TEXT                -- JSON blob of session-specific data
);

-- Wallet XPUBs table for device-derived public keys
CREATE TABLE IF NOT EXISTS wallet_xpubs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id    TEXT NOT NULL,
    path         TEXT NOT NULL,      -- "m/44'/0'/0'"
    label        TEXT NOT NULL,      -- "Bitcoin Legacy"
    caip         TEXT NOT NULL,      -- "bip122:000000000019d6689c085ae165831e93/slip44:0"
    pubkey       TEXT NOT NULL,      -- xpub string
    created_at   INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    UNIQUE(device_id, path, caip),
    FOREIGN KEY (device_id) REFERENCES devices(device_id) ON DELETE CASCADE
);

-- Portfolio cache table for balance data from external APIs
CREATE TABLE IF NOT EXISTS portfolio_cache (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey       TEXT NOT NULL,      -- xpub from wallet_xpubs
    caip         TEXT NOT NULL,      -- matching caip from wallet_xpubs
    balance      TEXT NOT NULL,      -- balance as string (to preserve precision)
    balance_usd  TEXT NOT NULL,      -- USD value as string
    price_usd    TEXT NOT NULL,      -- price per unit in USD
    symbol       TEXT,               -- BTC, etc.
    last_updated INTEGER NOT NULL,   -- epoch seconds
    UNIQUE(pubkey, caip)
);

-- Enhanced portfolio cache with more detailed balance information
CREATE TABLE IF NOT EXISTS portfolio_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    pubkey TEXT NOT NULL,      -- xpub from wallet_xpubs
    caip TEXT NOT NULL,        -- CAIP identifier (e.g., "eip155:1/slip44:60")
    network_id TEXT NOT NULL,  -- Network identifier (e.g., "eip155:1")
    ticker TEXT NOT NULL,      -- Asset ticker (e.g., "ETH", "BTC")
    address TEXT,              -- Specific address if applicable
    balance TEXT NOT NULL,     -- Balance as string (preserve precision)
    balance_usd TEXT NOT NULL, -- USD value as string
    price_usd TEXT NOT NULL,   -- Price per unit in USD
    type TEXT,                 -- 'balance', 'staking', 'delegation', 'reward', 'unbonding'
    
    -- Additional fields from pioneer-sdk
    name TEXT,                 -- Asset full name
    icon TEXT,                 -- Asset icon URL
    precision INTEGER,         -- Decimal places for display
    contract TEXT,             -- Contract address for tokens
    
    -- Staking specific fields
    validator TEXT,            -- Validator address for delegations
    unbonding_end INTEGER,     -- Timestamp when unbonding completes
    rewards_available TEXT,    -- Available rewards amount
    
    -- Metadata
    last_updated INTEGER NOT NULL,
    last_block_height INTEGER,
    is_verified BOOLEAN DEFAULT 0,
    
    UNIQUE(device_id, pubkey, caip, address, type, validator)
);

-- Dashboard aggregation cache (pre-computed totals)
CREATE TABLE IF NOT EXISTS portfolio_dashboard (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    total_value_usd TEXT NOT NULL,
    
    -- Network breakdowns (JSON)
    networks_json TEXT NOT NULL,      -- Array of {networkId, name, valueUsd, percentage}
    assets_json TEXT NOT NULL,        -- Array of {ticker, name, valueUsd, balance, percentage}
    
    -- Statistics
    total_assets INTEGER,
    total_networks INTEGER,
    last_24h_change_usd TEXT,
    last_24h_change_percent TEXT,
    
    -- Combined portfolio flag
    is_combined BOOLEAN DEFAULT 0,    -- True if this is a combined multi-device portfolio
    included_devices TEXT,            -- JSON array of device_ids if combined
    
    last_updated INTEGER NOT NULL,
    UNIQUE(device_id)
);

-- Portfolio history for tracking value over time
CREATE TABLE IF NOT EXISTS portfolio_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    total_value_usd TEXT NOT NULL,
    snapshot_json TEXT              -- Full portfolio snapshot as JSON
);

-- Asset registry table - stores all known assets
CREATE TABLE IF NOT EXISTS assets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    caip TEXT NOT NULL UNIQUE,              -- e.g., "eip155:1/slip44:60"
    network_id TEXT NOT NULL,               -- e.g., "eip155:1", "cosmos:cosmoshub-4"
    chain_id TEXT,                          -- e.g., "1" for Ethereum mainnet
    symbol TEXT NOT NULL,                   -- e.g., "ETH", "BTC", "USDC"
    name TEXT NOT NULL,                     -- e.g., "Ethereum", "Bitcoin", "USD Coin"
    
    -- Asset type information
    asset_type TEXT CHECK(asset_type IN ('native', 'token', 'nft')),
    is_native BOOLEAN DEFAULT 0,
    contract_address TEXT,                  -- For tokens/NFTs
    token_id TEXT,                         -- For NFTs
    
    -- Display information
    icon TEXT,                             -- Icon URL
    color TEXT,                            -- Hex color code
    decimals INTEGER,                      -- Decimal places
    precision INTEGER,                     -- Display precision
    
    -- Network information
    network_name TEXT,                     -- e.g., "Ethereum Mainnet"
    native_asset_caip TEXT,                -- CAIP of the native gas asset for this network
    
    -- Explorer links
    explorer TEXT,                         -- Base explorer URL
    explorer_address_link TEXT,            -- Address explorer pattern
    explorer_tx_link TEXT,                 -- Transaction explorer pattern
    
    -- Additional metadata
    coin_gecko_id TEXT,                    -- CoinGecko ID for price data
    chain_reference TEXT,                  -- Reference in chain (e.g., IBC denom)
    tags TEXT,                            -- JSON array of tags
    
    -- Source tracking
    source TEXT DEFAULT 'pioneer-discovery',
    is_verified BOOLEAN DEFAULT 1,
    
    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_updated INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Derivation paths table - stores HD wallet paths from default-paths.json
CREATE TABLE IF NOT EXISTS derivation_paths (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path_id TEXT NOT NULL UNIQUE,          -- e.g., "bitcoin_44", "ethereum_44"
    note TEXT,                             -- Human-readable description
    blockchain TEXT NOT NULL,              -- e.g., "bitcoin", "ethereum", "cosmos"
    symbol TEXT NOT NULL,                  -- Primary symbol (e.g., "BTC", "ETH")
    
    -- Network associations
    networks TEXT NOT NULL,                -- JSON array of network IDs this path works with
    
    -- Script type (for UTXO chains)
    script_type TEXT,                      -- e.g., "p2pkh", "p2sh-p2wpkh", "p2wpkh"
    
    -- BIP32 path components
    address_n_list TEXT NOT NULL,          -- JSON array for account path (e.g., [44, 0, 0])
    address_n_list_master TEXT NOT NULL,   -- JSON array for address path (e.g., [44, 0, 0, 0, 0])
    
    -- Cryptographic curve
    curve TEXT NOT NULL DEFAULT 'secp256k1',
    
    -- UI hints
    show_display BOOLEAN DEFAULT 0,        -- Whether to show on device display
    is_default BOOLEAN DEFAULT 0,          -- Whether this is the default path for the blockchain
    
    -- Additional metadata
    tags TEXT,                            -- JSON array of tags
    version INTEGER DEFAULT 1,             -- Path format version
    
    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_updated INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Path to CAIP mapping table - maps derivation paths to asset CAIPs
CREATE TABLE IF NOT EXISTS path_asset_mapping (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path_id TEXT NOT NULL,                 -- References derivation_paths.path_id
    caip TEXT NOT NULL,                    -- References assets.caip
    network_id TEXT NOT NULL,              -- Network this mapping applies to
    is_primary BOOLEAN DEFAULT 0,          -- Whether this is the primary path for this asset
    
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    
    UNIQUE(path_id, caip, network_id),
    FOREIGN KEY (path_id) REFERENCES derivation_paths(path_id),
    FOREIGN KEY (caip) REFERENCES assets(caip)
);

-- Network metadata table - comprehensive network information
CREATE TABLE IF NOT EXISTS networks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    network_id TEXT NOT NULL UNIQUE,       -- e.g., "eip155:1", "cosmos:cosmoshub-4"
    name TEXT NOT NULL,                    -- e.g., "Ethereum Mainnet", "Cosmos Hub"
    short_name TEXT,                       -- e.g., "ETH", "COSMOS"
    chain_id TEXT,                         -- Chain ID (numeric for EVM, string for others)
    
    -- Network type
    network_type TEXT CHECK(network_type IN ('evm', 'utxo', 'cosmos', 'other')),
    
    -- Native asset
    native_asset_caip TEXT NOT NULL,       -- CAIP of native asset
    native_symbol TEXT NOT NULL,           -- Native asset symbol
    
    -- RPC endpoints
    rpc_urls TEXT,                         -- JSON array of RPC URLs
    ws_urls TEXT,                          -- JSON array of WebSocket URLs
    
    -- Explorer information
    explorer_url TEXT,
    explorer_api_url TEXT,
    explorer_api_key_required BOOLEAN DEFAULT 0,
    
    -- Chain specific features
    supports_eip1559 BOOLEAN DEFAULT 0,    -- EVM: Supports EIP-1559
    supports_memo BOOLEAN DEFAULT 0,       -- Supports memo/message field
    supports_tokens BOOLEAN DEFAULT 0,     -- Supports token standards
    
    -- Fee configuration
    fee_asset_caip TEXT,                   -- Asset used for fees (usually same as native)
    min_fee TEXT,                          -- Minimum fee in native units
    
    -- Additional metadata
    tags TEXT,                             -- JSON array of tags
    is_testnet BOOLEAN DEFAULT 0,
    is_active BOOLEAN DEFAULT 1,
    
    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_updated INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    
    FOREIGN KEY (native_asset_caip) REFERENCES assets(caip)
);

-- Transaction cache for recent activity
CREATE TABLE IF NOT EXISTS transaction_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    txid TEXT NOT NULL,
    caip TEXT NOT NULL,
    type TEXT NOT NULL,              -- 'send', 'receive', 'swap', 'stake', 'unstake'
    amount TEXT NOT NULL,
    amount_usd TEXT,
    fee TEXT,
    fee_usd TEXT,
    from_address TEXT,
    to_address TEXT,
    timestamp INTEGER NOT NULL,
    block_height INTEGER,
    status TEXT,                     -- 'pending', 'confirmed', 'failed'
    metadata_json TEXT,              -- Additional transaction-specific data
    UNIQUE(device_id, txid, caip)
);

-- Cached public keys and addresses
CREATE TABLE IF NOT EXISTS cached_pubkeys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    derivation_path TEXT NOT NULL,
    coin_name TEXT NOT NULL,
    script_type TEXT,
    xpub TEXT,
    address TEXT,
    chain_code BLOB,
    public_key BLOB,
    cached_at INTEGER NOT NULL,
    last_used INTEGER NOT NULL,
    UNIQUE(device_id, derivation_path, coin_name, script_type)
);

-- Device cache metadata
CREATE TABLE IF NOT EXISTS cache_metadata (
    device_id TEXT PRIMARY KEY,
    label TEXT,
    firmware_version TEXT,
    initialized BOOLEAN,
    frontload_status TEXT CHECK(frontload_status IN ('pending', 'in_progress', 'completed', 'failed')),
    frontload_progress INTEGER DEFAULT 0,
    last_frontload INTEGER,
    error_message TEXT
);

-- Frontload progress tracking per asset/network
CREATE TABLE IF NOT EXISTS frontload_progress (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    network_id TEXT NOT NULL,
    paths_total INTEGER NOT NULL,
    paths_completed INTEGER NOT NULL,
    last_path TEXT,
    status TEXT CHECK(status IN ('pending', 'in_progress', 'completed', 'failed')),
    error_message TEXT,
    started_at INTEGER,
    completed_at INTEGER,
    UNIQUE(device_id, network_id)
);

-- Fee rate cache table for network fee estimates
CREATE TABLE IF NOT EXISTS fee_rate_cache (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    caip         TEXT NOT NULL UNIQUE, -- network identifier
    fastest      INTEGER NOT NULL,    -- sat/vbyte
    fast         INTEGER NOT NULL,    -- sat/vbyte
    average      INTEGER NOT NULL,    -- sat/vbyte
    last_updated INTEGER NOT NULL     -- epoch seconds
);

-- Meta table for key-value storage (including onboarding state)
CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    val TEXT
);

-- ========== INDEXES ==========

-- Core table indexes
CREATE UNIQUE INDEX IF NOT EXISTS idx_accounts_fp_xpub ON accounts(wallet_fp, xpub);
CREATE INDEX IF NOT EXISTS idx_addresses_account ON addresses(account_id);
CREATE INDEX IF NOT EXISTS idx_txs_account_block ON txs(account_id, block_height);

-- Device indexes for performance
CREATE INDEX IF NOT EXISTS idx_devices_setup_incomplete 
ON devices(setup_complete) WHERE setup_complete = FALSE;
CREATE INDEX IF NOT EXISTS idx_devices_serial ON devices(serial_number);
CREATE INDEX IF NOT EXISTS idx_devices_last_seen ON devices(last_seen);
CREATE INDEX IF NOT EXISTS idx_device_connections_device ON device_connections(device_id);
CREATE INDEX IF NOT EXISTS idx_device_connections_time ON device_connections(connected_at, disconnected_at);

-- Wallet indexes
CREATE INDEX IF NOT EXISTS idx_wallet_xpubs_device_id ON wallet_xpubs(device_id);
CREATE INDEX IF NOT EXISTS idx_wallet_xpubs_lookup ON wallet_xpubs(device_id, path, caip);

-- Portfolio indexes
CREATE INDEX IF NOT EXISTS idx_portfolio_cache_updated ON portfolio_cache(last_updated);
CREATE INDEX IF NOT EXISTS idx_portfolio_history_lookup ON portfolio_history(device_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_portfolio_balances_device ON portfolio_balances(device_id);
CREATE INDEX IF NOT EXISTS idx_portfolio_balances_lookup ON portfolio_balances(device_id, network_id, ticker);
CREATE INDEX IF NOT EXISTS idx_portfolio_balances_pubkey ON portfolio_balances(pubkey);
CREATE INDEX IF NOT EXISTS idx_portfolio_balances_updated ON portfolio_balances(last_updated);

-- Cache indexes
CREATE INDEX IF NOT EXISTS idx_cached_pubkeys_lookup ON cached_pubkeys(device_id, derivation_path);
CREATE INDEX IF NOT EXISTS idx_cached_pubkeys_coin ON cached_pubkeys(device_id, coin_name);
CREATE INDEX IF NOT EXISTS idx_cached_pubkeys_last_used ON cached_pubkeys(last_used);

-- Asset indexes
CREATE INDEX IF NOT EXISTS idx_assets_network_id ON assets(network_id);
CREATE INDEX IF NOT EXISTS idx_assets_symbol ON assets(symbol);
CREATE INDEX IF NOT EXISTS idx_assets_contract ON assets(contract_address);
CREATE INDEX IF NOT EXISTS idx_assets_type ON assets(asset_type);
CREATE INDEX IF NOT EXISTS idx_paths_blockchain ON derivation_paths(blockchain);
CREATE INDEX IF NOT EXISTS idx_paths_symbol ON derivation_paths(symbol);

-- Transaction indexes
CREATE INDEX IF NOT EXISTS idx_transaction_cache_device ON transaction_cache(device_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_transaction_cache_status ON transaction_cache(status, timestamp DESC);

-- Fee cache indexes
CREATE INDEX IF NOT EXISTS idx_fee_cache_updated ON fee_rate_cache(last_updated);

-- ========== VIEWS ==========

-- Combined portfolio view across all devices
CREATE VIEW IF NOT EXISTS v_combined_portfolio AS
SELECT 
    'combined' as device_id,
    caip,
    network_id,
    ticker,
    SUM(CAST(balance AS REAL)) as total_balance,
    SUM(CAST(balance_usd AS REAL)) as total_value_usd,
    MAX(price_usd) as price_usd,
    MAX(last_updated) as last_updated
FROM portfolio_balances
WHERE type = 'balance'
GROUP BY caip, network_id, ticker;

-- Per-device portfolio summary
CREATE VIEW IF NOT EXISTS v_device_portfolio_summary AS
SELECT 
    device_id,
    COUNT(DISTINCT caip) as total_assets,
    COUNT(DISTINCT network_id) as total_networks,
    SUM(CAST(balance_usd AS REAL)) as total_value_usd,
    MAX(last_updated) as last_updated
FROM portfolio_balances
WHERE type = 'balance'
GROUP BY device_id;

-- Assets with their network information
CREATE VIEW IF NOT EXISTS v_assets_with_networks AS
SELECT 
    a.*,
    n.name as network_name,
    n.network_type,
    n.explorer_url as network_explorer,
    n.supports_tokens,
    n.is_testnet
FROM assets a
LEFT JOIN networks n ON a.network_id = n.network_id;

-- Derivation paths with their primary assets
CREATE VIEW IF NOT EXISTS v_paths_with_assets AS
SELECT 
    dp.*,
    pam.caip,
    a.symbol as asset_symbol,
    a.name as asset_name,
    a.icon as asset_icon
FROM derivation_paths dp
LEFT JOIN path_asset_mapping pam ON dp.path_id = pam.path_id AND pam.is_primary = 1
LEFT JOIN assets a ON pam.caip = a.caip;

-- ========== TRIGGERS ==========

-- Trigger to update last_used timestamp on access
CREATE TRIGGER IF NOT EXISTS update_last_used_timestamp 
AFTER UPDATE ON cached_pubkeys
FOR EACH ROW
WHEN NEW.last_used = OLD.last_used
BEGIN
    UPDATE cached_pubkeys 
    SET last_used = strftime('%s', 'now') 
    WHERE id = NEW.id;
END;

-- Triggers to update timestamps
CREATE TRIGGER IF NOT EXISTS update_assets_timestamp 
AFTER UPDATE ON assets
FOR EACH ROW
BEGIN
    UPDATE assets SET last_updated = strftime('%s', 'now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_paths_timestamp 
AFTER UPDATE ON derivation_paths
FOR EACH ROW
BEGIN
    UPDATE derivation_paths SET last_updated = strftime('%s', 'now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_networks_timestamp 
AFTER UPDATE ON networks
FOR EACH ROW
BEGIN
    UPDATE networks SET last_updated = strftime('%s', 'now') WHERE id = NEW.id;
END;

-- ========== DEFAULT DATA ==========

-- Default onboarding data
INSERT OR IGNORE INTO meta (key, val) VALUES 
    ('db_version', '6'),
    ('onboarding_completed', 'false'),
    ('first_install_timestamp', CAST(strftime('%s', 'now') AS TEXT));

-- User preferences with defaults
INSERT OR IGNORE INTO meta (key, val) VALUES 
    ('pref_language', 'en'),
    ('pref_theme', 'system'),
    ('pref_currency', 'USD'),
    ('pref_units', 'metric'),
    ('pref_analytics_enabled', 'false');
"#; 