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

# Read project name from package.json
$packageJson = Get-Content "package.json" -Raw | ConvertFrom-Json
$projectName = $packageJson.name
Write-Host "Project name: $projectName" -ForegroundColor Cyan

# Copy static files to the correct location within standalone structure
# Next.js standalone expects: <project-name>/.next/static (preserving project structure)
Write-Host "Copying static files..." -ForegroundColor Yellow
$staticDest = Join-Path $bundleDir "$projectName\.next"
New-Item -ItemType Directory -Path $staticDest -Force | Out-Null
Copy-Item -Recurse -Path ".next\static" -Destination $staticDest

# Copy public directory to the correct location
if (Test-Path "public") {
    Write-Host "Copying public directory..." -ForegroundColor Yellow
    $publicDest = Join-Path $bundleDir $projectName
    Copy-Item -Recurse -Path "public" -Destination $publicDest
}

Write-Host "Web UI bundle prepared successfully at $bundleDir" -ForegroundColor Green
