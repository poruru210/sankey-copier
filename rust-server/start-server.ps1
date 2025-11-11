# Rust Server Startup Script
# Kills any process using port 3000 and starts the Rust server

$ErrorActionPreference = "Stop"
$PORT = 3000

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Rust Server Startup Script" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if port is in use
Write-Host "[1/3] Checking if port $PORT is in use..." -ForegroundColor Yellow

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

# Verify cargo is installed
Write-Host "[2/3] Verifying Rust/Cargo installation..." -ForegroundColor Yellow

$cargoPath = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargoPath) {
    Write-Host "  -> ERROR: Cargo not found. Please install Rust." -ForegroundColor Red
    Write-Host "  -> Visit: https://rustup.rs/" -ForegroundColor Red
    exit 1
}
Write-Host "  -> Cargo found: $($cargoPath.Source)" -ForegroundColor Green
Write-Host ""

# Start the server
Write-Host "[3/3] Starting Rust server on port $PORT..." -ForegroundColor Yellow
Write-Host "  -> Running: cargo run --release" -ForegroundColor Cyan
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Run cargo in release mode
# Use development mode for faster compilation: cargo run
cargo run --release
