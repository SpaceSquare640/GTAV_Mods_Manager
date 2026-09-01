# GTAV Mods Manager — UI Design

This branch holds the UI design source of truth for
[GTAV Mods Manager](https://github.com/SpaceSquare640/GTAV_Mods_Manager): a static
HTML/CSS/vanilla-JS mockup of every screen in the desktop app.

It is a standalone branch with **no shared history** with the `Source_Code` branch.
The two are tracked separately on purpose — the design mockup is not part of the
Rust/Tauri build, and nothing here is compiled, imported, or shipped. Changes here
get ported into `crates/gtavmm-app` on the `Source_Code` branch by hand.

## Why this branch exists

The design file previously lived outside version control entirely. On 2026-08-31 it
was accidentally overwritten — 4174 lines reduced to a 45-line fragment — and could
not be recovered from any snapshot, sync, or editor history. Putting it under git is
the fix.

The initial commit deliberately records that damaged state as-is rather than an empty
folder, so the recovery work that follows is visible in the history.

## Workflow

Design changes land here **before** they are implemented in the app. Edit the mockup,
review it in a browser, then port the result into the React components on
`Source_Code`.

## License

Part of the GTAV Mods Manager project, licensed under AGPL-3.0-only. See the
`Source_Code` branch for the full license text.
