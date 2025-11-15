# Rust Server - SANKEY Copier Backend

Backend server for the SANKEY Copier MT4/MT5 trade copying system.

## Configuration

The server uses a layered configuration system supporting multiple TOML files. Configuration files are loaded in the following order, with later files overriding earlier ones:

### Configuration File Precedence

1. **config.toml** (Required)
   - Base configuration file
   - Contains all required settings
   - Checked into version control

2. **config.{ENV}.toml** (Optional)
   - Environment-specific overrides (e.g., config.dev.toml, config.prod.toml)
   - Only loaded when `CONFIG_ENV` environment variable is set
   - Checked into version control
   - Shared by team members in the same environment

3. **config.local.toml** (Optional)
   - Personal overrides for individual developers
   - Highest priority - overrides all other files
   - **Git-ignored** - never committed
   - Use for machine-specific settings

### Environment Variables

- **CONFIG_ENV**: Controls which environment-specific config to load
  - **No default** - must be explicitly set to load environment config
  - Common values: `dev`, `prod`, `staging`
  - File pattern: `config.{CONFIG_ENV}.toml`
  - **Security**: Production environments should NOT set this variable unless config.prod.toml exists

### Example Usage

**Production (default):**
```bash
# Loads: config.toml → config.local.toml (no environment config)
# Safest for production - only uses base config
cargo run
```

**Development:**
```bash
# Loads: config.toml → config.dev.toml → config.local.toml
# Use the provided start-server.ps1 script, or set CONFIG_ENV manually:
CONFIG_ENV=dev cargo run
```

**Custom environment:**
```bash
# Loads: config.toml → config.staging.toml → config.local.toml
CONFIG_ENV=staging cargo run
```

### Configuration Sections

See [config.toml](config.toml) for the complete structure and available options:

- `[server]` - HTTP server settings (host, port)
- `[webui]` - Web UI connection settings
- `[database]` - Database connection URL
- `[zeromq]` - ZeroMQ port configuration
- `[cors]` - CORS policy settings
- `[logging]` - File logging configuration

### Development Configuration

The `config.dev.toml` file contains development-specific overrides:

```toml
[cors]
disable = true  # Disable CORS restrictions for local development
```

**WARNING:** Never set `cors.disable = true` in production environments!

### Personal Overrides

Create `config.local.toml` for personal settings that should not be shared:

```toml
[server]
port = 9090  # Use a different port on your machine

[cors]
additional_origins = ["http://localhost:5173"]  # Add your local frontend
```

This file is git-ignored and will not be committed to the repository.

## Building

### Development Build
```bash
cargo build
```

### Release Build
```bash
cargo build --release
```

### With Version Information (CI/CD)
```bash
PACKAGE_VERSION=1.0.0 FILE_VERSION=1.0.0.123 cargo build --release
```

## Running

### Development
```bash
cargo run
```

### Production
```bash
CONFIG_ENV=prod ./target/release/sankey-copier-server
```

## Version Information

The server embeds version information in the Windows executable:
- **PACKAGE_VERSION**: Semantic version (e.g., "1.0.0")
- **FILE_VERSION**: Build version with commit count (e.g., "1.0.0.123")
- **BUILD_INFO**: Detailed build metadata (e.g., "1.0.0+build.123.abc1234")

Version information is automatically generated from:
1. Environment variables (CI/CD): `PACKAGE_VERSION`, `FILE_VERSION`
2. Git tags and commit history (local development)

View version info:
```bash
# Windows: Right-click exe → Properties → Details tab
# Or check build output for version information
```
