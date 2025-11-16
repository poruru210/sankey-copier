# GitHub Actions Workflows

This directory contains the build workflows for the SANKEY Copier project. The workflows have been modularized for better maintainability and reusability.

## Workflow Structure

### Orchestrators
- **`ci-pr.yml`**
  - Pull Request focused pipeline (and manual dispatch helper)
  - Runs `version-info`, change detection, Rust/MQL checks, Web lint/typecheck/E2E, and optional preview deploys
  - Skips Windows-heavy jobs by default but allows manual overrides (`force_*` inputs)

- **`ci-release.yml`**
  - Runs on `push main`, tags `v*`, or manual dispatch
  - Orchestrates production-grade builds (Rust DLL/server, MQL, Desktop, Installer) and Vercel production deploys
  - Shares the same versioning job and change filters to avoid unnecessary work

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

### Deployment Workflow

- **`deploy-vercel.yml`**
  - Reusable workflow invoked from `ci-pr`/`ci-release`
  - Builds and deploys the Web UI via Vercel with environment metadata
  - Skips forked PRs automatically (no secrets leakage)

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

`ci-pr.yml` and `ci-release.yml` both expose `workflow_dispatch` inputs (`force_web`, `force_rust`, `force_mql`, `force_installer`, etc.) so maintainers can override the path filters and trigger heavy builds or deployments on demand.

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
