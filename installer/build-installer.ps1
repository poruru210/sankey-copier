# SANKEY Copier Unified Installer Build Script
# Builds rust-server + Desktop App + MT4/MT5 components and creates Windows installer

param(
    [switch]$SkipBuild,
    [switch]$SkipMQL
)

$ErrorActionPreference = "Stop"

Write-Host "=====================================" -ForegroundColor Cyan
Write-Host "Building SANKEY Copier Installer" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan

$PROJECT_ROOT = (Get-Item $PSScriptRoot).Parent.FullName

if (-not $SkipBuild) {
    # 1. Build rust-server
    Write-Host "`n[1/4] Building rust-server..." -ForegroundColor Yellow
    Push-Location "$PROJECT_ROOT\rust-server"
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw "rust-server build failed" }
    Pop-Location

    # 2. Build web-ui (static export for Desktop App)
    Write-Host "`n[2/4] Building web-ui (static export)..." -ForegroundColor Yellow
    Push-Location "$PROJECT_ROOT\web-ui"
    $env:NEXT_BUILD_MODE = "export"

    # Check if pnpm is available, fallback to npm
    $packageManager = "npm"
    if (Get-Command pnpm -ErrorAction SilentlyContinue) {
        $packageManager = "pnpm"
    }

    & $packageManager install
    if ($LASTEXITCODE -ne 0) { throw "web-ui install failed" }

    & $packageManager run build
    if ($LASTEXITCODE -ne 0) { throw "web-ui build failed" }
    Pop-Location

    # 3. Build Desktop App (Tauri)
    Write-Host "`n[3/4] Building Desktop App..." -ForegroundColor Yellow
    Push-Location "$PROJECT_ROOT\desktop-app"

    npm install
    if ($LASTEXITCODE -ne 0) { throw "Desktop App install failed" }

    # Build without bundles (we'll create installer with Inno Setup)
    npm run tauri build -- --bundles none
    if ($LASTEXITCODE -ne 0) { throw "Desktop App build failed" }
    Pop-Location

    # 4. Build MT4/MT5 components (optional)
    if (-not $SkipMQL) {
        Write-Host "`n[4/4] Building MT4/MT5 components..." -ForegroundColor Yellow
        if (Test-Path "$PROJECT_ROOT\mql") {
            Push-Location "$PROJECT_ROOT\mql"

            # Check if build script exists
            if (Test-Path ".\build.ps1") {
                .\build.ps1
                if ($LASTEXITCODE -ne 0) {
                    Write-Host "⚠️  MQL build failed, but continuing..." -ForegroundColor Yellow
                }
            } else {
                Write-Host "⚠️  MQL build script not found, skipping..." -ForegroundColor Yellow
            }

            Pop-Location
        } else {
            Write-Host "⚠️  MQL directory not found, skipping..." -ForegroundColor Yellow
        }
    } else {
        Write-Host "`n[4/4] Skipping MT4/MT5 components..." -ForegroundColor Yellow
    }
} else {
    Write-Host "Skipping builds (using existing binaries)..." -ForegroundColor Yellow
}

# 5. Build installer with Inno Setup
Write-Host "`n[5/5] Building installer with Inno Setup..." -ForegroundColor Yellow

# Check if Inno Setup is installed
$InnoSetupPaths = @(
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles}\Inno Setup 6\ISCC.exe"
)

$InnoSetupPath = $null
foreach ($path in $InnoSetupPaths) {
    if (Test-Path $path) {
        $InnoSetupPath = $path
        break
    }
}

if (-not $InnoSetupPath) {
    Write-Host "❌ Inno Setup 6 not found!" -ForegroundColor Red
    Write-Host "" -ForegroundColor Yellow
    Write-Host "Please install Inno Setup 6 from:" -ForegroundColor Yellow
    Write-Host "https://jrsoftware.org/isdl.php" -ForegroundColor Yellow
    Write-Host "" -ForegroundColor Yellow
    Write-Host "Or install via Chocolatey:" -ForegroundColor Yellow
    Write-Host "  choco install innosetup -y" -ForegroundColor Yellow
    exit 1
}

Write-Host "Using Inno Setup: $InnoSetupPath" -ForegroundColor Gray

# Compile installer
Push-Location "$PROJECT_ROOT\installer"
& $InnoSetupPath "setup.iss"
if ($LASTEXITCODE -ne 0) { throw "Installer build failed" }
Pop-Location

Write-Host "`n✅ Installer build completed!" -ForegroundColor Green
Write-Host "Output: installer\output\SankeyCopierSetup-1.0.0.exe" -ForegroundColor Green
Write-Host "" -ForegroundColor White

# Display file size
$installerPath = "$PROJECT_ROOT\installer\output\SankeyCopierSetup-1.0.0.exe"
if (Test-Path $installerPath) {
    $fileSize = (Get-Item $installerPath).Length / 1MB
    Write-Host "Installer size: $([math]::Round($fileSize, 2)) MB" -ForegroundColor Cyan
}
