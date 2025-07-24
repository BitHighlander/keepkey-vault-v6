# Current Dialog System Analysis

## Overview
This document analyzes the current state of the dialog system in the KeepKey Vault application.

## Components Inventory

### Dialog Context System
- **Location**: `src/contexts/DialogContext.tsx`
- **Purpose**: Global dialog management with priority queue
- **Features**:
  - Priority-based dialog queue
  - Lazy loading with React Suspense
  - Dialog state management
  - Various dialog type support

### Dialog Components

#### Device-Related Dialogs
1. **BootloaderUpdateDialog**
   - Location: `src/components/BootloaderUpdateDialog.tsx`
   - Purpose: Bootloader firmware updates
   - Status: Has stub implementation

2. **FirmwareUpdateDialog**
   - Location: `src/components/FirmwareUpdateDialog.tsx`
   - Purpose: Device firmware updates
   - Status: Has stub implementation

3. **DeviceInvalidStateDialog**
   - Location: `src/components/DeviceInvalidStateDialog.tsx`
   - Purpose: Handle invalid device states
   - Status: Stub only

4. **EnterBootloaderModeDialog**
   - Location: `src/components/EnterBootloaderModeDialog.tsx`
   - Purpose: Guide user to enter bootloader mode
   - Status: Implemented

5. **PinUnlockDialog**
   - Location: `src/components/PinUnlockDialog.tsx`
   - Purpose: PIN entry for device unlock
   - Status: Implemented

#### Wallet-Related Dialogs
1. **WalletCreationWizard**
   - Location: `src/components/WalletCreationWizard/`
   - Components:
     - WalletCreationWizard.tsx (main)
     - FactoryState.tsx
     - DeviceLabel.tsx
     - DevicePin.tsx
     - RecoveryFlow.tsx
     - RecoveryPin.tsx
     - RecoverySettings.tsx
   - Purpose: Multi-step wallet creation
   - Status: Partially implemented

2. **SeedVerificationWizard**
   - Location: `src/components/SeedVerificationWizard/`
   - Purpose: Seed phrase verification
   - Status: Stub only

#### Other Dialogs
1. **OnboardingWizard**
   - Location: `src/components/OnboardingWizard/`
   - Purpose: First-time user onboarding
   - Status: Stub only

2. **TroubleshootingWizard**
   - Location: `src/components/TroubleshootingWizard/`
   - Purpose: Help users troubleshoot issues
   - Status: Stub only

3. **SettingsDialog**
   - Location: `src/components/SettingsDialog.tsx`
   - Purpose: Application settings
   - Status: Implemented

### UI Components
1. **dialog.tsx**
   - Location: `src/components/ui/dialog.tsx`
   - Purpose: Chakra UI v3 dialog replacements
   - Status: Basic implementation

2. **modal.tsx**
   - Location: `src/components/ui/modal.tsx`
   - Purpose: Basic modal components
   - Status: Basic implementation

## Current Issues

### 1. Inconsistent Implementation
- Some dialogs are fully implemented, others are stubs
- Mixed use of Chakra UI components and custom implementations
- No consistent pattern for dialog creation

### 2. Type Safety Problems
```typescript
// Many components use 'any' types
export const KeepKeyDeviceList = (props: any) => {
  return <div>KeepKeyDeviceList Stub</div>;
};

// Missing proper interfaces
const SeedVerificationWizard = (props: any) => {
  return <div>SeedVerificationWizard Stub</div>;
};
```

### 3. State Management Complexity
- DialogContext manages a queue but not all dialogs use it
- Some components manage their own open/close state
- Inconsistent patterns for dialog lifecycle

### 4. Organization Issues
- Dialogs scattered across component directories
- No clear separation between different dialog types
- Mixed concerns (UI + business logic)

### 5. Missing Features
- No built-in animation support
- Limited accessibility features
- No consistent error handling
- No dialog composition patterns

## Usage Patterns

### Current Dialog Opening Pattern
```typescript
// Using DialogContext
const { showDialog } = useDialog();
showDialog({
  id: 'firmware-update',
  component: FirmwareUpdateDialog,
  props: { deviceId: '123' }
});

// Direct component usage
const [isOpen, setIsOpen] = useState(false);
<FirmwareUpdateDialog 
  isOpen={isOpen} 
  onClose={() => setIsOpen(false)} 
/>
```

### Problems with Current Patterns
1. Inconsistent APIs
2. No type safety for props
3. Manual state management
4. No standard error handling

## Dependencies

### Current Dependencies
- Chakra UI v3 (partially integrated)
- React 18.3.1
- TypeScript (with relaxed settings)

### Missing Type Definitions
- Several components lack proper TypeScript interfaces
- Props often typed as `any`
- No central type definitions for dialogs

## Recommendations

1. **Standardize Dialog Interface**: Create base interfaces all dialogs must implement
2. **Centralize Dialog Management**: Use single system for all dialogs
3. **Improve Type Safety**: Add proper TypeScript definitions
4. **Organize Components**: Create clear directory structure
5. **Add Testing**: Currently no tests for dialog components
6. **Document Patterns**: Create clear usage documentation

## Migration Risks

1. **Breaking Changes**: Need careful migration to avoid breaking existing functionality
2. **State Management**: Must preserve current dialog behavior
3. **Performance**: Ensure new system doesn't degrade performance
4. **Testing Gap**: No existing tests to validate migration

## Next Steps

1. Review this analysis
2. Prioritize issues to address
3. Design new architecture
4. Plan migration strategy
5. Implement incrementally