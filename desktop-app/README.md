# SANKEY Copier Desktop Application

## Overview

Tauri-based desktop application for SANKEY Copier settings management. This desktop app provides a native window interface for the Next.js web UI, with automatic port detection and Node.js process management.

## Architecture

### Components

- **Tauri (Rust)**: Native window management and Node.js process control
- **Next.js Standalone**: Web UI server (shared with service version)
- **Dynamic Port**: Automatically finds available port on launch
- **Node.js Runtime**: Required prerequisite (checked by installer)

### How It Works

1. Desktop app launches and finds an available port
2. Starts Next.js server as a child process on the detected port
3. Opens webview pointing to `http://localhost:{port}`
4. On exit, gracefully terminates the Node.js process and releases the port

## Prerequisites

- **Node.js v20 LTS**: Required for running the Next.js server
  - Installer will check for Node.js and prompt to download if not found
- **Rust toolchain**: Required only for building (not for running)
- **Tauri CLI**: Required only for building

## Building

### Development Build

```bash
cd desktop-app
pnpm install
pnpm run dev
```

### Production Build

```bash
cd desktop-app
pnpm install
pnpm run build
```

Output: `src-tauri/target/release/sankey-copier-desktop.exe`

## GitHub Actions Integration

The desktop app is built as part of the main build workflow:

1. `build-web.yml` - Builds Next.js standalone
2. `build-desktop.yml` - Builds Tauri desktop app
3. `build-installer.yml` - Packages desktop app with installer

## Installation

The desktop app is included in the main SANKEY Copier installer:

- **Desktop shortcut**: `SANKEY Copier` on desktop
- **Start menu**: `Open Desktop App` in SANKEY Copier folder
- **Installation path**: `C:\Program Files\SANKEY Copier\sankey-copier-desktop.exe`

## Usage

### Desktop Mode vs Service Mode

| Feature | Desktop Mode | Service Mode |
|---------|--------------|--------------|
| Launch | Manual (double-click) | Automatic (Windows service) |
| Port | Dynamic (auto-detected) | Fixed (8080) |
| Use case | Settings configuration | Continuous operation |
| Access | Local only | Can access remotely |

### Launching Desktop App

1. Double-click desktop shortcut or start menu icon
2. App automatically starts Next.js server
3. Opens in native window
4. Close window to exit (server stops automatically)

## File Structure

```
desktop-app/
├── src-tauri/
│   ├── src/
│   │   └── main.rs          # Main application logic
│   ├── Cargo.toml           # Rust dependencies
│   ├── tauri.conf.json      # Tauri configuration
│   └── build.rs             # Build script
├── package.json             # Node.js dependencies
└── README.md                # This file
```

## Configuration

### Tauri Configuration (`tauri.conf.json`)

- **Window size**: 1200x800
- **Product name**: SANKEY Copier
- **Bundle identifier**: com.sankey.copier.desktop

### Port Detection

- Uses `portpicker` crate to find available ports
- Waits up to 30 seconds for server to be ready
- Automatically retries if initial port is occupied

## Troubleshooting

### Desktop app won't start

1. **Check Node.js**: Open command prompt and run `node --version`
   - If not found, install Node.js v20 LTS from https://nodejs.org/
2. **Check logs**: Look for error messages in command prompt
3. **Try service mode**: Use the web interface via service mode (http://localhost:8080)

### Port conflict

- Desktop app automatically finds available ports
- If all ports are occupied, app will fail to start
- Close other applications using high port numbers

### Slow startup

- First launch may take longer as Next.js initializes
- Subsequent launches should be faster
- Desktop app waits for server readiness before showing window

## Development Notes

### Key Implementation Details

- **Process Management**: Node.js process is spawned as child and terminated on exit
- **Path Resolution**: Uses executable directory to locate `web-ui/server.js`
- **Port Waiting**: Polls port with TCP connection attempts before showing UI
- **Window Event**: Cleanup handled via `WindowEvent::Destroyed`

### Future Enhancements

- [ ] Add system tray integration for background operation
- [ ] Implement server health monitoring
- [ ] Add error dialog for startup failures
- [ ] Support custom port specification via config file
- [ ] Add update notification mechanism

## License

See main SANKEY Copier license.
