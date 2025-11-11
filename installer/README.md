# SANKEY Copier Windows Installer

This directory contains the Windows installer configuration and build scripts.

## Prerequisites

### 1. Inno Setup
Download and install Inno Setup 6.x from:
- Official site: https://jrsoftware.org/isdl.php
- Version: 6.2.2 or later

### 2. NSSM (Non-Sucking Service Manager)
Download NSSM from:
- Official site: https://nssm.cc/download
- Direct link: https://nssm.cc/release/nssm-2.24.zip

**Installation:**
1. Download `nssm-2.24.zip`
2. Extract the zip file
3. Copy `nssm-2.24/win64/nssm.exe` to `installer/resources/nssm.exe`
4. (Optional) Copy `nssm-2.24/win32/nssm.exe` to `installer/resources/nssm-x86.exe` for 32-bit support

**Note:** NSSM binaries are not included in this repository. You must download them separately.

### 3. Build Tools
- Rust toolchain (for building rust-server)
- Node.js 18+ (for building web-ui)
- PowerShell 5.1+ (for running build scripts)

## Version Number Format

SANKEY Copier uses **Semantic Versioning (SemVer)**:

```
MAJOR.MINOR.PATCH
```

- **MAJOR**: Incompatible API changes or major features (e.g., 1.0.0 → 2.0.0)
- **MINOR**: Backward-compatible new features (e.g., 1.0.0 → 1.1.0)
- **PATCH**: Backward-compatible bug fixes (e.g., 1.0.0 → 1.0.1)

**Examples:**
- `1.0.0` - Initial release
- `1.1.0` - Added new feature (e.g., Telegram notifications)
- `1.1.1` - Fixed bug in notifications
- `2.0.0` - Breaking change (e.g., new database schema)

**Git tags must include 'v' prefix:**
```bash
git tag v1.0.0        # Correct
git tag 1.0.0         # Won't trigger workflow
```

### Version Management

**GitHub Actions automatically updates versions:**
- All `Cargo.toml` files (rust-server, mql-zmq-dll, sankey-copier-tray)
- `web-ui/package.json`
- Inno Setup installer configuration

**Repository versions:**
- Default version in repository: `1.0.0`
- During build, GitHub Actions replaces with actual version from git tag
- Tray app "About" dialog shows version from CARGO_PKG_VERSION (dynamically updated)

## Building the Installer

### Option 1: Using GitHub Actions (Recommended)

The easiest way to create an installer is using GitHub Actions:

**For tagged releases:**
```bash
git tag v1.0.0
git push origin v1.0.0
```
The workflow will automatically build and create a GitHub release with the installer.

**Manual trigger:**
1. Go to Actions tab on GitHub
2. Select "Build Windows Installer" workflow
3. Click "Run workflow"
4. Enter version number
5. Download installer from Artifacts

**Advantages:**
- Fully automated build process
- No need to manually download NSSM
- Consistent build environment
- Installer automatically attached to GitHub releases

### Option 2: Local Build

#### Quick Start - Automated Build (Recommended)

**ONE-STEP BUILD:** Run the automated PowerShell build script:

```powershell
cd installer
.\build-installer.ps1
```

**With custom version:**
```powershell
.\build-installer.ps1 -Version "1.2.3"
```

**Skip tests (faster build):**
```powershell
.\build-installer.ps1 -Version "1.2.3" -SkipTests
```

This script will:
1. Build Rust server in release mode (with tests)
2. Build MQL ZMQ DLL (64-bit for MT5)
3. Build MQL ZMQ DLL (32-bit for MT4)
4. Build system tray application
5. Build Next.js web UI in standalone mode
6. Verify all required files exist
7. Compile Windows installer with Inno Setup
8. Show installer location and size

**⚠️ IMPORTANT:** The build script must complete successfully for the installer to work correctly. If any step fails, the script will stop with an error message.

#### Manual Build (Step by Step)

If you prefer to build components manually or need to troubleshoot:

**Step 1: Build Rust Server**
```powershell
cd rust-server
cargo test --release      # Optional but recommended
cargo build --release
```

**Step 2: Build MQL ZMQ DLLs**
```powershell
cd mql-zmq-dll

# 64-bit (for MT5)
cargo build --release

# 32-bit (for MT4)
cargo build --release --target i686-pc-windows-msvc
```

**Step 3: Build Tray Application**
```powershell
cd sankey-copier-tray
cargo build --release
```

