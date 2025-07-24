# Dialog System Technical Specification

## Architecture Overview

### Core Principles
1. **Single Source of Truth**: All dialog state managed by DialogProvider
2. **Type Safety**: Full TypeScript support with no `any` types
3. **Composability**: Build complex dialogs from simple primitives
4. **Performance**: Lazy loading and optimal rendering
5. **Accessibility**: WCAG 2.1 AA compliance

## Type Definitions

### Base Types
```typescript
// Dialog identification and priority
export type DialogId = string;
export type DialogPriority = 'low' | 'normal' | 'high' | 'critical';

// Dialog types
export enum DialogType {
  Modal = 'modal',
  Wizard = 'wizard',
  Alert = 'alert',
  Confirmation = 'confirmation',
  Custom = 'custom'
}

// Base dialog configuration
export interface DialogConfig<T = unknown> {
  id: DialogId;
  type: DialogType;
  priority?: DialogPriority;
  persistent?: boolean;
  props?: T;
  onClose?: () => void;
  onComplete?: (result: any) => void;
}

// Dialog instance with runtime data
export interface DialogInstance<T = unknown> extends DialogConfig<T> {
  timestamp: number;
  status: 'pending' | 'active' | 'closing';
}
```

### Component Props
```typescript
// Base props all dialogs receive
export interface BaseDialogProps {
  isOpen: boolean;
  onClose: () => void;
  dialogId: DialogId;
}

// Modal dialog props
export interface ModalDialogProps extends BaseDialogProps {
  title?: string;
  size?: 'sm' | 'md' | 'lg' | 'xl' | 'full';
  closeOnOverlayClick?: boolean;
  closeOnEsc?: boolean;
}

// Wizard dialog props
export interface WizardDialogProps extends BaseDialogProps {
  title?: string;
  steps: WizardStep[];
  currentStep?: number;
  onStepChange?: (step: number) => void;
  onComplete?: (data: any) => void;
}

// Wizard step definition
export interface WizardStep {
  id: string;
  title: string;
  component: React.ComponentType<WizardStepProps>;
  validation?: (data: any) => boolean | Promise<boolean>;
}

// Props passed to wizard step components
export interface WizardStepProps {
  data: any;
  onNext: (data?: any) => void;
  onBack: () => void;
  isFirstStep: boolean;
  isLastStep: boolean;
}
```

## Context API

### DialogContext
```typescript
export interface DialogContextValue {
  // State
  dialogs: DialogInstance[];
  activeDialog: DialogInstance | null;
  
  // Actions
  openDialog: <T>(config: DialogConfig<T>) => DialogId;
  closeDialog: (id: DialogId) => void;
  closeAllDialogs: () => void;
  updateDialog: (id: DialogId, updates: Partial<DialogConfig>) => void;
  
  // Utilities
  isDialogOpen: (id: DialogId) => boolean;
  getDialog: (id: DialogId) => DialogInstance | undefined;
}
```

### Usage Hook
```typescript
export function useDialog(): DialogContextValue;
export function useDialogState(id: DialogId): DialogInstance | undefined;
```

## Component Architecture

### Dialog Provider
```typescript
export const DialogProvider: React.FC<{
  children: React.ReactNode;
  maxDialogs?: number;
  defaultPriority?: DialogPriority;
}> = ({ children, maxDialogs = 5, defaultPriority = 'normal' }) => {
  // Implementation
};
```

### Base Dialog Component
```typescript
export const Dialog: React.FC<ModalDialogProps> & {
  Header: React.FC<{ children: React.ReactNode }>;
  Body: React.FC<{ children: React.ReactNode }>;
  Footer: React.FC<{ children: React.ReactNode }>;
} = ({ children, ...props }) => {
  // Implementation
};
```

### Wizard Component
```typescript
export const Wizard: React.FC<WizardDialogProps> = ({
  steps,
  currentStep = 0,
  onStepChange,
  onComplete,
  ...props
}) => {
  // Implementation
};
```

## Implementation Details

### Priority Queue Algorithm
```typescript
class DialogQueue {
  private queue: DialogInstance[] = [];
  private readonly maxSize: number;
  
  constructor(maxSize: number) {
    this.maxSize = maxSize;
  }
  
  add(dialog: DialogInstance): void {
    // Insert based on priority
    const index = this.findInsertIndex(dialog.priority);
    this.queue.splice(index, 0, dialog);
    
    // Remove oldest low priority if over limit
    if (this.queue.length > this.maxSize) {
      this.removeLowPriority();
    }
  }
  
  private findInsertIndex(priority: DialogPriority): number {
    // Implementation
  }
  
  private removeLowPriority(): void {
    // Implementation
  }
}
```

