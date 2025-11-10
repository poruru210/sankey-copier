# SANKEY Copier Build Script for Windows
# This script builds all components required for the installer
# Run this script from the installer directory

param(
    [switch]$SkipRust = $false,
    [switch]$SkipWebUI = $false,
    [switch]$SkipMQL = $false,
    [switch]$SkipTray = $false,
    [switch]$Clean = $false
)

$ErrorActionPreference = "Stop"
$ProgressPreference = 'SilentlyContinue'

# Colors for output
function Write-Info { param($Message) Write-Host "[INFO] $Message" -ForegroundColor Cyan }
function Write-Success { param($Message) Write-Host "[SUCCESS] $Message" -ForegroundColor Green }
function Write-Error { param($Message) Write-Host "[ERROR] $Message" -ForegroundColor Red }
function Write-Warning { param($Message) Write-Host "[WARNING] $Message" -ForegroundColor Yellow }

# Get project root directory (parent of installer)
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

Write-Info "Project Root: $ProjectRoot"
Write-Info "Installer Directory: $ScriptDir"

# Check prerequisites
function Test-Prerequisites {
    Write-Info "Checking prerequisites..."

    # Check Rust
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "Rust is not installed. Please install from https://rustup.rs/"
        exit 1
    }
    Write-Success "Rust: $(cargo --version)"

    # Check Node.js
    if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
        Write-Error "Node.js is not installed. Please install from https://nodejs.org/"
        exit 1
    }
    Write-Success "Node.js: $(node --version)"

    # Check npm
    if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
        Write-Error "npm is not installed."
        exit 1
    }
    Write-Success "npm: $(npm --version)"

    # Check for NSSM
    $nssmPath = Join-Path $ScriptDir "resources\nssm.exe"
    if (-not (Test-Path $nssmPath)) {
        Write-Warning "NSSM not found at: $nssmPath"
        Write-Warning "Please download NSSM from https://nssm.cc/download"
        Write-Warning "Extract and copy nssm.exe to installer\resources\"

        $response = Read-Host "Do you want to continue without NSSM? (y/N)"
        if ($response -ne "y" -and $response -ne "Y") {
            exit 1
        }
    } else {
        Write-Success "NSSM found"
    }
}

# Clean previous builds
function Invoke-Clean {
    Write-Info "Cleaning previous builds..."

    # Clean Rust
    if (Test-Path "$ProjectRoot\rust-server\target") {
        Write-Info "Cleaning Rust build..."
        Push-Location "$ProjectRoot\rust-server"
        cargo clean
        Pop-Location
    }

    # Clean Next.js
    if (Test-Path "$ProjectRoot\web-ui\.next") {
        Write-Info "Cleaning Next.js build..."
        Remove-Item "$ProjectRoot\web-ui\.next" -Recurse -Force
    }
    if (Test-Path "$ProjectRoot\web-ui\out") {
        Remove-Item "$ProjectRoot\web-ui\out" -Recurse -Force
    }

    # Clean MQL DLL
    if (Test-Path "$ProjectRoot\mql-zmq-dll\target") {
        Write-Info "Cleaning MQL DLL build..."
        Push-Location "$ProjectRoot\mql-zmq-dll"
        cargo clean
        Pop-Location
    }

    # Clean Tray App
    if (Test-Path "$ProjectRoot\sankey-copier-tray\target") {
        Write-Info "Cleaning Tray App build..."
        Push-Location "$ProjectRoot\sankey-copier-tray"
        cargo clean
        Pop-Location
    }

    Write-Success "Clean completed"
}

# Build Rust server
function Build-RustServer {
    Write-Info "Building Rust server..."

    Push-Location "$ProjectRoot\rust-server"

    try {
        Write-Info "Running cargo build --release..."
        cargo build --release

        $exePath = "target\release\sankey-copier-server.exe"
        if (-not (Test-Path $exePath)) {
            Write-Error "Build failed: $exePath not found"
            exit 1
        }

        $fileInfo = Get-Item $exePath
        $sizeInMB = [math]::Round($fileInfo.Length / 1MB, 2)
        Write-Success "Rust server built successfully ($sizeInMB MB)"
        Write-Info "Location: $exePath"
    }
    finally {
        Pop-Location
    }
}

# Build Next.js Web UI
function Build-WebUI {
    Write-Info "Building Next.js Web UI..."

    Push-Location "$ProjectRoot\web-ui"

    try {
        # Install dependencies if node_modules doesn't exist
        if (-not (Test-Path "node_modules")) {
            Write-Info "Installing dependencies..."
            npm install
        }

        # Build Next.js in standalone mode
        Write-Info "Building Next.js standalone..."
        $env:NEXT_BUILD_STANDALONE = "true"
        npm run build

        if (-not (Test-Path ".next\standalone")) {
            Write-Error "Build failed: .next\standalone not found"
            Write-Info "Make sure next.config.js has output: 'standalone'"
            exit 1
        }

        Write-Success "Next.js Web UI built successfully"
        Write-Info "Location: .next\standalone"
    }
    finally {
        Pop-Location
    }
}

