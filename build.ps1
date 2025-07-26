# KeepKey Vault v4 Build System - Windows PowerShell Edition
param([string]$Command = "help")

function Test-CommandExists($CommandName) {
    $null = Get-Command $CommandName -ErrorAction SilentlyContinue
    return $?
}

function Show-Help {
    Write-Host ""
    Write-Host "KeepKey Vault v4 Build System - Windows Edition" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage: .\build.ps1 [command]" -ForegroundColor White
    Write-Host ""
    Write-Host "Available commands:" -ForegroundColor Yellow
    Write-Host "  check-deps    - Check dependencies" -ForegroundColor White
    Write-Host "  deps          - Install dependencies" -ForegroundColor White
    Write-Host "  setup         - Initial project setup" -ForegroundColor White
    Write-Host "  vault         - Start development server" -ForegroundColor White
    Write-Host "  vault-dev     - Quick development build" -ForegroundColor White
    Write-Host "  vault-build   - Build for production" -ForegroundColor White
    Write-Host "  test          - Run tests" -ForegroundColor White
    Write-Host "  clean         - Clean build artifacts" -ForegroundColor White
    Write-Host "  help          - Show this help" -ForegroundColor White
    Write-Host ""
    Write-Host "Dependencies needed:" -ForegroundColor Yellow
    Write-Host "  - Rust/Cargo: https://rustup.rs/" -ForegroundColor White
    Write-Host "  - Node.js or Bun: https://nodejs.org/ or https://bun.sh/" -ForegroundColor White
    Write-Host "  - Tauri CLI: cargo install tauri-cli" -ForegroundColor White
    Write-Host ""
}

function Test-Dependencies {
    Write-Host "Checking dependencies..." -ForegroundColor Cyan
    
    $missing = @()
    
    if (-not (Test-CommandExists "cargo")) {
        $missing += "Rust/Cargo"
        Write-Host "ERROR: Rust/Cargo not found" -ForegroundColor Red
    } else {
        Write-Host "OK: Found Rust/Cargo" -ForegroundColor Green
    }
    
    $hasPackageManager = $false
    if (Test-CommandExists "bun") {
        Write-Host "OK: Found Bun" -ForegroundColor Green
        $hasPackageManager = $true
    } elseif (Test-CommandExists "npm") {
        Write-Host "OK: Found npm" -ForegroundColor Green  
        $hasPackageManager = $true
    } else {
        $missing += "Node.js/npm or Bun"
        Write-Host "ERROR: No package manager found" -ForegroundColor Red
    }
    
    try {
        $null = cargo tauri --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "OK: Found Tauri CLI" -ForegroundColor Green
        } else {
            throw "Tauri CLI not working"
        }
    } catch {
        $missing += "Tauri CLI"
        Write-Host "ERROR: Tauri CLI not found" -ForegroundColor Red
    }
    
    if ($missing.Count -eq 0) {
        Write-Host "SUCCESS: All dependencies found!" -ForegroundColor Green
        return $true
    } else {
        Write-Host "ERROR: Missing: $($missing -join ', ')" -ForegroundColor Red
        return $false
    }
}

function Install-Dependencies {
    Write-Host "Installing dependencies..." -ForegroundColor Magenta
    
    if (-not (Test-Path "projects/keepkey-vault")) {
        Write-Host "ERROR: Project directory not found!" -ForegroundColor Red
        return $false
    }
    
    Set-Location "projects/keepkey-vault"
    
    try {
        if (Test-CommandExists "bun") {
            Write-Host "Using Bun..." -ForegroundColor Magenta
            bun install
        } elseif (Test-CommandExists "npm") {
            Write-Host "Using npm..." -ForegroundColor Magenta
            npm install
        } else {
            Write-Host "ERROR: No package manager found!" -ForegroundColor Red
            return $false
        }
        
        Write-Host "SUCCESS: Dependencies installed" -ForegroundColor Green
        return $true
    } finally {
        Set-Location "../.."
    }
}

function Start-DevServer {
    Write-Host "Starting development server..." -ForegroundColor Yellow
    
    Set-Location "projects/keepkey-vault"
    
    try {
        if (Test-CommandExists "bun") {
            bun tauri dev
        } else {
            npm run tauri dev
        }
    } finally {
        Set-Location "../.."
    }
}

function Build-Production {
    Write-Host "Building for production..." -ForegroundColor Yellow
    
    Set-Location "projects/keepkey-vault"
    
    try {
        if (Test-CommandExists "bun") {
            bun tauri build
        } else {
            npm run tauri build
        }
        Write-Host "SUCCESS: Build complete!" -ForegroundColor Green
    } finally {
        Set-Location "../.."
    }
}

function Run-Tests {
    Write-Host "Running tests..." -ForegroundColor Yellow
    
    if (Test-Path "projects/keepkey-vault/src-tauri") {
        Set-Location "projects/keepkey-vault/src-tauri"
        cargo test
        Set-Location "../../.."
    }
    
    Set-Location "projects/keepkey-vault"
    try {
        if (Test-CommandExists "bun") {
            try { bun test } catch { Write-Host "No frontend tests" -ForegroundColor Gray }
        } else {
            try { npm test } catch { Write-Host "No frontend tests" -ForegroundColor Gray }
        }
    } finally {
        Set-Location "../.."
    }
    
    Write-Host "SUCCESS: Tests complete" -ForegroundColor Green
}

function Clear-BuildArtifacts {
    Write-Host "Cleaning build artifacts..." -ForegroundColor DarkYellow
    
    $paths = @(
        "projects/keepkey-vault/node_modules",
        "projects/keepkey-vault/dist", 
        "projects/keepkey-vault/target"
    )
    
    foreach ($path in $paths) {
        if (Test-Path $path) {
            Write-Host "Removing $path" -ForegroundColor Gray
            Remove-Item $path -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
    
    if (Test-Path "projects/keepkey-vault/src-tauri") {
        Set-Location "projects/keepkey-vault/src-tauri"
        cargo clean
        Set-Location "../../.."
    }
    
    Write-Host "SUCCESS: Cleanup complete" -ForegroundColor Green
}

# Main execution
switch ($Command.ToLower()) {
    "check-deps" {
        $result = Test-Dependencies
        if (-not $result) { exit 1 }
    }
    "deps" {
        if (-not (Test-Dependencies)) { exit 1 }
        Install-Dependencies
    }
    "setup" {
        Write-Host "Setting up project..." -ForegroundColor Yellow
        if (-not (Test-Dependencies)) { 
            Write-Host "Please install missing dependencies first" -ForegroundColor Red
            exit 1 
        }
        Install-Dependencies
        Write-Host "SUCCESS: Setup complete!" -ForegroundColor Green
    }
    "vault" {
        if (-not (Test-Dependencies)) { exit 1 }
        Install-Dependencies
        Start-DevServer
    }
    "vault-dev" {
        Start-DevServer
    }
    "vault-build" {
        if (-not (Test-Dependencies)) { exit 1 }
        Install-Dependencies
        Build-Production
    }
    "test" {
        Run-Tests
    }
    "clean" {
        Clear-BuildArtifacts
    }
    "help" {
        Show-Help
    }
    default {
        Write-Host "ERROR: Unknown command: $Command" -ForegroundColor Red
        Show-Help
        exit 1
    }
} 