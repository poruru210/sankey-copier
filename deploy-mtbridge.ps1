#Requires -Version 5.1
<#
.SYNOPSIS
    Builds and deploys SANKEY Copier components to installation directory.
    
.DESCRIPTION
    This script:
    1. Builds sankey-copier-mt-bridge in release mode (DLL)
    2. Compiles MQL5/MQL4 EAs using MetaEditor
    3. Copies all components to "C:\Program Files\SANKEY Copier"
    
    Requires administrator privileges to copy to Program Files.
    
.EXAMPLE
    .\deploy-mtbridge.ps1
#>

# Self-elevate to admin if not already
if (-NOT ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")) {
    Write-Host "Requesting administrator privileges..." -ForegroundColor Yellow
    Start-Process PowerShell -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`"" -Verb RunAs
    exit
}

$ErrorActionPreference = "Stop"

# Configuration
$ProjectRoot = Split-Path -Parent $PSCommandPath
$TargetDir = "C:\Program Files\SANKEY Copier"
$MetaEditor5 = "C:\Program Files\XMTrading MT5\MetaEditor64.exe"
# MetaEditor4 path - adjust if needed
$MetaEditor4 = "C:\Program Files (x86)\XMTrading MT4\metaeditor.exe"

Write-Host "=== SANKEY Copier Deploy Script ===" -ForegroundColor Cyan
Write-Host ""

# Ask about clean build
$cleanBuild = Read-Host "Perform clean build? (y/N)"
$doClean = $cleanBuild -eq 'y' -or $cleanBuild -eq 'Y'

# Step 1: Build Rust components (release)
if ($doClean) {
    Write-Host "[1/5] Cleaning and rebuilding Rust components..." -ForegroundColor Green
    Push-Location $ProjectRoot
    try {
        # Delete DLL directly to force rebuild
        $dllPath = Join-Path $ProjectRoot "target\release\sankey_copier_zmq.dll"
        if (Test-Path $dllPath) { 
            Remove-Item $dllPath -Force
            Write-Host "      Deleted existing DLL" -ForegroundColor Gray
        }
        # Delete relay-server exe
        $serverPath = Join-Path $ProjectRoot "target\release\sankey-copier-server.exe"
        if (Test-Path $serverPath) { 
            Remove-Item $serverPath -Force
            Write-Host "      Deleted existing relay-server" -ForegroundColor Gray
        }
        # Also delete cached files to force full recompile
        Get-ChildItem "target\release" -Filter "sankey_copier*" -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue
        Get-ChildItem "target\release\deps" -Filter "*sankey_copier*" -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue
        Get-ChildItem "target\release\deps" -Filter "*relay_server*" -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue
        
        cargo build --release -p sankey-copier-mt-bridge -p sankey-copier-relay-server
        if ($LASTEXITCODE -ne 0) {
            throw "Cargo build failed with exit code $LASTEXITCODE"
        }
        Write-Host "      Clean build successful!" -ForegroundColor Green
    } finally {
        Pop-Location
    }
} else {
    Write-Host "[1/5] Building Rust components in release mode..." -ForegroundColor Green
    Push-Location $ProjectRoot
    try {
        cargo build --release -p sankey-copier-mt-bridge -p sankey-copier-relay-server
        if ($LASTEXITCODE -ne 0) {
            throw "Cargo build failed with exit code $LASTEXITCODE"
        }
        Write-Host "      Build successful!" -ForegroundColor Green
    } finally {
        Pop-Location
    }
}

# Step 2: Compile MQL5 EAs
Write-Host "[2/5] Compiling MQL5 EAs..." -ForegroundColor Green
$MqlInclude = Join-Path $ProjectRoot "mt-advisors\Include"

if (Test-Path $MetaEditor5) {
    $mq5Files = @(
        "mt-advisors\MT5\SankeyCopierMaster.mq5",
        "mt-advisors\MT5\SankeyCopierSlave.mq5"
    )
    foreach ($mq5 in $mq5Files) {
        $fullPath = Join-Path $ProjectRoot $mq5
        Write-Host "      Compiling: $mq5" -ForegroundColor Gray
        & $MetaEditor5 "/compile:$fullPath" "/include:$MqlInclude" "/log" 2>&1 | Out-Null
    }
    Write-Host "      MQL5 compilation complete!" -ForegroundColor Green
} else {
    Write-Host "      MetaEditor5 not found, skipping MQL5 compilation" -ForegroundColor Yellow
}

# Step 3: Compile MQL4 EAs
Write-Host "[3/5] Compiling MQL4 EAs..." -ForegroundColor Green
if (Test-Path $MetaEditor4) {
    $mq4Files = @(
        "mt-advisors\MT4\SankeyCopierMaster.mq4",
        "mt-advisors\MT4\SankeyCopierSlave.mq4"
    )
    foreach ($mq4 in $mq4Files) {
        $fullPath = Join-Path $ProjectRoot $mq4
        Write-Host "      Compiling: $mq4" -ForegroundColor Gray
        & $MetaEditor4 "/compile:$fullPath" "/include:$MqlInclude" "/log" 2>&1 | Out-Null
    }
    Write-Host "      MQL4 compilation complete!" -ForegroundColor Green
} else {
    Write-Host "      MetaEditor4 not found, skipping MQL4 compilation" -ForegroundColor Yellow
}

# Step 4: Copy files to installation directory
Write-Host "[4/5] Copying to installation directory..." -ForegroundColor Green

# DLL paths
$DllSource = Join-Path $ProjectRoot "target\release\sankey_copier_zmq.dll"
if (-not (Test-Path $DllSource)) {
    throw "DLL not found at: $DllSource"
}

# Relay server exe
$ServerSource = Join-Path $ProjectRoot "target\release\sankey-copier-server.exe"
if (Test-Path $ServerSource) {
    Copy-Item -Path $ServerSource -Destination (Join-Path $TargetDir "sankey-copier-server.exe") -Force
    Write-Host "      relay-server -> sankey-copier-server.exe" -ForegroundColor Gray
} else {
    Write-Host "      WARNING: relay-server.exe not found, skipping" -ForegroundColor Yellow
}

# Copy DLL to MT5\Libraries (inside mt-advisors)
$Mt5LibDir = Join-Path $TargetDir "mt-advisors\MT5\Libraries"
if (-not (Test-Path $Mt5LibDir)) { New-Item -ItemType Directory -Path $Mt5LibDir -Force | Out-Null }
Copy-Item -Path $DllSource -Destination (Join-Path $Mt5LibDir "sankey_copier_zmq.dll") -Force
Write-Host "      DLL -> mt-advisors\MT5\Libraries\" -ForegroundColor Gray

# Copy DLL to MT4\Libraries (inside mt-advisors)
$Mt4LibDir = Join-Path $TargetDir "mt-advisors\MT4\Libraries"
if (-not (Test-Path $Mt4LibDir)) { New-Item -ItemType Directory -Path $Mt4LibDir -Force | Out-Null }
Copy-Item -Path $DllSource -Destination (Join-Path $Mt4LibDir "sankey_copier_zmq.dll") -Force
Write-Host "      DLL -> mt-advisors\MT4\Libraries\" -ForegroundColor Gray

# Copy compiled .ex5 files
$Mt5ExpDir = Join-Path $TargetDir "mt-advisors\MT5\Experts"
if (-not (Test-Path $Mt5ExpDir)) { New-Item -ItemType Directory -Path $Mt5ExpDir -Force | Out-Null }
$ex5Master = Join-Path $ProjectRoot "mt-advisors\MT5\SankeyCopierMaster.ex5"
$ex5Slave = Join-Path $ProjectRoot "mt-advisors\MT5\SankeyCopierSlave.ex5"
if (Test-Path $ex5Master) { Copy-Item $ex5Master -Destination $Mt5ExpDir -Force; Write-Host "      Master.ex5 -> mt-advisors\MT5\Experts\" -ForegroundColor Gray }
if (Test-Path $ex5Slave) { Copy-Item $ex5Slave -Destination $Mt5ExpDir -Force; Write-Host "      Slave.ex5 -> mt-advisors\MT5\Experts\" -ForegroundColor Gray }

# Copy compiled .ex4 files
$Mt4ExpDir = Join-Path $TargetDir "mt-advisors\MT4\Experts"
if (-not (Test-Path $Mt4ExpDir)) { New-Item -ItemType Directory -Path $Mt4ExpDir -Force | Out-Null }
$ex4Master = Join-Path $ProjectRoot "mt-advisors\MT4\SankeyCopierMaster.ex4"
$ex4Slave = Join-Path $ProjectRoot "mt-advisors\MT4\SankeyCopierSlave.ex4"
if (Test-Path $ex4Master) { Copy-Item $ex4Master -Destination $Mt4ExpDir -Force; Write-Host "      Master.ex4 -> mt-advisors\MT4\Experts\" -ForegroundColor Gray }
if (Test-Path $ex4Slave) { Copy-Item $ex4Slave -Destination $Mt4ExpDir -Force; Write-Host "      Slave.ex4 -> mt-advisors\MT4\Experts\" -ForegroundColor Gray }

Write-Host ""
Write-Host "=== Deployment Complete ===" -ForegroundColor Cyan
Write-Host ""

# Keep window open
Read-Host "Press Enter to close"
