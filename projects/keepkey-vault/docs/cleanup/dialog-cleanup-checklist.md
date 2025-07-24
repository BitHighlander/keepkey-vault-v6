# Dialog System Cleanup Checklist

## Pre-Cleanup Tasks

- [ ] Create comprehensive backup/branch
- [ ] Document current functionality
- [ ] Identify all dialog usage points
- [ ] Create test scenarios

## Phase 1: Type Safety Improvements

### Remove `any` Types
- [ ] Replace `props: any` with proper interfaces in:
  - [ ] KeepKeyDeviceList.tsx
  - [ ] SeedVerificationWizard.tsx
  - [ ] BootloaderUpdateWizard.tsx
  - [ ] FirmwareUpdateWizard.tsx
  - [ ] TroubleshootingWizard.tsx
  - [ ] OnboardingWizard.tsx
  - [ ] DeviceInvalidStateDialog.tsx

### Create Type Definitions
- [ ] DialogProps base interface
- [ ] WizardProps interface
- [ ] ModalProps interface
- [ ] Specific props for each dialog

### Enable Strict TypeScript
- [ ] Re-enable strict mode in tsconfig.json
- [ ] Fix all TypeScript errors
- [ ] Enable noUnusedLocals
- [ ] Enable noUnusedParameters

## Phase 2: Component Organization

### Create New Structure
- [ ] Create `src/dialogs` directory
- [ ] Create subdirectories:
  - [ ] core/
  - [ ] components/
  - [ ] implementations/

### Move Components
- [ ] Move dialog components to new structure
- [ ] Update all imports
- [ ] Remove old component locations

### Standardize Naming
- [ ] Rename components for consistency
- [ ] Use consistent file naming pattern
- [ ] Update all references

## Phase 3: Implementation Cleanup

### Complete Stub Implementations
- [ ] BootloaderUpdateWizard
- [ ] FirmwareUpdateWizard
- [ ] DeviceInvalidStateDialog
- [ ] OnboardingWizard
- [ ] TroubleshootingWizard
- [ ] SeedVerificationWizard
- [ ] KeepKeyDeviceList

### Remove Duplicate Code
- [ ] Identify common patterns
- [ ] Create shared components
- [ ] Remove redundancy

### Fix State Management
- [ ] Centralize dialog state
- [ ] Remove local state management
- [ ] Implement consistent patterns

## Phase 4: UI/UX Improvements

### Chakra UI Integration
- [ ] Complete Chakra UI v3 migration
- [ ] Remove custom modal implementations
- [ ] Use Chakra theming consistently

### Accessibility
- [ ] Add ARIA labels
- [ ] Implement keyboard navigation
- [ ] Add focus management
- [ ] Test with screen readers

### Animations
- [ ] Add enter/exit animations
- [ ] Implement loading states
- [ ] Add transitions between wizard steps

## Phase 5: Testing & Documentation

### Add Tests
- [ ] Unit tests for each dialog
- [ ] Integration tests for dialog system
- [ ] E2E tests for critical flows

### Documentation
- [ ] API documentation
- [ ] Usage examples
- [ ] Migration guide
- [ ] Best practices

## Phase 6: Performance Optimization

### Code Splitting
- [ ] Lazy load dialog components
- [ ] Optimize bundle size
- [ ] Remove unused code

### Rendering Optimization
- [ ] Implement React.memo where appropriate
- [ ] Optimize re-renders
- [ ] Profile performance

## Phase 7: Final Cleanup

### Remove Old Code
- [ ] Delete unused components
- [ ] Remove commented code
- [ ] Clean up imports

### Code Quality
- [ ] Run linter
- [ ] Format code
- [ ] Add code comments where needed

## Validation Checklist

### Functionality
- [ ] All dialogs open correctly
- [ ] All dialogs close correctly
- [ ] Priority queue works
- [ ] Nested dialogs work
- [ ] Error handling works

### Performance
- [ ] No performance regression
- [ ] Bundle size acceptable
- [ ] Memory usage normal

### User Experience
- [ ] Smooth animations
- [ ] Responsive design
- [ ] Accessible
- [ ] Consistent styling

## Sign-off

- [ ] Code review completed
- [ ] Testing completed
- [ ] Documentation updated
- [ ] Team approval received
- [ ] Deployed successfully

## Notes

### Known Issues to Address
1. Mixed dialog management approaches
2. Inconsistent prop passing
3. No standard error handling
4. Limited accessibility support
5. Performance concerns with large forms

### Dependencies to Update
- Consider upgrading Chakra UI
- Review React 18 features to use
- TypeScript strict mode compliance

### Future Enhancements
1. Dialog presets/templates
2. Advanced animation system
3. Dialog state persistence
4. Analytics integration
5. A/B testing support