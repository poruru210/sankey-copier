# SANKEY Copier Tray Application

System tray application for managing SANKEY Copier Windows services.

## Architecture

The application has been refactored from a single 670-line file into a modular architecture:

```
src/
├── main.rs         (69 lines)  - Entry point and event loop
├── config.rs       (78 lines)  - Configuration management
├── elevation.rs    (64 lines)  - UAC elevation utilities
├── icon.rs         (68 lines)  - Tray icon loading
├── menu.rs        (194 lines)  - Menu creation and event handling
├── service.rs     (147 lines)  - Windows service control
└── ui.rs           (94 lines)  - Message box dialogs
```

### Module Responsibilities

#### `main.rs`
- Application entry point
- Event loop management
- Tray icon initialization
- Coordinates all other modules

#### `config.rs`
- Loads configuration from `config.toml`
- Manages server and Web UI port settings
- Provides Web URL generation

#### `elevation.rs`
- Handles UAC (User Account Control) elevation
- Creates and executes elevated batch commands
- Required for Windows service operations

#### `icon.rs`
- Loads tray icon from embedded resources
- Fallback to file system if embedded icon fails
- Generates default icon as last resort

#### `menu.rs`
- Creates system tray menu structure
- Handles menu item events
- Coordinates actions between UI and service modules
- Defines `AppEvent` enum for application control

#### `service.rs`
- Controls Windows services via NSSM
- Start/stop/restart operations for:
  - Server service (`SankeyCopierServer`)
  - Web UI service (`SankeyCopierWebUI`)
- Query service status

#### `ui.rs`
- Windows message box dialogs
- Error and info notifications
- About dialog with version information

## Benefits of Refactoring

### Before
- ❌ Single 670-line file
- ❌ Mixed responsibilities
- ❌ Global state scattered throughout
- ❌ Dead code included
- ❌ Difficult to test
- ❌ Hard to maintain

### After
- ✅ 7 focused modules (69-194 lines each)
- ✅ Clear separation of concerns
- ✅ Better encapsulation
- ✅ No dead code
- ✅ Easier to test individual components
- ✅ Improved maintainability

## Building

```bash
cargo build --release
```

**Note:** This is a Windows-only application. Building requires:
- Windows target toolchain
- NSSM (Non-Sucking Service Manager)
- Windows API libraries

### CI Coverage

Pull Request builds now include a Windows `Build Tray Application` job whenever files under `tray-app/` change, ensuring binaries stay green before merging.

## Usage

The tray application provides:

1. **UI Menu**
   - Open Web Interface
   - Start/Stop/Restart Web UI service

2. **Service Menu**
   - Start/Stop/Restart Server service

3. **Status Check**
   - View current service status

4. **About**
   - Application version and information

## Dependencies

- `tray-icon` - System tray functionality
- `winit` - Event loop
- `windows` - Windows API access
- `webbrowser` - Open URLs in browser
- `anyhow` - Error handling
- `serde` / `toml` - Configuration parsing
- `image` - Icon loading

## License

MIT License - See main project README for details.
