# GTAV Mods Manager

A cross-platform (Windows + Linux) mod manager for Grand Theft Auto V, covering Legacy and
Enhanced single-player mods, LSPDFR, and FiveM.

## Status

The **core engine (`gtavmm-core`) and CLI (`gtavmm-cli`) are functional and tested** — install,
uninstall, enable/disable, conflict detection, recycle bin, full backups, multi-profile
switching, FiveM resource load-order resolution, an SP→FiveM add-on vehicle converter, and an
opt-in local/cloud AI assistant (crash-log diagnosis, translation drafts, a known-fix rule
library, and a Plan → confirm → execute action layer) are all implemented against real mod
samples, not just synthetic tests.

The **desktop app (`gtavmm-app`, Tauri + React) is under active development, not yet feature
complete**. It currently has a working shell (sidebar navigation, i18n with English/Traditional
Chinese) and a handful of pages wired to real backend commands (Legacy SP mod listing, the
FiveM server load-order tool, the SP→FiveM converter). The install wizard, profile switching UI,
and most workspace pages are not ported yet — the CLI remains the most complete way to use the
tool today. See the [Wiki](https://github.com/SpaceSquare640/GTAV_Mods_Manager/wiki) for a full
breakdown of what's implemented vs. planned.

**Known gap**: `.rar` archives are explicitly detected and rejected (no pure-Rust decoder
exists for the format) rather than silently mishandled — see the mod format notes in the Wiki.

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

- `crates/gtavmm-core` — all business logic: game/edition detection (Legacy/Enhanced, SP/LSPDFR,
  FiveM client/server), mod install/uninstall/enable/disable, conflict and protected-file
  checks, recycle bin, full backups, multi-profile switching, FiveM resource dependency
  resolution, an SP-add-on-vehicle-pack → FiveM converter, and an opt-in AI assistant module
  (local Ollama or a user-supplied cloud API key). No UI or CLI dependencies.
- `crates/gtavmm-cli` — command-line interface over `gtavmm-core`. Today this is the most
  complete way to use every feature; it will remain available as a power-user mode once the
  desktop app catches up.
- `crates/gtavmm-app` — the Tauri v2 + React + TypeScript desktop app. Scaffolded and under
  active development; see the Wiki for current page-by-page status.

## Development

```
cargo build
cargo test
cargo run -p gtavmm-cli -- --help
```

To run the desktop app in dev mode:

```
cd crates/gtavmm-app
npm install
npm run tauri dev
```

## Documentation

The [Wiki](https://github.com/SpaceSquare640/GTAV_Mods_Manager/wiki) has the full user-facing
documentation: getting started, installing mods, managing your library, the SP→FiveM converter,
backup/restore, the safety model, advanced tools, FAQ, and the changelog.

## Legal

- [Terms of Use](TERMS.md)
- [Privacy Policy](PRIVACY.md)
- [Contributing](CONTRIBUTING.md)

## Disclaimer

This is an unofficial, non-commercial, open-source project and is not affiliated with or
endorsed by Rockstar Games or Take-Two Interactive. See [TERMS.md](TERMS.md) for details.
