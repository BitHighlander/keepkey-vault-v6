# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

### Core Development Commands
```bash
# Build and run in development mode (recommended for development)
make vault

# Quick development build (skips dependency checks)
make vault-dev

# Production build (requires Apple notarization credentials on macOS)
make vault-build

# Run tests
make test

# Clean build artifacts
make clean        # Clean all including dependencies
make clean-build  # Clean only build outputs (keep dependencies)
```

### Backend-specific Commands (Rust/Tauri)
```bash
# Run Rust tests
cd projects/keepkey-vault/src-tauri && cargo test

# Check compilation
cd projects/keepkey-vault/src-tauri && cargo check

# Run linter
cd projects/keepkey-vault/src-tauri && cargo clippy
```

### Frontend-specific Commands (React/TypeScript)
```bash
# Install dependencies (uses Bun if available, falls back to npm)
cd projects/keepkey-vault && bun install

# Type checking
cd projects/keepkey-vault && tsc --noEmit
```

## Architecture Overview

### Project Structure
This is a monorepo containing multiple projects:
- `projects/keepkey-vault/` - Main Tauri + React application
- `projects/keepkey-usb/` - Core Rust library for USB device communication (`keepkey_rust` crate)

### Backend Architecture (Rust/Tauri)

The backend currently has architectural issues documented in `BACKEND_REFACTOR_PLAN.md`:

1. **Missing Modules**: The code references `logging`, `cache`, and `event_controller` modules that don't exist
2. **Monolithic Structure**: `commands.rs` contains 4,384 lines with 53+ command handlers
3. **Circular Dependencies**: Commands and device modules have circular references

**Current Flow**:
```
Frontend → Tauri Command (commands.rs) → Device Queue → keepkey_rust library
```

**Key Components**:
- `src-tauri/src/lib.rs` - Application setup, USB monitoring, device queue management
- `src-tauri/src/commands.rs` - All Tauri command handlers (needs refactoring)
- `keepkey_rust` crate - Core USB communication and device management

### Frontend Architecture (React/TypeScript)

**Key Components**:
- Dialog system for device interactions (PIN, firmware updates, etc.)
- Context providers for state management (DialogContext, BlockingActionsContext)
- Wizard components for complex flows (onboarding, recovery, updates)

## Development Philosophy

The project follows a "Fail Fast, Fail Forward" philosophy as documented in the existing CLAUDE.md:

1. **Fail Fast**: Test critical assumptions first before building complex features
2. **Fail Forward**: Build reusable tools and services, not one-off scripts
3. **Make Requirements Less Dumb**: Question all requirements and simplify aggressively

## Current State and Priorities

### Critical Issues to Fix
1. **Backend won't compile** - Missing modules need to be restored or stubbed
2. **Circular dependencies** - Need architectural refactoring per `BACKEND_REFACTOR_PLAN.md`

### Active Work
- Backend refactor is in progress on the `backend-refactor` branch
- Goal is to separate concerns, reduce code duplication, and improve maintainability

## USB Device Communication

The project uses the `keepkey_rust` library which provides:
- Multi-device queue management
- HID and WebUSB transport layers
- Protocol buffer message handling
- Firmware update capabilities

Device detection and event handling is managed through USB monitoring in `lib.rs`.

## Testing Approach

1. **Unit Tests**: Run with `cargo test` in the Rust backend
2. **Integration Tests**: Test device communication with actual hardware
3. **Manual Testing**: Use the development build (`make vault`) for end-to-end testing

## Important Notes

- The project is transitioning from v5 to v6 with significant architectural changes
- Some code is duplicated between the queue system and command handlers
- The device module appears disconnected from the main application flow
- Production builds on macOS require Apple notarization credentials