# Privacy Policy

_Last updated: 2026-08-27_

## Summary

GTAV Mods Manager is **offline-first**: in its current form, it does not collect,
transmit, or store any of your data anywhere except your own machine. There is no
first-party server, no account system, and no telemetry.

## What the Software stores, and where

Everything the Software tracks — your detected game path, installed-mod records,
install/uninstall history, settings, and the recycle bin — is kept in a local SQLite
database and local folders under your OS's standard per-user application-data
directory (e.g. `%APPDATA%\SpaceSquare\GTAVModsManager` on Windows). None of this is
transmitted anywhere. Uninstalling the Software and deleting that folder removes all
of it.

## Network activity in the current version

As of this version, the Software's core install/uninstall/conflict-detection/backup
functionality performs **no network requests at all**. It only reads and writes files
on your local disk and your local database.

## Planned features and their intended data handling

The following are **not yet implemented**. This section describes the privacy design
they are planned to follow once built, so you know what to expect — it does not
describe current behavior.

- **Mod source integration** (e.g. checking GTA5-Mods.com/LCPDFR.com for update
  information): will only send the minimal identifiers needed for that lookup (e.g. a
  mod name/version), never personal information.
- **AI assistance** (optional, off by default when built): local inference (e.g. via
  Ollama) will keep all data on your machine. If you choose to configure a cloud AI
  provider instead, your device will connect **directly** to the provider you
  configured, using your own API key — this project will not operate any relay server
  and will never see that traffic.
- **Crash/error reporting** (optional, off by default when built): will pre-fill a
  GitHub Issue draft with de-identified diagnostic information (app version, OS
  version, error text) — with no file paths, usernames, or machine identifiers — and
  require you to review and manually submit it. Nothing is sent automatically.

## Children's privacy

The Software does not perform age verification (see [TERMS.md](TERMS.md) §9) and does
not knowingly collect data from anyone, since it does not collect data from anyone —
see above.

## Changes to this policy

If a future version adds functionality that changes this policy (e.g. an actually
implemented AI feature or crash reporter), this file will be updated to describe the
real, current behavior at that time, and the "Last updated" date above will change.

## Contact

Questions about this policy: open an issue on
[this repository](https://github.com/SpaceSquare640/GTAV_Mods_Manager/issues).
