# GTAV Mods Manager

A cross-platform (Windows + Linux) mod manager for Grand Theft Auto V, covering Legacy/Enhanced
single-player mods, LSPDFR, and FiveM.

**Status**: core engine (Rust) in development. The desktop UI (Tauri + React) is intentionally
deferred until the core engine and CLI are functional and tested — see `crates/gtavmm-app/README.md`.

## License

AGPL-3.0-only. See [LICENSE](LICENSE). Data assets (e.g. rule/translation content, where
present) are separately licensed under CC BY-SA 4.0.

## Author

SpaceSquare

## Repository

https://github.com/SpaceSquare640/GTAV_Mods_Manager

**This is the only official distribution channel.** Releases are published via
[GitHub Releases](https://github.com/SpaceSquare640/GTAV_Mods_Manager/releases), each with a
SHA-256 checksum — verify your download against it. We take no responsibility for copies
obtained elsewhere.

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

## Legal

- [Terms of Use](TERMS.md)
- [Privacy Policy](PRIVACY.md)
- [Contributing](CONTRIBUTING.md)

## Disclaimer

This is an unofficial, non-commercial, open-source project and is not affiliated with or
endorsed by Rockstar Games or Take-Two Interactive. See [TERMS.md](TERMS.md) for details.
