# Versioning Strategy

This document defines the versioning strategy for the SANKEY Copier project.

## Version Numbers

The project uses **three different version formats** for different purposes:

### 1. PACKAGE_VERSION

**Format**: `MAJOR.MINOR.PATCH` (Semantic Versioning)

**Example**: `1.2.3`

**Purpose**:
- Cargo.toml (Rust packages)
- package.json (npm packages)
- Release tags
- User-facing version number

**Rules**:
- MAJOR: Breaking changes
- MINOR: New features (backward compatible)
- PATCH: Bug fixes (backward compatible)

### 2. FILE_VERSION

**Format**: `MAJOR.MINOR.PATCH.BUILD` (Windows 4-component version)

**Example**: `1.2.3.169`

**Purpose**:
- Windows executable file VERSIONINFO (numeric format)
- Displayed in Windows Explorer file properties
- Used for version comparison by Windows

**Rules**:
- First 3 components: Same as PACKAGE_VERSION
- BUILD: Total commit count from repository start
- Each component: 0-65535 range

### 3. BUILD_INFO

**Format**: `MAJOR.MINOR.PATCH+build.COUNT.HASH[-dirty]` (Extended SemVer with metadata)

**Example**: `1.2.3+build.169.abc1234-dirty`

**Purpose**:
- Log output (application startup messages)
- Debug information
- Heartbeat messages (ZeroMQ communication)
- Support inquiries (traceability)

**Components**:
- Base version: Same as PACKAGE_VERSION
- `build.COUNT`: Total commit count
- `HASH`: Short commit hash (7 characters)
- `-dirty` (optional): Working tree has uncommitted changes

## Version Generation

### Build-time Generation (build.rs)

All Rust projects (`rust-server`, `mql-zmq-dll`, `sankey-copier-tray`) generate these versions during build:

```rust
// Environment variables set by build.rs:
PACKAGE_VERSION = "1.2.3"
FILE_VERSION    = "1.2.3.169"
BUILD_INFO      = "1.2.3+build.169.abc1234"
```

### Version Sources

1. **Git Tags** (primary source):
   ```bash
   git describe --tags --abbrev=0 --match "v[0-9]*"
   # Example: v1.2.3
   ```

2. **Commit Count** (for BUILD component):
   ```bash
   git rev-list --count HEAD
   # Example: 169
   ```

3. **Commit Hash** (for BUILD_INFO):
   ```bash
   git rev-parse --short HEAD
   # Example: abc1234
   ```

4. **Dirty State** (for BUILD_INFO):
   ```bash
   git diff --quiet || echo "-dirty"
   ```

### Fallback Behavior

If no Git tags exist:
- PACKAGE_VERSION: `0.1.0`
- FILE_VERSION: `0.1.0.{commit_count}`
- BUILD_INFO: `0.1.0+build.{commit_count}.{hash}`

## Release Process

### Creating a Release

1. **Decide version number** (follow Semantic Versioning):
   - Breaking changes → Bump MAJOR
   - New features → Bump MINOR
   - Bug fixes → Bump PATCH

2. **Create and push tag**:
   ```bash
   git tag v1.2.3 -m "Release version 1.2.3"
   git push origin v1.2.3
   ```

3. **GitHub Actions automatically**:
   - Builds all components with version `1.2.3`
   - Updates Cargo.toml and package.json
   - Compiles Windows installer
   - Creates GitHub Release

### Development Workflow

**During Development** (no new tag):
```
Commit #170 on top of v1.2.3

PACKAGE_VERSION = "1.2.3"
FILE_VERSION    = "1.2.3.170"
BUILD_INFO      = "1.2.3+build.170.def5678"
```

**With Uncommitted Changes**:
```
BUILD_INFO = "1.2.3+build.170.def5678-dirty"
```

## Usage in Code

### Rust Code

```rust
// Log output (use BUILD_INFO for detailed information)
tracing::info!("Server Version: {}", env!("BUILD_INFO"));

// Heartbeat message (use BUILD_INFO for traceability)
HeartbeatMessage {
    version: env!("BUILD_INFO").to_string(),
    // ...
}
```

### Windows VERSIONINFO

```rust
// sankey-copier-tray/build.rs

// String version (shown in Explorer)
res.set("ProductVersion", env!("BUILD_INFO"));      // "1.2.3+build.169.abc1234"
res.set("FileVersion", env!("FILE_VERSION"));       // "1.2.3.169"

// Numeric version (for programmatic access)
let version = parse_version(env!("FILE_VERSION")); // 0x0001000200030169
res.set_version_info(winres::VersionInfo::FILEVERSION, version);
```

### GitHub Actions

```yaml
- name: Get version from build.rs logic
  id: version
  run: |
    # Get tag (or fallback to v0.1.0)
    TAG=$(git describe --tags --abbrev=0 --match "v[0-9]*" 2>/dev/null || echo "v0.1.0")
    VERSION=${TAG#v}

    # Get commit count
    BUILD=$(git rev-list --count HEAD)

    # Get commit hash
    HASH=$(git rev-parse --short HEAD)

    # Generate versions
    echo "package_version=$VERSION" >> $GITHUB_OUTPUT
    echo "file_version=$VERSION.$BUILD" >> $GITHUB_OUTPUT
    echo "build_info=$VERSION+build.$BUILD.$HASH" >> $GITHUB_OUTPUT
```

## Examples

### Release Build (tag: v1.2.3, commit #169)

| Component | Version | Usage |
|-----------|---------|-------|
| PACKAGE_VERSION | `1.2.3` | Cargo.toml, package.json |
| FILE_VERSION | `1.2.3.169` | Windows file properties |
| BUILD_INFO | `1.2.3+build.169.e0e786b` | Logs, debug info |

### Development Build (no tag, commit #169)

| Component | Version | Usage |
|-----------|---------|-------|
| PACKAGE_VERSION | `0.1.0` | Cargo.toml, package.json |
| FILE_VERSION | `0.1.0.169` | Windows file properties |
| BUILD_INFO | `0.1.0+build.169.e0e786b` | Logs, debug info |

### Dirty Build (uncommitted changes)

| Component | Version | Usage |
|-----------|---------|-------|
| BUILD_INFO | `1.2.3+build.169.e0e786b-dirty` | Clearly indicates uncommitted changes |

## Best Practices

1. **Always create tags for releases**:
   - Use `v` prefix (e.g., `v1.2.3`)
   - Follow Semantic Versioning
   - Add meaningful tag messages

2. **Never manually edit version numbers**:
   - GitHub Actions automatically updates Cargo.toml and package.json
   - Local builds use git-derived versions

3. **Check BUILD_INFO in logs**:
   - Verify the build you're running
   - The `-dirty` suffix indicates development build

4. **For support inquiries**:
   - Ask users to provide BUILD_INFO from log files
   - This provides exact commit hash for troubleshooting

## Comparison with Other Systems

| System | Format | Example |
|--------|--------|---------|
| Semantic Versioning | `X.Y.Z` | `1.2.3` |
| CalVer | `YYYY.MM.PATCH` | `2025.11.1` |
| Windows | `X.Y.Z.B` | `1.2.3.169` |
| SemVer + Metadata | `X.Y.Z+meta` | `1.2.3+build.169` |
| **This Project** | **3 formats** | **See above** |

Our approach combines the best of all these systems, using the right format for each purpose.
