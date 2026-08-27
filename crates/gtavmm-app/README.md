# gtavmm-app (placeholder — not implemented yet)

This will become the Tauri + React desktop UI. Per project decision, UI work is deferred until
the core engine (`gtavmm-core`) and CLI (`gtavmm-cli`) are functional and covered by tests.

Do not `tauri init` here yet — this crate is intentionally not a workspace member until that
phase begins, to avoid maintaining a half-configured Tauri/Node project through core-only
development.
