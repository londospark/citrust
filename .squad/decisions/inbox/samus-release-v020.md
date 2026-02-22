# Decision: v0.2.0 Release Process

**By:** Samus (Lead)  
**Date:** 2026-07-16  
**Status:** Executed  
**Category:** Release

## Summary

Cut v0.2.0 release following git flow: develop → master merge, version bump, tag on master, sync back to develop.

## What shipped in v0.2.0

1. **AppImage packaging** (Issue #20, PR #23) — GUI-only AppImage via linuxdeploy
2. **Content-based decryption detection** — Detects already-decrypted ROMs even when NoCrypto flag is wrong
3. **Reverse NoCrypto detection** — Handles ROMs with NoCrypto flag set but content still encrypted
4. **Dependency updates** — Cargo dependency refresh
5. **CI fixes** — Workflow improvements for GUI builds

## Release artifacts (built by CI on v* tag)

- `citrust` — Linux CLI binary (x86_64)
- `citrust-gui` — Linux GUI binary (x86_64)
- `citrust.exe` — Windows CLI binary (x86_64)
- `citrust-gui-*.AppImage` — Linux GUI AppImage (portable)

## Process decisions

- **Version bump location:** All 3 member crate Cargo.toml files (workspace root has no version)
- **Metainfo update:** Must add release entry to `packaging/io.github.londospark.citrust.metainfo.xml` before tagging
- **Commit message files:** Use `-m` for simple messages; use `-F tmpfile` for complex multiline messages to avoid shell quoting issues
- **Tag format:** `v0.2.0` (matches release workflow trigger `v*`)

## Follow-up items

- [ ] Replace `packaging/citrust.png` placeholder with real 256×256 PNG icon
- [ ] SteamOS/Steam Deck hardware testing
- [ ] Flatpak packaging (Priority #1 from distribution strategy)
