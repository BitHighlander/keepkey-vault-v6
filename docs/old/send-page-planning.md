# Send Page Planning Document

## Overview
The **Send** page allows users to construct and broadcast Bitcoin transactions while balancing simplicity for novices and granular control for power users. The workflow spans **compose → review → sign → broadcast**. This document defines UI/UX, state, storage, and backend requirements, mirroring the level of detail found in *receive-page-planning.md*.

---
## 1. User Interface Components

### 1.1 Compose Section
| Element | Easy Mode | Advanced / Coin Control | Notes |
|---------|-----------|-------------------------|-------|
| **Recipient(s)** | Single address + amount fields. | Multiple outputs table (address, amount, label). | Future: payjoin & lightning support. |
| **Fee Selector** | Radio buttons: _slow / medium / fast_.  Displays fiat estimate. | Numeric sat/vB input + mempool stats helper. | Fee presets configurable. |
| **Input Selector** | **Slider** (0 → 100 %) that auto-picks inputs by ascending age/value. | **Coin Control Table** listing every UTXO with: checkbox _selected_, lock 🔒 toggle, value, address, confirmations. | Locked inputs are persisted in DB. |
| **Max Button** | ✔ | ✔ | Sets amount = spendable balance minus fees. |

### 1.2 Review Transaction Dialog (Inspired by keepkey-client UTXO review)

**Layout**: Tabbed interface with clean approve/reject buttons

```tsx
<Tabs defaultIndex={0}>
  <TabList>
    <Tab>Summary</Tab>
    <Tab>Fees</Tab>
    <Tab>Advanced</Tab>
  </TabList>
  <TabPanels>
    <TabPanel>
      {/* Summary Tab - Main Review */}
      <VStack spacing={4}>
        {/* Asset Icon */}
        <Avatar size="lg" src={assetIcon} />
        
        {/* Transaction Flow Diagram */}
        <TransactionDiagram inputs={inputs} outputs={outputs} fee={fee} />
        
        {/* Inputs Section */}
        <VStack spacing={2} align="stretch">
          <Text fontWeight="bold">Inputs ({inputs.length})</Text>
          {inputs.map((input, i) => (
            <HStack key={i} justify="space-between">
              <Badge colorScheme="teal">UTXO</Badge>
              <Text fontSize="xs">{input.txid.slice(0,8)}...:{input.vout}</Text>
              <Text fontWeight="medium">{formatBTC(input.amount)}</Text>
            </HStack>
          ))}
        </VStack>
        
        {/* Outputs Section */}
        <VStack spacing={2} align="stretch">
          <Text fontWeight="bold">Outputs</Text>
          {outputs.map((output, i) => (
            <VStack key={i} align="stretch">
              <HStack justify="space-between">
                <Badge colorScheme={output.isChange ? 'purple' : 'orange'}>
                  {output.isChange ? 'Change to' : 'To'}
                </Badge>
                <Text fontSize="xs">{output.address.slice(0,12)}...</Text>
              </HStack>
              <HStack justify="space-between">
                <Badge colorScheme="green">Amount</Badge>
                <Text fontWeight="medium">{formatBTC(output.amount)}</Text>
              </HStack>
            </VStack>
          ))}
        </VStack>
        
        {/* Totals Summary */}
        <Divider />
        <VStack spacing={2} align="stretch">
          <HStack justify="space-between">
            <Text fontWeight="semibold">Total Input</Text>
            <Text>{formatBTC(totalInput)}</Text>
          </HStack>
          <HStack justify="space-between">
            <Text fontWeight="semibold">Total Output</Text>
            <Text>{formatBTC(totalOutput)}</Text>
          </HStack>
          <HStack justify="space-between">
            <Text fontWeight="semibold" color="orange.400">Network Fee</Text>
            <Text color="orange.400">{formatBTC(fee)} (${feeUSD})</Text>
          </HStack>
        </VStack>
      </VStack>
    </TabPanel>
    
    <TabPanel>
      {/* Fee Details Tab */}
      <VStack spacing={4}>
        <Text fontSize="lg" fontWeight="bold">Fee Breakdown</Text>
        
        <RadioGroup value={feeOption} onChange={setFeeOption}>
          <VStack align="flex-start">
            <Radio value="high">High ({feeRates.high} sat/vB) - ${highFeeUSD}</Radio>
            <Radio value="medium">Medium ({feeRates.medium} sat/vB) - ${mediumFeeUSD}</Radio>
            <Radio value="low">Low ({feeRates.low} sat/vB) - ${lowFeeUSD}</Radio>
            <Radio value="custom">Custom</Radio>
          </VStack>
        </RadioGroup>
        
        {feeOption === 'custom' && (
          <Input 
            placeholder="Enter fee rate (sat/vB)"
            value={customFeeRate}
            onChange={setCustomFeeRate}
            type="number"
          />
        )}
        
        <Table size="sm">
          <Tbody>
            <Tr><Td>Transaction Size</Td><Td>{txSize} vBytes</Td></Tr>
            <Tr><Td>Fee Rate</Td><Td>{selectedFeeRate} sat/vB</Td></Tr>
            <Tr><Td>Total Fee</Td><Td>{fee} sats (${feeUSD})</Td></Tr>
            <Tr><Td>BTC Price</Td><Td>${btcPrice}</Td></Tr>
          </Tbody>
        </Table>
      </VStack>
    </TabPanel>
    
    <TabPanel>
      {/* Advanced/Raw Tab */}
      <VStack spacing={4}>
        <Text fontSize="lg" fontWeight="bold">Transaction Details</Text>
        <Box bg="gray.800" p={4} borderRadius="md" fontSize="xs" fontFamily="mono">
          {/* Raw PSBT or transaction hex */}
          <Text wordBreak="break-all">{rawTransaction}</Text>
        </Box>
        
        {/* Security Warnings */}
        {warnings.length > 0 && (
          <VStack spacing={2} align="stretch">
            <Text fontWeight="bold" color="orange.400">⚠️ Warnings</Text>
            {warnings.map((warning, i) => (
              <Text key={i} fontSize="sm" color="orange.300">{warning}</Text>
            ))}
          </VStack>
        )}
      </VStack>
    </TabPanel>
  </TabPanels>
</Tabs>

{/* Action Buttons */}
<HStack justify="center" mt={6} spacing={4}>
  <Button colorScheme="green" size="lg" onClick={handleApprove}>
    ✓ Approve Transaction
  </Button>
  <Button colorScheme="red" variant="outline" size="lg" onClick={handleReject}>
    ✗ Reject
  </Button>
</HStack>
```

