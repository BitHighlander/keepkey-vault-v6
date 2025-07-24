# Receive Page Planning Document

## Overview
The Receive page needs to handle a complex address derivation and management system while presenting a simple, intuitive interface to users. This document outlines the complete functionality required.

## User Interface Components

### Primary Controls
1. **Account Dropdown**
   - Options: Account 1, Account 2, Account 3, etc.
   - "Add More Account" option at bottom
   - Accounts are BIP44 account level (m/purpose'/coin'/account')
   - Each account maintains independent address chains

2. **Script Type Dropdown**
   - Legacy (P2PKH) - BIP44 - m/44'/0'/account'
   - SegWit (P2SH-P2WPKH) - BIP49 - m/49'/0'/account'  
   - Native SegWit (P2WPKH) - BIP84 - m/84'/0'/account'
   - Default: Native SegWit for new users

3. **Change Address Toggle**
   - OFF: Show external/receive addresses (chain 0)
   - ON: Show internal/change addresses (chain 1)
   - Most users should never need this
   - Advanced feature for address verification

### Display Elements
4. **Current Address Display**
   - QR Code of current address
   - Address string (copyable)
   - Derivation path in BIP39 format
   - Usage status indicator

5. **Action Buttons**
   - "Generate Next Address" 
   - "Copy Address"
   - "View on Device" (hardware verification)
   - "Mark as Used" (offline mode only)

## State Management

### Wallet State
```
{
  selectedDeviceId: string,
  selectedAccount: number,
  selectedScriptType: 'p2pkh' | 'p2sh-p2wpkh' | 'p2wpkh',
  showChangeAddresses: boolean,
  currentAddressIndex: number,
  isOnlineMode: boolean
}
```

### Address State
```
{
  deviceId: string,
  account: number,
  scriptType: string,
  isChange: boolean,
  addressIndex: number,
  address: string,
  pubkey: string,
  derivationPath: string,
  isUsed: boolean,
  firstSeen: timestamp,
  lastChecked: timestamp
}
```

## Backend Integration

### Address Generation
- **Fresh Receive Address**: Backend determines next unused external address index
- **Fresh Change Address**: Backend determines next unused internal address index
- **Address Lookup**: Query backend for "new address" and "new change" per account/script type
- **Device Communication**: Generate addresses through KeepKey device

### Online vs Offline Modes

#### Online Mode
- Backend queries blockchain to determine address usage
- Automatic `is_used` status updates
- Real-time fresh address discovery
- Transaction history integration

#### Offline Mode  
- Manual `is_used` toggle for users
- Address table shows all generated addresses
- User must manually mark addresses as used
- "View on Device" for address verification

## Data Storage (SQLite)

### Addresses Table
```sql
CREATE TABLE addresses (
    id INTEGER PRIMARY KEY,
    device_id TEXT NOT NULL,
    account INTEGER NOT NULL,
    script_type TEXT NOT NULL,
    is_change BOOLEAN NOT NULL,
    address_index INTEGER NOT NULL,
    address TEXT NOT NULL,
    pubkey TEXT,
    derivation_path TEXT NOT NULL,
    is_used BOOLEAN DEFAULT FALSE,
    is_used_manual BOOLEAN DEFAULT FALSE,
    first_seen TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_checked TIMESTAMP,
    UNIQUE(device_id, account, script_type, is_change, address_index)
);
```

### Accounts Table
```sql
CREATE TABLE accounts (
    id INTEGER PRIMARY KEY,
    device_id TEXT NOT NULL,
    account_number INTEGER NOT NULL,
    label TEXT,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(device_id, account_number)
);
```

## Address Derivation Logic

### BIP Standards
- **BIP44 (Legacy)**: m/44'/0'/account'/change/address_index
- **BIP49 (SegWit)**: m/49'/0'/account'/change/address_index  
- **BIP84 (Native SegWit)**: m/84'/0'/account'/change/address_index

### Path Components
- **Purpose**: 44/49/84 (hardened)
- **Coin Type**: 0 for Bitcoin (hardened)
- **Account**: 0,1,2,3... (hardened)
- **Change**: 0 (external/receive) or 1 (internal/change)
- **Address Index**: 0,1,2,3... (non-hardened)

## Complex Processes

### Address Usage Detection
1. **Online Method**: 
   - Query blockchain APIs for transaction history
   - Mark address as used if found in any transaction
   - Update `is_used` flag automatically

2. **Offline Method**:
   - User manually toggles `is_used_manual` flag
   - Show warning about manual management
   - Provide address table for bulk management

### Fresh Address Discovery
1. Backend maintains "gap limit" (typically 20)
2. Scans consecutive unused addresses up to gap limit
3. Returns next unused address index
4. Handles both external and change address chains

### Device-Specific Storage
- All addresses tied to specific `device_id`
- Multiple devices can have same derivation paths
- Cross-device address collision handling
- Device switching preserves per-device state

## User Experience Flow

### Standard Flow (Online)
1. User selects account and script type
2. System shows current fresh address
3. User copies address for receiving funds
4. System automatically detects usage and increments

### Advanced Flow (Offline)
1. User enables offline mode
2. System shows address management table
3. User can manually mark addresses as used
4. "View on Device" for address verification
5. Manual fresh address generation

### Change Address Flow
1. User toggles "Change Addresses" 
2. System shows warning about advanced feature
3. User can view/verify change addresses on device
4. Primarily for transaction verification

## Security Considerations

### Address Verification
- "View on Device" shows address on KeepKey screen
- User must confirm address matches display
- Prevents man-in-the-middle attacks
- Critical for large amounts

### Gap Limit Management
- Maintain proper gap limits per BIP44
- Warn user about address reuse
- Handle edge cases with large gaps

### Privacy Considerations
- Fresh address for each transaction
- Change address explanation
- Address clustering awareness

## Implementation Phases

### Phase 1: Basic Functionality
- Account and script type selection
- Fresh address generation
- QR code display
- Copy functionality

### Phase 2: Advanced Features  
- Change address toggle
- Address usage tracking
- SQLite storage implementation
- Offline mode support

### Phase 3: Management Interface
- Address table view
- Manual usage marking
- Bulk address operations
- Account management

### Phase 4: Device Integration
- "View on Device" functionality
- Hardware address verification
- Device-specific state management
- Multi-device support

## Technical Requirements

### Frontend
- React state management for complex UI state
- SQLite integration for local storage
- QR code generation
- Device communication interfaces

### Backend  
- Address derivation logic
- Blockchain API integration
- Gap limit management
- Fresh address discovery algorithms

### Database
- Efficient address indexing
- Device-specific partitioning
- Usage status tracking
- Performance optimization for large address sets

This planning document provides the foundation for implementing a robust, user-friendly receive page that handles the complexity of Bitcoin address management while presenting a clean interface to users. 