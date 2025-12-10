#Requires -Version 5.1
<#
.SYNOPSIS
    Builds SANKEY Copier components locally and bundles them into 'dist'.
.DESCRIPTION
    Refactored version intended for better error handling and DRY principles.
.EXAMPLE
    .\build-local.ps1 -Clean
#>

param (
    [switch]$Clean
)

$ErrorActionPreference = "Stop"

# Configuration
$ProjectRoot = Split-Path -Parent $PSCommandPath
$MetaEditor5 = "C:\Program Files\XMTrading MT5\MetaEditor64.exe"
$MetaEditor4 = "C:\Program Files (x86)\XMTrading MT4\metaeditor.exe"
$DistDir     = Join-Path $ProjectRoot "dist"
$MqlInclude  = Join-Path $ProjectRoot "mt-advisors\Include"

Write-Host "=== SANKEY Copier Local Build Script ===" -ForegroundColor Cyan
Write-Host ""

# Early Clean of dist
if ($Clean) {
    if (Test-Path $DistDir) {
        Write-Host "[-] Cleaning 'dist' directory..." -ForegroundColor Yellow
        Remove-Item $DistDir -Recurse -Force
    }
}

# --- Step 1: Build Rust components ---
Write-Host "[1/3] Building Rust components..." -ForegroundColor Green
Push-Location $ProjectRoot
try {
    # Check specifically for 32-bit target
    $targets = rustup target list --installed
    if ($targets -notcontains "i686-pc-windows-msvc") {
        Write-Warning "Target 'i686-pc-windows-msvc' not found. Installing..."
        rustup target add i686-pc-windows-msvc
    }

    # Separate Clean logic from Build logic to avoid code duplication
    if ($Clean) {
        Write-Host "      Running CLEAN setup..." -ForegroundColor Yellow
        
        # Specific cleanup
        $PathsToDelete = @(
            "target\release\sankey_copier_zmq.dll",
            "target\i686-pc-windows-msvc\release\sankey_copier_zmq.dll",
            "target\release\sankey-copier-server.exe",
            "tray-app\target\release\sankey-copier-tray.exe"
        )
        foreach ($p in $PathsToDelete) {
            if (Test-Path $p) { Remove-Item $p -Force; Write-Host "      Deleted $p" -ForegroundColor Gray }
        }
    }

    # Unified Build Commands (Runs for both Clean and Incremental)
    Write-Host "      Building Workspace (64-bit DLL + Server)..." -ForegroundColor Gray
    cargo build --release -p sankey-copier-mt-bridge -p sankey-copier-relay-server
    if ($LASTEXITCODE -ne 0) { throw "Cargo build (Workspace) failed" }

    Write-Host "      Building Tray App..." -ForegroundColor Gray
    Push-Location "tray-app"
    if ($Clean) { cargo clean } # Local clean for tray app if requested
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "Cargo build (Tray App) failed" }
    Pop-Location

    Write-Host "      Building 32-bit DLL (MT4)..." -ForegroundColor Gray
    cargo build --release -p sankey-copier-mt-bridge --target i686-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw "Cargo build (32-bit) failed" }

    Write-Host "      Rust Build successful!" -ForegroundColor Green

} finally {
    Pop-Location
}

# --- Step 2 & 3: Compile MQL EAs (Helper Function) ---
function Compile-MQL {
    param (
        [string]$EditorPath,
        [string[]]$SourceFiles,
        [string]$Version
    )

    if (-not (Test-Path $EditorPath)) {
        Write-Host "      MetaEditor$Version not found at '$EditorPath'. Skipping." -ForegroundColor Yellow
        return
    }

    foreach ($src in $SourceFiles) {
        $fullPath = Join-Path $ProjectRoot $src
        $compiledExt = if ($Version -eq "5") { ".ex5" } else { ".ex4" }
        $expectedOut = $fullPath -replace "\.mq$Version`$", $compiledExt
        
        # Remove old artifact to ensure new one is created
        if (Test-Path $expectedOut) { Remove-Item $expectedOut -Force }

        Write-Host "      Compiling: $src" -ForegroundColor Gray
        
        # Start process and wait. MetaEditor logs to a file, but simple exit code check is minimal requirement
        $proc = Start-Process -FilePath $EditorPath -ArgumentList "/compile:`"$fullPath`"", "/include:`"$MqlInclude`"", "/log" -Wait -PassThru -NoNewWindow
        
        # Verify if .ex file exists after compilation
        if (-not (Test-Path $expectedOut)) {
            Write-Error "Compilation FAILED for $src. Check the compilation log in the same directory."
            # Optionally throw to stop script: throw "MQL Compilation failed"
        }
    }
    Write-Host "      MQL$Version compilation complete!" -ForegroundColor Green
}

