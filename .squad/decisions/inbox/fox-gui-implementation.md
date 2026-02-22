# GUI Implementation Complete (Issues #18 & #19)

**By:** Fox (GUI Dev)  
**Date:** 2026-02-22  
**Status:** Implemented  
**Category:** Implementation

## Summary

Completed full egui/eframe GUI implementation for citrust with gamepad-friendly design targeting SteamOS.

## Implementation Details

**Architecture:**
- Optional `gui` feature with `eframe` and `rfd` dependencies
- Separate binary `citrust-gui` (9.2 MB) — CLI unaffected (1.6 MB)
- Background threading for decryption with `mpsc::channel` progress updates
- Three-screen workflow: File Selection → Decrypting → Done

**UI Design (Gamepad-Friendly):**
- Dark theme matching SteamOS aesthetic
- Large fonts: 48px heading, 28px buttons, 24px body text
- Large hit targets: 400x80px buttons (minimum)
- 1280x800 default window (Steam Deck native resolution)
- Full keyboard navigation (Tab/Enter) for gamepad mapping via Steam Input

**Screen Flow:**

1. **Select File Screen:**
   - Title: "citrust — 3DS ROM Decrypter"
   - "Select ROM File" button → rfd native file dialog (*.3ds filter)
   - Shows selected path
   - "Decrypt" button appears after selection

2. **Decrypting Screen:**
   - Shows file name
   - Shows encryption method (Original/Key7x/Key9x/etc.)
   - Real-time progress messages (scrollable log)
   - Current section tracker (ExHeader → ExeFS → RomFS)
   - Elapsed time display
   - Warning: Cannot cancel (in-place modification)

3. **Done Screen:**
   - Success message with total duration
   - "Decrypt Another" button (resets to screen 1)
   - "Quit" button (closes app)

**Technical Choices:**

- **rfd with XDG Portal backend:** Native Linux file dialogs, works across all DEs
- **Wayland + X11 support:** eframe features enabled for both
- **Steam Input reliance:** No `gilrs` yet — Steam Input maps gamepad to keyboard (Tab/Enter)
  - Future enhancement path documented in code
- **Threading model:** Decryption on background thread, UI polls channel each frame
- **Borrow checker pattern:** Collect state changes in locals, apply after releasing mutable borrow

## Build Commands

```bash
# CLI only (Phase 1-3)
cargo build

# GUI (Phase 4)
cargo build --features gui --bin citrust-gui
```

## Binaries

- `target/debug/citrust.exe` — 1.6 MB CLI
- `target/debug/citrust-gui.exe` — 9.2 MB GUI

## Files Modified/Created

- **Created:** `src/gui.rs` (270+ LOC)
- **Modified:** `Cargo.toml` (added `[features]`, optional deps, `[[bin]]` section)

## Testing Status

- ✅ CLI build unaffected
- ✅ GUI builds with `--features gui`
- ✅ Both binaries compile successfully
- ⚠️ Manual UX testing pending (requires test ROM and actual hardware/SteamOS)

## Future Enhancements

1. **Direct gamepad input via `gilrs`:**
   - Detect controller state each frame
   - Map D-pad/buttons to focus movement + selection
   - Fallback to keyboard if no controller detected

2. **Controller glyphs:**
   - Show Xbox/PlayStation button icons based on detected controller
   - Steam Input Controller Database (SICD) integration

3. **Progress granularity:**
   - Parse progress strings to show % complete per section
   - Visual progress bars (currently text-only)

4. **Error recovery:**
   - "Retry" button on error screen
   - Validation before decryption (is file already decrypted?)

## Recommendation

**Ready for Phase 4 integration testing.** Next steps:
1. Samus: Workspace split (#17) — move to `citrust-core`, `citrust-cli`, `citrust-gui` crates
2. Fox/Toad: Manual UX testing with real ROM on Linux/SteamOS
3. Fox: Add `gilrs` direct gamepad input (Issue #19 enhancement)
4. Samus: AppImage packaging (#20)
