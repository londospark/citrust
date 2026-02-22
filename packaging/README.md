# citrust Packaging

This directory contains files for building the citrust-gui AppImage.

## Files

| File | Description |
|------|-------------|
| `citrust-gui.desktop` | FreeDesktop `.desktop` entry for the application |
| `io.github.londospark.citrust.metainfo.xml` | AppStream metadata for software centers |
| `build-appimage.sh` | Script to build the AppImage using linuxdeploy |
| `citrust.png` | Application icon (**placeholder needed**) |

## Icon Needed

An application icon is required at `packaging/citrust.png` before building a production AppImage.

**Requirements:**
- Format: PNG
- Recommended size: 256×256 pixels (minimum)
- Should be recognizable at small sizes (e.g., 48×48 in taskbars)

The build script will generate a minimal 1×1 pixel placeholder if no icon is present, but a real icon should be provided before any release.

## Building an AppImage

```bash
# First, build the GUI binary
cargo build --release -p citrust-gui

# Then create the AppImage
./packaging/build-appimage.sh target/release/citrust-gui
```

The output will be `citrust-gui-x86_64.AppImage` in the `packaging/` directory.

## CI Integration

The `build-appimage.sh` script is designed to run in GitHub Actions. Example usage in a workflow:

```yaml
- run: cargo build --release --target x86_64-unknown-linux-gnu -p citrust-gui
- run: chmod +x packaging/build-appimage.sh && packaging/build-appimage.sh target/x86_64-unknown-linux-gnu/release/citrust-gui
```