### 1.3 Transaction Flow Diagram Component

```tsx
const TransactionDiagram = ({ inputs, outputs, fee }) => (
  <Box textAlign="center" my={4}>
    <Text fontSize="sm" fontWeight="bold" mb={2}>Transaction Flow</Text>
    
    {/* Inputs flowing down */}
    <Flex justify="center" mb={2}>
      {inputs.map((input, i) => (
        <VStack key={i} spacing={1} mx={2}>
          <Text fontSize="xs">Input {i + 1}</Text>
          <Box w="2px" h="20px" bg="blue.400" />
        </VStack>
      ))}
    </Flex>
    
    {/* Central transaction node */}
    <Box 
      border="2px solid" 
      borderColor="blue.400"
      borderRadius="full"
      w="80px" h="80px"
      display="flex"
      alignItems="center"
      justifyContent="center"
      mx="auto"
      bg="blue.50"
    >
      <Text fontSize="xs" fontWeight="bold" textAlign="center">
        Bitcoin TX
      </Text>
    </Box>
    
    {/* Outputs and fee flowing out */}
    <Flex justify="center" mt={2}>
      {outputs.map((output, i) => (
        <VStack key={i} spacing={1} mx={2}>
          <Box w="2px" h="20px" bg={output.isChange ? "purple.400" : "green.400"} />
          <Text fontSize="xs">{output.isChange ? "Change" : "Recipient"}</Text>
        </VStack>
      ))}
      {/* Fee line */}
      <VStack spacing={1} mx={2}>
        <Box w="2px" h="20px" bg="orange.400" />
        <Text fontSize="xs" color="orange.400">Network Fee</Text>
      </VStack>
    </Flex>
  </Box>
);
```

### 1.4 Dialogs / Wizards
1. **Review Transaction** - Tabbed interface as shown above
2. **Sign on Device**  
   - Animated instructions + progress bar.  
   - Handles PIN/passphrase requests.  
   - Retries on HID reconnect.
3. **Broadcast Result**  
   - Success txid + explorer link.  
   - Failure reason with retry.

### 1.5 Navigation & Feedback
- Breadcrumbs: **Compose → Review → Sign → Broadcast**.  Greyed steps become blue when complete.
- Toasts for clipboard copy, device errors, broadcast status.

---
## 2. State Management

```ts
interface SendWalletState {
  selectedDeviceId: string;
  selectedAccount: number;           // BIP44 account
  scriptType: 'p2pkh' | 'p2sh-p2wpkh' | 'p2wpkh';
  feeRatePreset: 'slow' | 'medium' | 'fast' | 'custom';
  customFeeRate?: number;            // sat/vB
  isEasyMode: boolean;               // true = slider, false = coin control
  sliderPercent: number;             // 0–100 when easy mode
  selectedInputs: Set<string>;       // txid:vout identifiers
  lockedInputs: Set<string>;         // persisted between sessions
  outputs: Array<{ address: string; amount: string }>;  // BTC in decimals
  changeAddress?: string;            // chosen by backend
  draftTx?: UnsignedTx;              // hex or PSBT
  signProgress?: {
    step: 'waiting' | 'pin' | 'passphrase' | 'signing' | 'done';
    message?: string;
  };
  broadcastStatus?: 'idle' | 'broadcasting' | 'success' | 'error';
  broadcastTxId?: string;
  error?: string;
  
  // Review dialog state
  reviewStep: 'compose' | 'review' | 'sign' | 'broadcast';
  showReviewDialog: boolean;
  reviewTab: 'summary' | 'fees' | 'advanced';
  warnings: string[];              // Security warnings for review
}
```