**Step 4: Build Web UI (CRITICAL)**
```powershell
cd web-ui
pnpm install
pnpm run build
```

**⚠️ CRITICAL:** This step creates the `.next/standalone` directory which is required for the Web UI service to work. If you skip this step or it fails, the Web UI service will not start after installation.

**Verify standalone build:**
```powershell
# This directory MUST exist
Test-Path web-ui\.next\standalone

# Should output: True
```

**Step 5: Compile Installer**

**Option A: Using Inno Setup GUI**
1. Open `installer/setup.iss` in Inno Setup Compiler
2. Click "Compile" button
3. Installer will be created in `installer/Output/`

**Option B: Using Command Line (with version)**
```powershell
# Default version (1.0.0)
"C:\Program Files (x86)\Inno Setup 6\ISCC.exe" setup.iss

# Specify version
"C:\Program Files (x86)\Inno Setup 6\ISCC.exe" /DMyAppVersion=1.2.3 setup.iss
```

### Output
The installer will be created as:
```
installer/Output/SankeyCopierSetup-x.x.x.exe
```

**Version in filename:**
- If you don't specify `/DMyAppVersion`, the default `1.0.0` is used
- Specify version via command line to match your release version

## Installer Features

The installer will:
- Install Rust server (`sankey-copier-server.exe`)
- Install Next.js web UI (standalone build)
- Install system tray application (`sankey-copier-tray.exe`)
- Install NSSM for service management
- Register Windows services:
  - `SankeyCopierServer` (Rust backend)
  - `SankeyCopierWebUI` (Next.js frontend)
- Create start menu shortcuts
- Set up configuration files
- Configure automatic startup
- Add tray application to Windows startup (optional)

## Directory Structure After Installation

```
C:\Program Files\SANKEY Copier\
├── sankey-copier-server.exe    # Rust server
├── sankey-copier-tray.exe      # System tray application
├── web-ui\                      # Next.js standalone build
│   ├── server.js
│   ├── .next\
│   └── public\
├── nssm.exe                     # Service manager
├── config.toml                  # Server configuration
├── data\                        # Database and logs
│   ├── sankey_copier.db
│   └── logs\
└── mql\                         # MT4/MT5 components
    ├── MT4\
    └── MT5\
```

## Windows Services

After installation, two Windows services will be registered:

### SankeyCopierServer
- **Display Name:** SANKEY Copier Server
- **Startup Type:** Automatic
- **Port:** 8080 (configurable)
- **Description:** Backend server for SANKEY Copier

### SankeyCopierWebUI
- **Display Name:** SANKEY Copier Web UI
- **Startup Type:** Automatic
- **Port:** 5173 (proxied through server)
- **Description:** Web interface for SANKEY Copier

## System Tray Application

The tray application provides convenient control of SANKEY Copier services:

### Features
- **System tray icon** - Always visible in Windows system tray
- **Quick service control** - Start, stop, and restart services with a click
- **Status checking** - View current service status
- **Web interface launcher** - Open web UI in browser directly
- **Windows startup** - Optionally launch automatically on login

### Using the Tray Application
Right-click the tray icon to access:
- **Open Web Interface** - Opens http://localhost:8080 in default browser
- **Start Services** - Starts both SankeyCopierServer and SankeyCopierWebUI
- **Stop Services** - Stops both services
- **Restart Services** - Restarts both services
- **Check Status** - Shows current service status in a popup
- **Quit** - Closes the tray application

**Note:** The tray application itself does not need to be running for services to work. It's just a convenient control interface.

## Service Management

### Manual Commands
```cmd
# Start services
sc start SankeyCopierServer
sc start SankeyCopierWebUI

# Stop services
sc stop SankeyCopierServer
sc stop SankeyCopierWebUI

# Restart services
sc stop SankeyCopierServer && sc start SankeyCopierServer
sc stop SankeyCopierWebUI && sc start SankeyCopierWebUI

# Check status
sc query SankeyCopierServer
sc query SankeyCopierWebUI
```

### Using NSSM
```cmd
# Check service status
nssm status SankeyCopierServer
nssm status SankeyCopierWebUI

# Restart services
nssm restart SankeyCopierServer
nssm restart SankeyCopierWebUI

# Edit service configuration
nssm edit SankeyCopierServer
```

## Uninstallation

The installer includes a full uninstaller that will:
1. Stop all running services
2. Remove Windows services
3. Delete program files
4. Clean up registry entries
5. Optionally remove user data (database, logs)

