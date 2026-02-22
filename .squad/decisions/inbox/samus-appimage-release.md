# Decision: AppImage in Release Pipeline

**By:** Samus (Lead)
**Date:** 2026-02-22
**Status:** Implemented
**Category:** Infrastructure

## Summary

Added AppImage packaging to the GitHub Actions release workflow. Every tagged release (v*) now produces 4 artifacts:

1. `citrust` — Linux CLI binary (x86_64)
2. `citrust-gui` — Linux GUI binary (x86_64, bare)
3. `citrust.exe` — Windows CLI binary (x86_64)
4. `citrust-gui-*.AppImage` — Linux GUI AppImage (portable, zero-install)

## Key Decisions

1. **Separate `build-appimage` job** rather than building AppImage in `build-linux` — keeps concerns separated and allows parallel execution
2. **linuxdeploy continuous release** for AppImage tooling — industry standard, no pinned version (tracks upstream)
3. **`LINUX_GUI_DEPS` env var** at workflow level — matches CI workflow pattern, keeps dep list DRY
4. **`build-linux` builds both CLI and GUI** — one job, two artifacts, shares the Rust cache
5. **Desktop file and icon in `packaging/`** — standard location for Linux packaging assets

## Files Changed

- `.github/workflows/release.yml` — added `build-appimage` job, updated `build-linux` and `release` jobs
- `packaging/citrust.png` — placeholder icon (to be replaced with proper branding)
