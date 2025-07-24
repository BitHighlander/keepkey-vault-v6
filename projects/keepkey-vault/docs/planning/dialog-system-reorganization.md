# Dialog System Reorganization Plan

## Overview
The current dialog system in the KeepKey Vault application needs reorganization to improve maintainability, reduce complexity, and provide better type safety.

## Current State Analysis

### Issues Identified
1. **Mixed Dialog Management**: Dialogs are managed through multiple systems:
   - Direct component state (useState in various components)
   - DialogContext with priority queue system
   - Direct modal/dialog components
   - Mixed Chakra UI v3 dialog components and custom implementations

2. **Type Safety Issues**: 
   - Many components have missing or incomplete TypeScript definitions
   - Props are often typed as `any`
   - Missing interfaces for dialog props

3. **Component Organization**:
   - Dialog components scattered across different directories
   - Some dialogs are wizards, some are simple modals
   - Inconsistent naming conventions

4. **State Management Complexity**:
   - DialogContext manages a queue but not all dialogs use it
   - Some dialogs manage their own open/close state
   - Nested dialogs cause complexity

## Proposed Architecture

### 1. Unified Dialog System

```typescript
// Core dialog types
interface Dialog {
  id: string;
  type: DialogType;
  priority: DialogPriority;
  props: Record<string, any>;
  persistent?: boolean;
}

enum DialogType {
  Modal = 'modal',
  Wizard = 'wizard',
  Alert = 'alert',
  Confirmation = 'confirmation'
}

enum DialogPriority {
  Low = 'low',
  Normal = 'normal',
  High = 'high',
  Critical = 'critical'
}
```

### 2. Directory Structure

```
src/
  dialogs/
    core/
      DialogProvider.tsx
      DialogContext.tsx
      DialogTypes.ts
      useDialog.ts
    components/
      Modal/
        Modal.tsx
        Modal.types.ts
        Modal.styles.ts
      Wizard/
        Wizard.tsx
        WizardStep.tsx
        Wizard.types.ts
      Alert/
        Alert.tsx
        Alert.types.ts
      Confirmation/
        Confirmation.tsx
        Confirmation.types.ts
    implementations/
      device/
        BootloaderUpdateDialog.tsx
        FirmwareUpdateDialog.tsx
        DeviceInvalidStateDialog.tsx
        PinUnlockDialog.tsx
      wallet/
        WalletCreationWizard/
          index.tsx
          steps/
            FactoryState.tsx
            DeviceLabel.tsx
            DevicePin.tsx
            BackupDisplay.tsx
            RecoveryFlow.tsx
        SeedVerificationWizard/
      onboarding/
        OnboardingWizard/
      settings/
        SettingsDialog.tsx
      troubleshooting/
        TroubleshootingWizard/
```

### 3. Core Components

#### DialogProvider
- Manages global dialog state
- Handles dialog queue and priority
- Provides context to entire app

#### Dialog Base Components
- **Modal**: Simple modal dialogs
- **Wizard**: Multi-step wizards with navigation
- **Alert**: Simple alert messages
- **Confirmation**: Yes/no confirmation dialogs

### 4. Implementation Phases

#### Phase 1: Setup Infrastructure
- [ ] Create new directory structure
- [ ] Define TypeScript interfaces and types
- [ ] Create base dialog components
- [ ] Implement new DialogProvider

#### Phase 2: Migrate Existing Dialogs
- [ ] Migrate simple modals (Alert, Confirmation)
- [ ] Migrate device-related dialogs
- [ ] Migrate wallet creation wizard
- [ ] Migrate other wizards

#### Phase 3: Remove Old System
- [ ] Remove old DialogContext
- [ ] Remove scattered dialog components
- [ ] Update all imports
- [ ] Clean up unused code

#### Phase 4: Testing & Documentation
- [ ] Add unit tests for dialog system
- [ ] Add integration tests
- [ ] Document usage patterns
- [ ] Create migration guide

## Benefits

1. **Consistency**: All dialogs follow same patterns
2. **Type Safety**: Full TypeScript support
3. **Maintainability**: Clear organization and separation
4. **Testability**: Easier to test isolated components
5. **Performance**: Better control over rendering
6. **Developer Experience**: Clear APIs and patterns

## Migration Strategy

### Step 1: Parallel Implementation
- Build new system alongside existing one
- No breaking changes initially

### Step 2: Gradual Migration
- Migrate one dialog at a time
- Test thoroughly after each migration
- Keep both systems working

### Step 3: Cutover
- Switch to new system completely
- Remove old code
- Update documentation

## Code Examples

### Using the New Dialog System

```typescript
// Simple modal
const { openDialog } = useDialog();

openDialog({
  type: DialogType.Modal,
  props: {
    title: 'Confirm Action',
    content: 'Are you sure?',
    onConfirm: () => console.log('Confirmed'),
    onCancel: () => console.log('Cancelled')
  }
});

// Wizard
openDialog({
  type: DialogType.Wizard,
  props: {
    title: 'Wallet Creation',
    steps: ['factory', 'label', 'pin', 'backup'],
    onComplete: (data) => console.log('Wizard completed', data)
  }
});
```

### Creating a Custom Dialog

```typescript
// CustomDialog.tsx
import { Dialog } from '@/dialogs/core';

interface CustomDialogProps extends DialogProps {
  customProp: string;
}

export const CustomDialog: React.FC<CustomDialogProps> = ({
  onClose,
  customProp
}) => {
  return (
    <Dialog onClose={onClose}>
      <Dialog.Header>Custom Dialog</Dialog.Header>
      <Dialog.Body>
        {customProp}
      </Dialog.Body>
      <Dialog.Footer>
        <Button onClick={onClose}>Close</Button>
      </Dialog.Footer>
    </Dialog>
  );
};
```

## Timeline

- **Week 1**: Setup infrastructure and base components
- **Week 2-3**: Migrate existing dialogs
- **Week 4**: Testing and documentation
- **Week 5**: Cleanup and optimization

## Success Metrics

1. All dialogs using unified system
2. 100% TypeScript coverage
3. Reduced bundle size
4. Improved render performance
5. Simplified component code

## Next Steps

1. Review and approve this plan
2. Create detailed technical specifications
3. Set up new directory structure
4. Begin implementation