### Lazy Loading
```typescript
// Dynamic imports for dialog components
const dialogComponents = {
  FirmwareUpdate: React.lazy(() => import('./dialogs/FirmwareUpdateDialog')),
  WalletCreation: React.lazy(() => import('./dialogs/WalletCreationWizard')),
  // ... other dialogs
};

// Render with Suspense
<Suspense fallback={<DialogLoader />}>
  <DynamicDialog type={dialogType} {...props} />
</Suspense>
```

### State Persistence
```typescript
interface DialogPersistence {
  save(id: DialogId, state: any): void;
  load(id: DialogId): any | null;
  clear(id: DialogId): void;
}

class LocalStorageDialogPersistence implements DialogPersistence {
  private readonly prefix = 'dialog_state_';
  
  save(id: DialogId, state: any): void {
    localStorage.setItem(this.prefix + id, JSON.stringify(state));
  }
  
  load(id: DialogId): any | null {
    const saved = localStorage.getItem(this.prefix + id);
    return saved ? JSON.parse(saved) : null;
  }
  
  clear(id: DialogId): void {
    localStorage.removeItem(this.prefix + id);
  }
}
```

## Dialog Registry

### Registration System
```typescript
interface DialogRegistration {
  type: string;
  component: React.ComponentType<any>;
  defaultProps?: any;
  validator?: (props: any) => boolean;
}

class DialogRegistry {
  private registry = new Map<string, DialogRegistration>();
  
  register(registration: DialogRegistration): void {
    this.registry.set(registration.type, registration);
  }
  
  get(type: string): DialogRegistration | undefined {
    return this.registry.get(type);
  }
  
  has(type: string): boolean {
    return this.registry.has(type);
  }
}
```

## Animation System

### Transition Configuration
```typescript
interface DialogTransition {
  enter: {
    from: MotionProps;
    to: MotionProps;
    duration?: number;
  };
  exit: {
    from: MotionProps;
    to: MotionProps;
    duration?: number;
  };
}

const defaultTransitions: Record<DialogType, DialogTransition> = {
  [DialogType.Modal]: {
    enter: {
      from: { opacity: 0, scale: 0.95 },
      to: { opacity: 1, scale: 1 },
      duration: 200
    },
    exit: {
      from: { opacity: 1, scale: 1 },
      to: { opacity: 0, scale: 0.95 },
      duration: 150
    }
  },
  // ... other types
};
```

## Error Handling

### Error Boundary
```typescript
export class DialogErrorBoundary extends React.Component<
  { children: React.ReactNode },
  { hasError: boolean; error: Error | null }
> {
  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }
  
  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('Dialog error:', error, errorInfo);
  }
  
  render() {
    if (this.state.hasError) {
      return <DialogErrorFallback error={this.state.error} />;
    }
    
    return this.props.children;
  }
}
```

### Error Recovery
```typescript
interface DialogError {
  dialogId: DialogId;
  error: Error;
  timestamp: number;
  recoverable: boolean;
}

interface ErrorRecoveryStrategy {
  canRecover(error: DialogError): boolean;
  recover(error: DialogError): void;
}
```

## Testing Strategy

### Unit Tests
```typescript
describe('DialogProvider', () => {
  it('should manage dialog queue by priority', () => {
    // Test implementation
  });
  
  it('should limit maximum dialogs', () => {
    // Test implementation
  });
  
  it('should handle dialog lifecycle', () => {
    // Test implementation
  });
});
```

### Integration Tests
```typescript
describe('Dialog System Integration', () => {
  it('should open wizard and navigate steps', () => {
    // Test implementation
  });
  
  it('should handle nested dialogs', () => {
    // Test implementation
  });
});
```

## Performance Considerations

1. **Lazy Loading**: Load dialog components on demand
2. **Memoization**: Use React.memo for dialog components
3. **Virtual DOM**: Minimize re-renders with proper keys
4. **Animation**: Use CSS transforms for better performance
5. **Bundle Size**: Code split by dialog type

## Accessibility Requirements

1. **Focus Management**: Trap focus within dialog
2. **Keyboard Navigation**: ESC to close, Tab to navigate
3. **ARIA Labels**: Proper roles and labels
4. **Screen Reader**: Announce dialog open/close
5. **High Contrast**: Support high contrast mode

## Migration Path

### Phase 1: Parallel Implementation
1. Implement new system alongside old
2. Create adapter for old dialog API
3. Test thoroughly

### Phase 2: Gradual Migration
1. Migrate one dialog at a time
2. Update imports incrementally
3. Maintain backward compatibility

### Phase 3: Cleanup
1. Remove old dialog system
2. Update all documentation
3. Remove compatibility layer