---
## 3. Database Storage (SQLite)

### 3.1 UTXOs Table
```sql
CREATE TABLE utxos (
    txid TEXT NOT NULL,
    vout INTEGER NOT NULL,
    device_id TEXT NOT NULL,
    account INTEGER NOT NULL,
    script_type TEXT NOT NULL,
    address TEXT NOT NULL,
    amount_sat INTEGER NOT NULL,
    confirmations INTEGER NOT NULL,
    is_locked BOOLEAN DEFAULT FALSE,
    PRIMARY KEY (txid, vout)
);
```
- `is_locked` corresponds to user-toggled 🔒 in coin control.

### 3.2 Pending Transactions
```sql
CREATE TABLE pending_txs (
    id INTEGER PRIMARY KEY,
    device_id TEXT,
    account INTEGER,
    psbt BLOB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```
Stores drafts if user closes app mid-signing.

---
## 4. Backend Responsibilities (Rust Axum)
1. **`/api/v2/utxos`** → return fresh UTXO set per account.
2. **`/api/v2/lock_utxo`** → toggle lock state in DB.
3. **`/api/v2/tx/build`**  
   Inputs: recipient list, feeRate, optional selectedInputs.  
   Returns: unsigned PSBT, fee, size, change output.
4. **`/api/v2/tx/sign`**  
   Consumes PSBT + device_id, returns signed PSBT.
5. **`/api/v2/tx/broadcast`** → pushes raw tx, returns txid.
6. **Coin Selection Algorithm**  
   - **Easy**: BnB or knapsack using sliderPercent of spendable.
   - **Advanced**: exact set chosen by user.

---
## 5. Transaction Flow
```text
User Input → /tx/build → Review Dialog → Sign Device → /tx/sign → /tx/broadcast → Done!
```
Steps with error rollbacks:
1. **Build** fails ⇒ show error in compose.
2. **Sign** fails ⇒ keep PSBT for retry.
3. **Broadcast** fails ⇒ allow manual broadcast with raw hex.

---
## 6. Coin Control Table Columns
| Column | Description | Sortable |
|--------|-------------|----------|
| Select ✔ | Checkbox for spending | N/A |
| Lock 🔒 | Prevent future spending | N/A |
| Amount (BTC) | Value of UTXO | ✔ |
| Address | Original deposit address |  |
| Confirmations | Age indicator | ✔ |
| Label | User label (optional) | ✔ |

Context menu options: _"Copy Outpoint", "Add Label", "View on Explorer"_.

---
## 7. Privacy & Security Considerations
- Warn if mixing script types within one tx.
- Highlight high-priority inputs that deanonymise clusters.
- Change address must use same script type as inputs.
- RBF flag on by default; allow disabling.

### 7.1 Security Warnings in Review Dialog
```tsx
const generateWarnings = (inputs, outputs) => {
  const warnings = [];
  
  // Mixed script types
  const scriptTypes = [...new Set(inputs.map(i => i.scriptType))];
  if (scriptTypes.length > 1) {
    warnings.push("⚠️ Mixing script types may reduce privacy");
  }
  
  // Low confirmations
  const lowConfirmInputs = inputs.filter(i => i.confirmations < 3);
  if (lowConfirmInputs.length > 0) {
    warnings.push(`⚠️ ${lowConfirmInputs.length} input(s) have low confirmations`);
  }
  
  // Sending entire balance
  if (outputs.filter(o => !o.isChange).length === 1 && !outputs.some(o => o.isChange)) {
    warnings.push("⚠️ Sending entire balance (no change output)");
  }
  
  return warnings;
};
```

---
## 8. Implementation Phases
1. **Phase 1: Easy Mode MVP**  
   - Single-recipient send, slider input selection, review/sign/broadcast dialogs.
   - **Review Dialog**: Summary tab with basic input/output display
2. **Phase 2: Advanced Coin Control**  
   - Full UTXO table, lock support, multi-recipient outputs.
   - **Review Dialog**: Complete tabbed interface with fee management
3. **Phase 3: Persistence & Recovery**  
   - Draft PSBT storage, pending tx resume.
   - **Review Dialog**: Advanced tab with raw transaction data
4. **Phase 4: Power Features**  
   - Custom change address, coinjoin, payjoin, child-pays-for-parent.
   - **Review Dialog**: Enhanced security warnings and privacy analysis

---
*Prepared: 2025-06-12*  
*Updated with UX examples from keepkey-client UTXO review: 2025-01-09*
