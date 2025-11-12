# PowerShell script to prepare web-ui for Tauri bundling
$ErrorActionPreference = "Stop"

Write-Host "Preparing web-ui for Tauri bundling..." -ForegroundColor Cyan

# Navigate to web-ui directory
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location (Join-Path $scriptDir "..\web-ui")

# Build Next.js standalone
Write-Host "Building Next.js standalone..." -ForegroundColor Yellow
pnpm install --frozen-lockfile
pnpm build

# Create bundle directory
$bundleDir = Join-Path $scriptDir "web-ui"
if (Test-Path $bundleDir) {
    Remove-Item -Recurse -Force $bundleDir
}
New-Item -ItemType Directory -Path $bundleDir | Out-Null

# Copy standalone build
Write-Host "Copying standalone build..." -ForegroundColor Yellow
Copy-Item -Recurse -Path ".next\standalone\*" -Destination $bundleDir

# Copy static files
Write-Host "Copying static files..." -ForegroundColor Yellow
$staticDest = Join-Path $bundleDir ".next"
New-Item -ItemType Directory -Path $staticDest -Force | Out-Null
Copy-Item -Recurse -Path ".next\static" -Destination $staticDest

# Copy public directory
if (Test-Path "public") {
    Write-Host "Copying public directory..." -ForegroundColor Yellow
    Copy-Item -Recurse -Path "public" -Destination $bundleDir
}

Write-Host "Web UI bundle prepared successfully at $bundleDir" -ForegroundColor Green