# Build MQL ZMQ DLL
function Build-MQLDLL {
    Write-Info "Building MQL ZMQ DLL..."

    Push-Location "$ProjectRoot\mql-zmq-dll"

    try {
        # Build 64-bit DLL
        Write-Info "Building 64-bit DLL..."
        cargo build --release

        if (-not (Test-Path "target\release\sankey_copier_zmq.dll")) {
            Write-Error "64-bit build failed"
            exit 1
        }
        Write-Success "64-bit DLL built"

        # Build 32-bit DLL
        Write-Info "Building 32-bit DLL..."
        rustup target add i686-pc-windows-msvc
        cargo build --release --target i686-pc-windows-msvc

        if (-not (Test-Path "target\i686-pc-windows-msvc\release\sankey_copier_zmq.dll")) {
            Write-Error "32-bit build failed"
            exit 1
        }
        Write-Success "32-bit DLL built"

        Write-Success "MQL ZMQ DLL built successfully (32-bit and 64-bit)"
    }
    finally {
        Pop-Location
    }
}

# Build Tray Application
function Build-TrayApp {
    Write-Info "Building Tray Application..."

    Push-Location "$ProjectRoot\sankey-copier-tray"

    try {
        Write-Info "Running cargo build --release..."
        cargo build --release

        $exePath = "target\release\sankey-copier-tray.exe"
        if (-not (Test-Path $exePath)) {
            Write-Error "Build failed: $exePath not found"
            exit 1
        }

        $fileInfo = Get-Item $exePath
        $sizeInKB = [math]::Round($fileInfo.Length / 1KB, 2)
        Write-Success "Tray application built successfully ($sizeInKB KB)"
        Write-Info "Location: $exePath"
    }
    finally {
        Pop-Location
    }
}

# Check Next.js configuration
function Test-NextJSConfig {
    $configPath = "$ProjectRoot\web-ui\next.config.js"
    if (Test-Path $configPath) {
        $config = Get-Content $configPath -Raw
        if ($config -notmatch "output:\s*['""]standalone['""]") {
            Write-Warning "next.config.js may not have standalone output configured"
            Write-Info "The installer requires standalone mode for Next.js"
        }
    }
}

# Create resource files if missing
function Initialize-Resources {
    $resourcesDir = Join-Path $ScriptDir "resources"

    # Create icon if missing
    $iconPath = Join-Path $resourcesDir "icon.ico"
    if (-not (Test-Path $iconPath)) {
        Write-Warning "Icon file not found: $iconPath"
        Write-Info "Using default icon"
    }

    # Create license file if missing
    $licensePath = Join-Path $resourcesDir "license.txt"
    if (-not (Test-Path $licensePath)) {
        Write-Info "Creating default license file..."
        @"
SANKEY Copier License

Copyright (c) 2024 SANKEY Copier Project

This software is provided for evaluation purposes.
See the full license at: https://github.com/your-org/sankey-copier/blob/main/LICENSE
"@ | Out-File -FilePath $licensePath -Encoding UTF8
    }
}

# Main execution
Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  SANKEY Copier Build Script" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# Check prerequisites
Test-Prerequisites

# Initialize resources
Initialize-Resources

# Clean if requested
if ($Clean) {
    Invoke-Clean
}

# Build components
$buildStart = Get-Date

if (-not $SkipRust) {
    Build-RustServer
} else {
    Write-Warning "Skipping Rust server build"
}

if (-not $SkipWebUI) {
    Test-NextJSConfig
    Build-WebUI
} else {
    Write-Warning "Skipping Web UI build"
}

if (-not $SkipMQL) {
    Build-MQLDLL
} else {
    Write-Warning "Skipping MQL DLL build"
}

if (-not $SkipTray) {
    Build-TrayApp
} else {
    Write-Warning "Skipping Tray Application build"
}

$buildEnd = Get-Date
$buildDuration = $buildEnd - $buildStart

Write-Host ""
Write-Host "================================================" -ForegroundColor Green
Write-Host "  Build Completed Successfully!" -ForegroundColor Green
Write-Host "================================================" -ForegroundColor Green
Write-Host "Build time: $($buildDuration.TotalMinutes.ToString("F2")) minutes" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "1. Verify NSSM is in installer\resources\nssm.exe" -ForegroundColor White
Write-Host "2. Open installer\setup.iss in Inno Setup Compiler" -ForegroundColor White
Write-Host "3. Click 'Compile' to create the installer" -ForegroundColor White
Write-Host "4. Installer will be created in installer\Output\" -ForegroundColor White
Write-Host ""