## Troubleshooting

### Web UI Service in PAUSED State

**Symptoms:**
- Service shows as "PAUSED" instead of "RUNNING"
- Web interface not accessible at http://localhost:8080
- Error in logs: `Cannot find module 'styled-jsx/package.json'` or `Cannot find module 'C:\Program'`

**Root Cause:**
The Web UI standalone build was not created before compiling the installer.

**Solution:**
1. Delete the old installer
2. Build Web UI properly:
   ```powershell
   cd web-ui
   pnpm install
   pnpm run build
   ```
3. Verify standalone build exists:
   ```powershell
   Test-Path .next\standalone  # Must return True
   ```
4. Rebuild installer using `build-installer.ps1` script

### Services Won't Start
1. Check Windows Event Viewer for errors:
   - Application Log: Look for "SankeyCopier" entries
   - System Log: Look for Service Control Manager errors
2. Verify file permissions in installation directory
3. Check if ports 8080 and 3000 are available:
   ```powershell
   netstat -ano | findstr "8080"
   netstat -ano | findstr "3000"
   ```
4. Check NSSM service configuration:
   ```cmd
   nssm get SankeyCopierWebUI Application
   nssm get SankeyCopierWebUI AppParameters
   nssm get SankeyCopierWebUI AppDirectory
   ```
5. View service logs:
   ```powershell
   Get-Content "C:\Program Files\SANKEY Copier\data\logs\webui-stderr.log" -Tail 50
   Get-Content "C:\Program Files\SANKEY Copier\data\logs\server-stderr.log" -Tail 50
   ```

### Tray Application Not Responding

**Symptoms:**
- Clicking tray icon does nothing
- "Open Web Interface" opens wrong URL

**Solutions:**
1. Verify tray app is using correct port (8080):
   - Check `sankey-copier-tray\src\main.rs` line 19
   - Should be: `const WEB_URL: &str = "http://localhost:8080";`
2. Rebuild tray application if port was wrong:
   ```powershell
   cd sankey-copier-tray
   cargo build --release
   ```
3. Restart tray application:
   - Close existing tray app (right-click → Quit)
   - Launch from Start Menu or run manually

### Web UI Not Accessible
1. Check if SankeyCopierWebUI service is running:
   ```cmd
   sc query SankeyCopierWebUI
   ```
2. Verify firewall settings allow connections on port 8080
3. Try accessing http://localhost:8080 directly in browser
4. Check server is running:
   ```cmd
   sc query SankeyCopierServer
   ```
5. Check logs in `C:\Program Files\SANKEY Copier\data\logs\`

### Node.js Path Issues with Spaces

**Symptoms:**
- Error: `Cannot find module 'C:\Program'`
- Service starts then immediately stops

**Root Cause:**
NSSM not properly quoting paths with spaces.

**Solution:**
Use the updated installer (v1.0.1+) which uses `AppParameters` to properly quote the server.js path.

**Manual Fix (if using old installer):**
```cmd
"C:\Program Files\SANKEY Copier\nssm.exe" set SankeyCopierWebUI AppParameters "C:\Program Files\SANKEY Copier\web-ui\server.js"
"C:\Program Files\SANKEY Copier\nssm.exe" start SankeyCopierWebUI
```

### Permission Issues
- Installer requires administrator privileges
- Services run under SYSTEM account by default
- MT4/MT5 installation detection requires running MT4/MT5

### Build Script Failures

**If `build-installer.ps1` fails:**
1. Check error message for which component failed
2. Try building that component manually to see detailed error
3. Common issues:
   - Rust not installed: Install from https://rustup.rs
   - Node.js not installed: Install from https://nodejs.org
   - pnpm not installed: Run `npm install -g pnpm`
   - 32-bit Rust target missing: Run `rustup target add i686-pc-windows-msvc`
   - Inno Setup not found: Install from https://jrsoftware.org/isdl.php

## Development Notes

### Testing Without Installing
You can test the components without creating an installer:

```bash
# Terminal 1: Start Rust server
cd rust-server
cargo run --release

# Terminal 2: Start Next.js dev server
cd web-ui
npm run dev
```

### Modifying the Installer
Edit `setup.iss` to customize:
- Installation directory
- Service names and descriptions
- Start menu entries
- Custom pages and prompts
- Pre/post-installation scripts

## License

See `resources/license.txt` for license information.
