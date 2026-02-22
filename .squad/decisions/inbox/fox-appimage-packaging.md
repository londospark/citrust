# Decision: AppImage Packaging for citrust-gui

**By:** Fox (GUI Dev)  
**Date:** 2026-02-22  
**Status:** Implemented  
**Category:** Infrastructure  
**Issue:** #20

## Summary

Created AppImage packaging infrastructure in `packaging/` directory for the GUI-only application (`citrust-gui`).

## Key Decisions

1. **GUI-only AppImage:** The AppImage packages only `citrust-gui`, not the CLI. CLI users download the standalone binary.
2. **linuxdeploy tooling:** Uses `linuxdeploy` (continuous release) to build the AppImage. No GTK plugin needed since egui/glow renders via OpenGL natively — no GTK runtime dependency.
3. **Self-contained build script:** `build-appimage.sh` auto-downloads and caches linuxdeploy, accepts the binary path as the only argument. Suitable for CI and local use.
4. **Placeholder icon fallback:** Script generates a minimal 1×1 PNG placeholder if `packaging/citrust.png` is missing, so CI won't fail before a real icon is provided.
5. **AppStream ID:** `io.github.londospark.citrust` — follows reverse-DNS convention for the GitHub org.

## Files

- `packaging/citrust-gui.desktop` — Desktop entry
- `packaging/io.github.londospark.citrust.metainfo.xml` — AppStream metadata
- `packaging/build-appimage.sh` — Build script
- `packaging/README.md` — Documentation and icon requirements

## CI Integration

The release workflow (`release.yml`) should add a step that builds the GUI binary and runs the build script. Example:

```yaml
- run: cargo build --release --target x86_64-unknown-linux-gnu -p citrust-gui
- run: chmod +x packaging/build-appimage.sh && packaging/build-appimage.sh target/x86_64-unknown-linux-gnu/release/citrust-gui
```

## Action Required

- A 256×256 PNG icon is needed at `packaging/citrust.png` before production release.
- The release workflow should be updated to include AppImage build and upload steps.
