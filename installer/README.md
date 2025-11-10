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

#### Step 1: Build Components

Run the PowerShell build script:
```powershell
cd installer
.\build.ps1
```

This script will:
1. Build Rust server in release mode
2. Build Next.js web UI in standalone mode
3. Build tray application
4. Build MQL DLLs (32-bit and 64-bit)

#### Step 2: Compile Installer

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

### Services Won't Start
1. Check Windows Event Viewer for errors
2. Verify file permissions in installation directory
3. Check if ports 8080 and 5173 are available
4. Run `nssm status SankeyCopierServer` for details

### Web UI Not Accessible
1. Check if SankeyCopierWebUI service is running
2. Verify firewall settings
3. Try accessing http://localhost:8080 directly
4. Check logs in `C:\Program Files\SANKEY Copier\data\logs\`

### Permission Issues
- Installer requires administrator privileges
- Services run under SYSTEM account by default
- MT4/MT5 installation detection requires running MT4/MT5

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
