# GitHub Actions Workflows

This directory contains the build workflows for the SANKEY Copier project. The workflows have been modularized for better maintainability and reusability.

## Workflow Structure

### Main Orchestrator
- **`build.yml`** (107 lines)
  - Main entry point that coordinates all build steps
  - Generates version information
  - Calls reusable workflows for each component
  - Handles workflow dispatch with build target selection

### Reusable Workflows

- **`build-rust.yml`** (149 lines)
  - Builds all Rust components (DLL, Server, Tray App)
  - Runs tests for each component
  - Uploads build artifacts

- **`build-web.yml`** (63 lines)
  - Builds the Next.js web UI
  - Handles pnpm dependencies and caching
  - Runs linter and builds for production

- **`build-mql.yml`** (589 lines)
  - Compiles MQL4/MQL5 Expert Advisors
  - Manages MetaTrader installation and caching
  - Handles both MT4 and MT5 platforms via matrix strategy

- **`build-installer.yml`** (330 lines)
  - Packages all components into Windows installer
  - Uses Inno Setup for installer creation
  - Creates GitHub releases for tagged versions

### Composite Actions

Located in `.github/actions/`:

- **`setup-rust/action.yml`**
  - Configures Rust toolchain with caching
  - Supports multiple targets and workspaces

- **`setup-node/action.yml`**
  - Sets up Node.js with pnpm
  - Configures pnpm store caching

## Benefits of Modularization

1. **Improved Readability**
   - Main workflow reduced from 977 to 107 lines (89% reduction)
   - Each component has its own focused workflow

2. **Better Maintainability**
   - Changes to one component don't affect others
   - Easier to understand and debug individual workflows

3. **Reusability**
   - Workflows can be called from other workflows
   - Common actions are extracted and shared

4. **Parallel Execution**
   - Independent workflows can run in parallel
   - Faster overall build times

5. **Selective Execution**
   - Can trigger specific component builds via workflow_dispatch
   - Reduced resource usage when only one component changes

## Workflow Dispatch

The main workflow supports manual triggering with build target selection:

- `all` - Build all components (default)
- `rust-dll` - Build only Rust DLL
- `rust-server` - Build only Rust Server
- `web-ui` - Build only Web UI
- `mql` - Build only MQL components

## Total Lines of Code

- **Before**: 977 lines (single monolithic file)
- **After**: 1,238 lines (modular structure)
  - Main orchestrator: 107 lines
  - Component workflows: 1,131 lines
  - Improved organization and maintainability

## Artifacts

Each workflow uploads its build outputs as artifacts:

- **Retention**: 1 day for intermediate artifacts, 90 days for final installer
- **Naming**: Clearly identifies component and version
- **Final Installer**: Downloads and packages all component artifacts
