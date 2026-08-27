# GTAV Mods Manager

A cross-platform (Windows + Linux) mod manager for Grand Theft Auto V, covering Legacy/Enhanced
single-player mods, LSPDFR, and FiveM.

**Status**: core engine (Rust) in development. The desktop UI (Tauri + React) is intentionally
deferred until the core engine and CLI are functional and tested — see `crates/gtavmm-app/README.md`.

## License

AGPL-3.0-only. See [LICENSE](LICENSE).

## Author

SpaceSquare

## Repository

https://github.com/SpaceSquare640/GTAV_Mods_Manager

## Workspace layout

- `crates/gtavmm-core` — all business logic (game detection, mod install/uninstall, conflict
  detection, recycle bin, settings). No UI or CLI dependencies.
- `crates/gtavmm-cli` — command-line interface over `gtavmm-core`, used for development/testing
  before the UI exists, and doubles as the planned power-user CLI mode.
- `crates/gtavmm-app` — placeholder for the future Tauri + React desktop app (not yet scaffolded).

## Development

```
cargo build
cargo test
cargo run -p gtavmm-cli -- --help
```

## Disclaimer

This is an unofficial, non-commercial, open-source project and is not affiliated with or
endorsed by Rockstar Games or Take-Two Interactive.
