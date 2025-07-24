# KeepKey Vault Documentation

This directory contains architectural documentation, planning documents, and cleanup guides for the KeepKey Vault application.

## Directory Structure

### `/architecture`
Technical documentation about the current and proposed system architecture.

- `current-dialog-system.md` - Analysis of the existing dialog system
- `dialog-system-spec.md` - Technical specification for the new dialog system

### `/planning`
Planning documents for upcoming features and refactoring efforts.

- `dialog-system-reorganization.md` - Comprehensive plan for dialog system refactoring

### `/cleanup`
Checklists and guides for code cleanup and technical debt reduction.

- `dialog-cleanup-checklist.md` - Step-by-step checklist for dialog system cleanup

## Current Focus: Dialog System Reorganization

We are currently planning a major refactoring of the dialog system to address:

1. **Type Safety**: Remove all `any` types and add proper TypeScript definitions
2. **Organization**: Create a clear, maintainable structure for dialog components
3. **Consistency**: Implement consistent patterns across all dialogs
4. **Performance**: Optimize rendering and bundle size
5. **User Experience**: Improve animations, accessibility, and error handling

## Quick Links

- [Dialog System Reorganization Plan](./planning/dialog-system-reorganization.md)
- [Current System Analysis](./architecture/current-dialog-system.md)
- [Technical Specification](./architecture/dialog-system-spec.md)
- [Cleanup Checklist](./cleanup/dialog-cleanup-checklist.md)

## Contributing

When adding new documentation:

1. Place architectural docs in `/architecture`
2. Place planning docs in `/planning`
3. Place cleanup guides in `/cleanup`
4. Update this README with new documents
5. Use clear, descriptive filenames
6. Include a table of contents for long documents

## Document Standards

- Use Markdown format
- Include clear headings and structure
- Add code examples where appropriate
- Keep documents focused on a single topic
- Update documents as implementation progresses