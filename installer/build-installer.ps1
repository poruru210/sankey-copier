# SANKEY Copier Windows Installer Build Script
# Builds all components in the correct order and creates installer package

param(
    [string]$Version = "1.0.0",
    [switch]$SkipTests = $false,
    [switch]$Verbose = $false
)

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "SANKEY Copier Installer Build Script" -ForegroundColor Cyan
Write-Host "Version: $Version" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Step 1: Build Rust Server
Write-Host "[1/7] Building Rust Server..." -ForegroundColor Yellow
Set-Location "$RootDir\rust-server"
if (-not $SkipTests) {
    Write-Host "  Running tests..." -ForegroundColor Gray
    cargo test --release
    if ($LASTEXITCODE -ne 0) { throw "Rust server tests failed" }
}
Write-Host "  Building release binary..." -ForegroundColor Gray
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "Rust server build failed" }
Write-Host "  ✓ Rust server built successfully" -ForegroundColor Green
Write-Host ""

# Step 2: Build MQL ZMQ DLL (64-bit for MT5)
Write-Host "[2/7] Building MQL ZMQ DLL (64-bit)..." -ForegroundColor Yellow
Set-Location "$RootDir\mql-zmq-dll"
Write-Host "  Building x64 release..." -ForegroundColor Gray
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "MQL ZMQ DLL (64-bit) build failed" }
Write-Host "  ✓ 64-bit DLL built successfully" -ForegroundColor Green
Write-Host ""

# Step 3: Build MQL ZMQ DLL (32-bit for MT4)
Write-Host "[3/7] Building MQL ZMQ DLL (32-bit)..." -ForegroundColor Yellow
Write-Host "  Building i686 release..." -ForegroundColor Gray
cargo build --release --target i686-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "MQL ZMQ DLL (32-bit) build failed" }
Write-Host "  ✓ 32-bit DLL built successfully" -ForegroundColor Green
Write-Host ""

# Step 4: Build System Tray Application
Write-Host "[4/7] Building System Tray Application..." -ForegroundColor Yellow
Set-Location "$RootDir\sankey-copier-tray"
Write-Host "  Building release binary..." -ForegroundColor Gray
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "Tray application build failed" }
Write-Host "  ✓ Tray application built successfully" -ForegroundColor Green
Write-Host ""

# Step 5: Build Web UI (Next.js standalone)
Write-Host "[5/7] Building Web UI (Next.js standalone)..." -ForegroundColor Yellow
Set-Location "$RootDir\web-ui"
Write-Host "  Installing dependencies..." -ForegroundColor Gray
pnpm install
if ($LASTEXITCODE -ne 0) { throw "Web UI dependency installation failed" }
Write-Host "  Building Next.js production build..." -ForegroundColor Gray
pnpm run build
if ($LASTEXITCODE -ne 0) { throw "Web UI build failed" }

# Verify standalone build exists
if (-not (Test-Path ".next\standalone")) {
    throw "Web UI standalone build was not created. Check next.config.ts for 'output: standalone' setting."
}
Write-Host "  ✓ Web UI built successfully (standalone mode)" -ForegroundColor Green
Write-Host ""

# Step 6: Verify all required files
Write-Host "[6/7] Verifying build outputs..." -ForegroundColor Yellow
$RequiredFiles = @(
    "$RootDir\rust-server\target\release\sankey-copier-server.exe",
    "$RootDir\sankey-copier-tray\target\release\sankey-copier-tray.exe",
    "$RootDir\mql-zmq-dll\target\release\sankey_copier_zmq.dll",
    "$RootDir\mql-zmq-dll\target\i686-pc-windows-msvc\release\sankey_copier_zmq.dll",
    "$RootDir\web-ui\.next\standalone\server.js",
    "$RootDir\web-ui\.next\standalone\package.json"
)

$AllFilesExist = $true
foreach ($File in $RequiredFiles) {
    if (Test-Path $File) {
        Write-Host "  ✓ $($File.Replace($RootDir + '\', ''))" -ForegroundColor Green
    } else {
        Write-Host "  ✗ MISSING: $($File.Replace($RootDir + '\', ''))" -ForegroundColor Red
        $AllFilesExist = $false
    }
}

if (-not $AllFilesExist) {
    throw "Some required files are missing. Build cannot continue."
}
Write-Host ""

# Step 7: Build Installer with Inno Setup
Write-Host "[7/7] Building Windows Installer..." -ForegroundColor Yellow
Set-Location "$RootDir\installer"

# Find Inno Setup Compiler
$IsccPaths = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles}\Inno Setup 6\ISCC.exe",
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe"
)

$IsccPath = $null
foreach ($Path in $IsccPaths) {
    if (Test-Path $Path) {
        $IsccPath = $Path
        break
    }
}

if (-not $IsccPath) {
    throw "Inno Setup Compiler (ISCC.exe) not found. Please install Inno Setup 6.2.2 or later from https://jrsoftware.org/isinfo.php"
}

Write-Host "  Using ISCC: $IsccPath" -ForegroundColor Gray
Write-Host "  Compiling installer script..." -ForegroundColor Gray

# Run Inno Setup Compiler with version parameter
& $IsccPath "/DMyAppVersion=$Version" "setup.iss"
if ($LASTEXITCODE -ne 0) { throw "Installer compilation failed" }

# Verify installer output
$InstallerFile = "Output\SankeyCopierSetup-$Version.exe"
if (-not (Test-Path $InstallerFile)) {
    throw "Installer file was not created: $InstallerFile"
}

$InstallerSize = (Get-Item $InstallerFile).Length / 1MB
Write-Host "  ✓ Installer created: $InstallerFile" -ForegroundColor Green
Write-Host "  Size: $([math]::Round($InstallerSize, 2)) MB" -ForegroundColor Gray
Write-Host ""

# Success summary
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "BUILD COMPLETED SUCCESSFULLY!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Installer Location:" -ForegroundColor White
Write-Host "  $RootDir\installer\$InstallerFile" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next Steps:" -ForegroundColor White
Write-Host "  1. Test the installer on a clean Windows system" -ForegroundColor Gray
Write-Host "  2. Verify all services start correctly" -ForegroundColor Gray
Write-Host "  3. Check Web UI is accessible at http://localhost:8080" -ForegroundColor Gray
Write-Host "  4. Test tray application functionality" -ForegroundColor Gray
Write-Host ""
