# Web UI Development Server Startup Script
# Kills any process using port 8080, cleans .next cache, and starts the Next.js dev server

$ErrorActionPreference = "Stop"
$PORT = 8080

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Web UI Dev Server Startup Script" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if port is in use
Write-Host "[1/4] Checking if port $PORT is in use..." -ForegroundColor Yellow

try {
    $connection = Get-NetTCPConnection -LocalPort $PORT -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1

    if ($connection) {
        $processId = $connection.OwningProcess
        $process = Get-Process -Id $processId -ErrorAction SilentlyContinue

        if ($process) {
            Write-Host "  -> Port $PORT is used by process: $($process.ProcessName) (PID: $processId)" -ForegroundColor Red
            Write-Host "  -> Killing process $processId..." -ForegroundColor Yellow

            try {
                Stop-Process -Id $processId -Force -ErrorAction Stop
                Write-Host "  -> Process killed successfully!" -ForegroundColor Green
                Start-Sleep -Seconds 1
            }
            catch {
                Write-Host "  -> Failed to kill process: $_" -ForegroundColor Red
                Write-Host "  -> Please manually close the process and try again." -ForegroundColor Red
                exit 1
            }
        }
    }
    else {
        Write-Host "  -> Port $PORT is available." -ForegroundColor Green
    }
}
catch {
    Write-Host "  -> Error checking port status: $_" -ForegroundColor Red
}

Write-Host ""

# Clean .next build cache
Write-Host "[2/4] Cleaning Next.js build cache..." -ForegroundColor Yellow

if (Test-Path ".next") {
    Write-Host "  -> .next directory found. Removing..." -ForegroundColor Yellow
    try {
        Remove-Item -Path ".next" -Recurse -Force -ErrorAction Stop
        Write-Host "  -> .next directory removed successfully!" -ForegroundColor Green
    }
    catch {
        Write-Host "  -> Warning: Failed to remove .next directory: $_" -ForegroundColor Yellow
        Write-Host "  -> Continuing anyway..." -ForegroundColor Yellow
    }
}
else {
    Write-Host "  -> .next directory not found (clean state)." -ForegroundColor Green
}

Write-Host ""

# Verify pnpm is installed
Write-Host "[3/4] Verifying Node.js/pnpm installation..." -ForegroundColor Yellow

$pnpmPath = Get-Command pnpm -ErrorAction SilentlyContinue
if (-not $pnpmPath) {
    Write-Host "  -> ERROR: pnpm not found. Please install pnpm." -ForegroundColor Red
    Write-Host "  -> Visit: https://pnpm.io/installation" -ForegroundColor Red
    Write-Host "  -> Recommended: run 'mise install' at the repo root (pins pnpm 10.20.0)" -ForegroundColor Yellow
    exit 1
}
Write-Host "  -> pnpm found: $($pnpmPath.Source)" -ForegroundColor Green

# Check if node_modules exists
if (-not (Test-Path "node_modules")) {
    Write-Host "  -> node_modules not found. Running pnpm install..." -ForegroundColor Yellow
    pnpm install
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  -> pnpm install failed!" -ForegroundColor Red
        exit 1
    }
}

Write-Host ""

# Start the dev server
Write-Host "[4/4] Starting Web UI dev server on port $PORT..." -ForegroundColor Yellow
Write-Host "  -> Running: pnpm dev" -ForegroundColor Cyan
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Run pnpm dev server
pnpm dev
