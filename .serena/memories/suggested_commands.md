PowerShell (`pwsh`) commands:
- `cd relay-server; cargo run --release` – start Axum/ZMQ backend (kills/reserves port via start-server.ps1 if needed).
- `cd relay-server; cargo test` – run Rust units/integration.
- `cd web-ui; pnpm install` then `pnpm dev` – Next.js dev server on 8080.
- `cd web-ui; pnpm test:e2e` (or `pnpm test:e2e:ui`) – Playwright UI tests (install browsers once via `pnpm exec playwright install`).
- `cd desktop-app; pnpm install; pnpm run dev` – Tauri desktop shell.
- `cd tray-app; cargo run` – Windows tray service controller.
- `tests\scripts`: `pip install -r tests/requirements.txt && pytest -v` – protocol regression suite.
- `docs/setup.md` described onboarding; follow `start-dev.ps1`/`start-server.ps1` helpers when available.