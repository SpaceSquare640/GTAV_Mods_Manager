# Contributing

This project is currently maintained by a single author (with AI-assisted
development). It's not actively recruiting contributors yet, but the repository is
public and AGPL-3.0-licensed, and external pull requests are welcome — reviewed at
the maintainer's discretion, since there's no formal review team yet.

## Development setup

```
cargo build
cargo test
cargo run -p gtavmm-cli -- --help
```

Before submitting a change, make sure it passes what CI checks (`.github/workflows/ci.yml`):

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Project structure

- `crates/gtavmm-core` — all business logic. No UI/CLI dependencies. This is where
  most functional changes belong.
- `crates/gtavmm-cli` — CLI wrapper over `gtavmm-core`.
- `crates/gtavmm-app` — placeholder for the future Tauri + React UI (not yet
  scaffolded — see its `README.md`).

Adding support for a new mode (Enhanced SP, LSPDFR, FiveM, etc.) should mean writing a
new `ModeProvider` implementation (see `crates/gtavmm-core/src/providers/mod.rs`), not
modifying `mod_analyzer`'s classification logic.

## License and attribution

By submitting a contribution, you agree it will be licensed under this project's
[AGPL-3.0-only license](LICENSE) (code) or CC BY-SA 4.0 (data, e.g. rule/translation
content), matching whichever part of the repository it touches.

If you publish a modified version or a derivative work of this project, please
**conspicuously credit** it — e.g. in your README's opening section or an in-app
About/Credits screen — along the lines of: "Based on GTAV Mods Manager, original
author: SpaceSquare." This is a project convention layered on top of (not a
replacement for) the AGPL-3.0's own notice-preservation requirements.

## Reporting issues

Open an issue on [this repository](https://github.com/SpaceSquare640/GTAV_Mods_Manager/issues).
There's no separate security-disclosure process yet — for anything sensitive, say so
in the issue and we'll figure out a private channel.