Write-Host "[2/3] Compiling MQL5 EAs..." -ForegroundColor Green
Compile-MQL -EditorPath $MetaEditor5 -Version "5" -SourceFiles @(
    "mt-advisors\MT5\SankeyCopierMaster.mq5",
    "mt-advisors\MT5\SankeyCopierSlave.mq5"
)

Write-Host "[3/3] Compiling MQL4 EAs..." -ForegroundColor Green
Compile-MQL -EditorPath $MetaEditor4 -Version "4" -SourceFiles @(
    "mt-advisors\MT4\SankeyCopierMaster.mq4",
    "mt-advisors\MT4\SankeyCopierSlave.mq4"
)

# --- Final Step: Bundle artifacts ---
Write-Host "[-] Bundling artifacts to 'dist'..." -ForegroundColor Green
if (Test-Path $DistDir) { Remove-Item $DistDir -Recurse -Force }
New-Item -ItemType Directory -Path $DistDir -Force | Out-Null

# Structure creation
$Dirs = @(
    "mt-advisors\MT4\Libraries",
    "mt-advisors\MT5\Libraries",
    "mt-advisors\MT4\Experts",
    "mt-advisors\MT5\Experts"
)
foreach ($d in $Dirs) {
    New-Item -ItemType Directory -Path (Join-Path $DistDir $d) -Force | Out-Null
}

# Copy Helper
function Copy-IfFound {
    param($Src, $Dest)
    if (Test-Path $Src) { 
        Copy-Item $Src $Dest 
    } else {
        Write-Warning "Artifact not found: $Src"
    }
}

# Rust Artifacts
Copy-IfFound (Join-Path $ProjectRoot "target\i686-pc-windows-msvc\release\sankey_copier_zmq.dll") (Join-Path $DistDir "mt-advisors\MT4\Libraries")
Copy-IfFound (Join-Path $ProjectRoot "target\release\sankey_copier_zmq.dll") (Join-Path $DistDir "mt-advisors\MT5\Libraries")
Copy-IfFound (Join-Path $ProjectRoot "target\release\sankey-copier-server.exe") $DistDir
Copy-IfFound (Join-Path $ProjectRoot "relay-server\config.toml") $DistDir
Copy-IfFound (Join-Path $ProjectRoot "tray-app\target\release\sankey-copier-tray.exe") $DistDir

# MQL Artifacts
Copy-IfFound "$ProjectRoot\mt-advisors\MT5\SankeyCopierMaster.ex5" (Join-Path $DistDir "mt-advisors\MT5\Experts")
Copy-IfFound "$ProjectRoot\mt-advisors\MT5\SankeyCopierSlave.ex5"  (Join-Path $DistDir "mt-advisors\MT5\Experts")
Copy-IfFound "$ProjectRoot\mt-advisors\MT4\SankeyCopierMaster.ex4" (Join-Path $DistDir "mt-advisors\MT4\Experts")
Copy-IfFound "$ProjectRoot\mt-advisors\MT4\SankeyCopierSlave.ex4"  (Join-Path $DistDir "mt-advisors\MT4\Experts")

Write-Host "      Artifacts collected in: $DistDir" -ForegroundColor Cyan
Write-Host ""
Write-Host "=== Build Complete ===" -ForegroundColor Cyan
Write-Host ""
Read-Host "Press Enter to close"
