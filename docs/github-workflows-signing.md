# GitHub Workflows - Signing Configuration

This document explains how the GitHub workflows handle signing credentials and allow for both signed and unsigned releases.

## Overview

The GitHub workflows have been updated to gracefully handle missing signing credentials, allowing the project to build and release even without proper signing certificates configured. This is particularly useful for:

- Development and testing environments
- Open source contributions
- CI/CD pipelines where signing is not immediately available

## Workflow Files

### 1. Build Workflow (`.github/workflows/build.yml`)

**Triggers:**
- Push to `main` or `master` branches
- Pull requests to `main` or `master` branches
- Manual workflow dispatch

**Features:**
- Builds the Vault v2 app for all platforms (macOS, Linux, Windows)
- Installs required dependencies including Protocol Buffers compiler
- Checks for signing credentials before building
- Creates draft releases with appropriate security warnings
- Provides clear feedback about signing status

### 2. Release Workflow (`.github/workflows/release.yml`)

**Triggers:**
- Push of version tags (e.g., `v1.0.0`)
- Manual workflow dispatch

**Features:**
- Creates and publishes GitHub releases
- Builds signed or unsigned releases based on available credentials
- Updates release notes with signing status
- Provides clear security warnings for unsigned releases

## Signing Credentials

The workflows check for the following signing credentials:

### Tauri Signing (All Platforms)
- `TAURI_PRIVATE_KEY`: Base64-encoded Tauri signing key
- `TAURI_KEY_PASSWORD`: Password for the Tauri signing key

### Apple Signing (macOS Only)
- `APPLE_ID`: Apple developer account email
- `APPLE_PASSWORD`: Apple app-specific password
- `APPLE_TEAM_ID`: Apple developer team ID
- `MACOS_CERTIFICATE_BASE64`: Base64-encoded Apple certificate
- `MACOS_CERTIFICATE_PASSWORD`: Apple certificate password
- `CODESIGN_IDENTITY`: Apple code signing identity
- `KEYCHAIN_NAME`: macOS keychain name
- `KEYCHAIN_PASSWORD`: macOS keychain password

## Behavior Matrix

| Signing Credentials | Build Result | Release Notes | Security Warning |
|---------------------|--------------|---------------|------------------|
| ✅ All Present | Signed build | "All releases are signed and verified" | None |
| ⚠️ Partial | Partially signed | Mixed status shown | Warnings for unsigned components |
| ❌ None | Unsigned build | "This release is NOT signed" | Clear development-only warning |

## Setting Up Signing

### For Development/Testing
No action needed - workflows will build unsigned releases automatically.

### For Production
Add the required secrets to your GitHub repository:

1. Go to your repository settings
2. Navigate to "Secrets and variables" → "Actions"
3. Add the required secrets listed above

### Generating Tauri Signing Keys

```bash
# Generate a new Tauri signing key
tauri signer generate -w ~/.tauri/myapp.key

# Convert to base64 for GitHub secrets
base64 -i ~/.tauri/myapp.key | pbcopy
```

## Workflow Outputs

### Build Summary
Each build generates a summary showing:
- Platform information
- Signing status for each component
- Warnings for unsigned builds

### Release Notes
Releases automatically include:
- Feature descriptions
- Download instructions
- Security status (signed/unsigned)
- Build information
- Development build warnings (if applicable)

## Security Considerations

### Unsigned Releases
- Marked with clear warnings
- Recommended for development only
- Users must acknowledge risks
- Not suitable for production distribution

### Signed Releases
- Verified by platform security systems
- Safe for production distribution
- Recommended for all public releases
- Provide user confidence

## Troubleshooting

### Build Fails Due to Missing Dependencies
- **Protocol Buffers Error**: The workflows automatically install `protoc` (Protocol Buffers compiler)
- **Platform-specific dependencies**: Each platform installs its required dependencies automatically
- Check the dependency installation steps in the workflow logs if builds fail

### Build Fails Due to Missing Credentials
- Check the "Check signing credentials" step in the workflow logs
- Verify all required secrets are set correctly
- Ensure base64 encoding is correct for certificate data

### Partial Signing
- Review which credentials are missing
- macOS builds can work with just Tauri signing
- Apple notarization requires all Apple credentials

### Release Notes Not Updated
- Check the "Update release notes" step
- Verify GitHub token has sufficient permissions
- Ensure release was created successfully

## Best Practices

1. **Development**: Use unsigned builds for testing
2. **Staging**: Test with signed builds before production
3. **Production**: Always use fully signed releases
4. **Security**: Rotate signing keys regularly
5. **Documentation**: Keep this guide updated with changes

## Migration Guide

If you're migrating from the old workflows:

1. The workflows now handle missing credentials gracefully
2. No changes needed for existing signed setups
3. New repositories work out-of-the-box without signing
4. Add signing credentials when ready for production

## Support

For issues with the workflows:
1. Check the GitHub Actions logs
2. Review the signing credential status
3. Verify all required secrets are configured
4. Consult the Tauri documentation for platform-specific requirements 