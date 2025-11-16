# Copilot Instructions

## System Snapshot
- `relay-server/` (Axum/Tokio) is the canonical API/WebSocket/ZeroMQ hub; skim `docs/architecture.md` before touching cross-component logic.
- Ports, DB path, CORS, and ZeroMQ sockets come from `relay-server/config.toml` (default API 3000, Web UI 8080, ZMQ 5555/5556/5557) with overrides `config.{ENV}.toml` → `config.local.toml`; never hardcode values.
- `web-ui/` is a Next.js 16 App Router app with Intlayer-based i18n; `app/[locale]/page.tsx` drives the node graph UI rendered via `@xyflow/react`.
- `desktop-app/` wraps the Web UI in Tauri, spawning the Next standalone server on a free port; `NEXT_BUILD_MODE=export` switches Next to static output for bundling.
- `tray-app/` (Rust) controls Windows services `SankeyCopierServer` and `SankeyCopierWebUI` via NSSM; it reads the same `config.toml` to open the correct browser URL.
- MetaTrader EAs live in `mt-advisors/MT4|MT5` and rely on the `mt-bridge/` DLL for ZeroMQ + MessagePack serialization; respect the dual-build targets (i686/x86_64).

## Running the Core Stack
- Rust backend: `cd relay-server && cargo run --release` or `.
























- For onboarding your own environment, follow `docs/setup.md` before running MT terminals; it documents MT installation detection used by the Desktop App installer.- Troubleshooting: `docs/troubleshooting/*.md` for DLL issues, service failures, and Cloudflare tunnel hiccups.- Deployment: `docs/CLOUDFLARE_SETUP.md`, `docs/VERCEL_DEPLOYMENT.md`, `docs/operations.md`.- Architecture/flow: `docs/architecture.md`, `docs/data-model.md`, `docs/api-specification.md`.## Most Useful References- Installer pipeline expects: web build (`web-ui`), Tauri desktop (`desktop-app`), tray app, and installer (`installer/build-installer.ps1`). Keep artifact names stable or update scripts/workflows accordingly.- ZeroMQ message topics are the account IDs; slaves subscribe to both trade (5556) and config (5557). Changing topic formats requires coordinated changes in MQL, `relay-server/src/zeromq/`, and the tests in `tests/test_zmq_communication.py`.- Rust services embed Windows file version info; set `PACKAGE_VERSION`/`FILE_VERSION` in CI when producing release binaries (see `relay-server/README.md`).- Intlayer dictionaries live next to components (`*.content.ts`); new strings must satisfy the typed schema and use `useIntlayer` hooks.- Web UI rewrites to the backend via Next middleware/proxy (`web-ui/proxy.ts`); keep REST calls under `/api/*` or `/ws` so environments (Vercel, desktop, tunnel) keep functioning.- Prefer pnpm across Node workspaces; lockfile is `pnpm-lock.yaml`. Scripts assume pnpm (PowerShell helpers call it explicitly).## Project Conventions & Gotchas- When adjusting MetaTrader messaging, sync updates across `docs/api-specification.md`, Rust models under `relay-server/src/models/`, and the MQL EAs.- Web UI E2E lives in `web-ui/__tests__` using Playwright with mocked APIs; commands are `pnpm test:e2e`, `pnpm test:e2e:ui`, etc. Install browsers once via `pnpm exec playwright install`.- Protocol regression tests are Python-based in `tests/`; run `pip install -r tests/requirements.txt && pytest -v` for MessagePack + ZeroMQ coverage.- Rust unit/integration tests live beside their crates (`cargo test` in `relay-server`, `mt-bridge`, `tray-app`).## Testing Matrix- Windows tray: `cd tray-app && cargo run` while ensuring NSSM-installed services exist (see `tray-app/service.rs` for service names).- Desktop mode: `cd desktop-app && pnpm install && pnpm run dev` for Tauri, or `pnpm run build` followed by `cargo tauri build` under `src-tauri`.- Web UI: `cd web-ui && pnpm install && pnpm dev` (Next listens on 8080 per `package.json`); `start-dev.ps1` also clears `.next` and frees the port.elay-server\start-server.ps1` (kills port 3000, sets `CONFIG_ENV=dev`).