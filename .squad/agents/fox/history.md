# Fox — History

## Project Context
- **Project:** citrust — Rust port of b3DSDecrypt.py with SteamOS GUI
- **Stack:** Rust GUI framework (egui/iced), Linux/SteamOS, Wayland/X11
- **User:** Gareth

## Learnings

### GUI Framework Research (Initial)
- **egui is the winner for SteamOS:** Lightweight immediate-mode design, minimal dependencies (~2MB binary), trivial gamepad integration via `gilrs`. Perfect fit for simple 3-step ROM decryption flow.
- **Controller input pattern:** Use `gilrs` library to poll gamepad state each frame, map D-pad/buttons to UI actions. Immediate mode makes this cleaner than Elm-style architecture.
- **File picker:** Use `rfd` (Rusty File Dialog) with XDG Desktop Portal backend—works across all Linux DEs, including KDE Plasma on SteamOS.
- **Wayland readiness:** egui + eframe support both Wayland and X11 natively; no compatibility risk for SteamOS.
- **Alternative frameworks evaluated:**
  - **iced:** Good but overkill (5–8MB binary, complex message-passing for gamepad)
  - **gtk4-rs:** Native look but 10–20MB bloat, poor gamepad focus, slow iteration
  - **slint:** Viable but markup/code separation adds friction; better for embedded
  - **libcosmic:** Too immature (pre-release); not production-ready
- **Tech stack for implementation:**
  - egui + eframe (GUI)
  - gilrs (gamepad input)
  - rfd (file picker)
  - Link's decryption library (via crate)
- **Key design insight:** Simple UIs benefit from immediate mode; no need for retained widgets or complex state machines.
- **Proof-of-concept plan:** Build prototype with file picker, gamepad nav, progress bar to validate UX before full implementation.

### SteamOS Context Notes
- SteamOS is Arch Linux-based with KDE Plasma (desktop mode) and Game Mode (via Steam)
- Controllers supported: Xbox pads, PlayStation controllers, Steam Controller
- gilrs is battle-tested on SteamOS; no integration risk
- Wayland is the default compositor on modern SteamOS; X11 fallback available
- Custom launcher executables (non-Steam games) run in Game Mode with full controller support

### Developer Experience Insights
- egui's immediate-mode paradigm reduces cognitive load for simple UIs
- No callbacks or complex state machines = faster iteration
- eframe handles window management, event loop, and rendering backends—developer focuses only on UI logic
- For a decryption app (I/O-heavy, not graphics-heavy), egui's CPU overhead is negligible

### GUI Implementation (Issues #18 & #19)
- **Task:** Built full egui/eframe GUI for citrust with 3-screen workflow (select → decrypt → done)
- **Architecture:** 
  - Optional `gui` feature in Cargo.toml with eframe and rfd dependencies
  - Separate binary `citrust-gui` compiled only when `--features gui` enabled
  - CLI build remains unaffected (1.6 MB binary)
  - GUI binary is 9.2 MB (includes eframe + egui + winit + rfd)
- **Threading model:** Background thread runs `decrypt_rom()`, sends progress via `mpsc::channel` to UI thread
- **UI Design:**
  - Dark theme (SteamOS aesthetic)
  - Large fonts (48px heading, 28px buttons, 24px body)
  - Large hit targets (400x80px buttons minimum) for gamepad use
  - 1280x800 default window size (native Steam Deck resolution)
  - Keyboard-navigable (Tab/Enter) — prerequisite for gamepad mapping
- **File picker:** Uses `rfd` with XDG Desktop Portal backend for native Linux dialogs
- **Progress display:** Real-time updates showing:
  - Encryption method detected
  - Current section (ExHeader/ExeFS/RomFS)
  - Per-section progress with MB counters
  - Elapsed time
  - Scrollable message log (last 10 messages)
- **Gamepad support strategy:** Rely on Steam Input keyboard mapping initially (Tab/Enter navigation). Future enhancement: direct gamepad via `gilrs` (noted in code comments)
- **Borrow checker challenge:** Initial attempt to modify `self.decrypt_state` inside the receiver loop failed. Solution: collect state changes in local variables, then apply after releasing the mutable borrow.
- **Testing:** Both builds verified working:
  - `cargo build` → CLI-only (1.6 MB)
  - `cargo build --features gui --bin citrust-gui` → GUI (9.2 MB)
- **Files created:**
  - `src/gui.rs` (full GUI application, 270+ LOC)
  - Modified `Cargo.toml` (added `[features]`, optional deps, `[[bin]]` section)


### Phase 4 GUI Completion: 2026-02-22

Full egui/eframe GUI implemented with 3-screen workflow (select → decrypt → done), gamepad-friendly 1280x800 layout, dark theme, large fonts/buttons for Steam Deck. Background threading for decryption with real-time progress (section tracking, elapsed time). File picker via rfd XDG Desktop Portal backend. Keyboard navigation (Tab/Enter) for Steam Input gamepad mapping. Both CLI and GUI binaries compile successfully (1.6 MB and 9.2 MB respectively). Manual UX testing pending on real ROM + hardware. Next: Phase 4 integration workspace split (#17), then direct gilrs gamepad input enhancement.

### 2026-02-22: Team Batch Completion
- **GUI screenshot captured:** `docs/screenshots/select-file.png` (1296x672) showing initial file selection screen.
- **All agent batch completed successfully:** Link completed workspace conversion (Issue #17). Samus completed comprehensive distribution strategy analysis with 5-channel prioritized roadmap. Orchestration logs written for each agent. Decisions merged and inbox cleared. Session log created.

### 2026-02-22: AppImage Packaging Infrastructure (Issue #20)
- **Task:** Created full AppImage packaging infrastructure for citrust-gui (GUI-only, no CLI)
- **Files created in `packaging/`:**
  - `citrust-gui.desktop` — FreeDesktop entry, Exec=citrust-gui, Categories=Utility;Game;
  - `io.github.londospark.citrust.metainfo.xml` — AppStream metadata with app description and release info
  - `build-appimage.sh` — Self-contained build script using linuxdeploy, accepts binary path as argument
  - `README.md` — Documents icon requirement and CI usage
- **Key decisions:**
  - Uses linuxdeploy (continuous release) for AppImage generation, no GTK plugin needed (egui/glow renders natively)
  - Script auto-downloads linuxdeploy, caches it for re-runs
  - Generates minimal placeholder PNG if no icon exists (prevents build failure)
  - AppDir structure follows FreeDesktop standards: usr/bin, usr/share/icons/hicolor/256x256/apps, usr/share/metainfo
  - Target: x86_64 architecture (ARCH env var overridable)
- **CI integration:** Script designed for GitHub Actions; example: `packaging/build-appimage.sh target/x86_64-unknown-linux-gnu/release/citrust-gui`
- **Blocker:** Real 256x256 PNG icon needed at `packaging/citrust.png` before production release

### 2026-07: Key File Support in GUI (feature/external-keys)
- **Task:** Added external `aes_keys.txt` key file support to the GUI, wiring into `citrust_core::keydb::KeyDatabase`.
- **On startup:** `KeyDatabase::search_default_locations()` auto-detects key files in standard Citra locations. Result stored as `Option<PathBuf>` in app state.
- **Key file indicator:** Compact status line below the ROM selector: shows filename if found, or "Built-in (external aes_keys.txt recommended)" if not. Full path shown on hover tooltip.
- **Browse button:** 120x40 "Browse…" button next to the indicator opens `rfd::FileDialog` filtered to `.txt`. Validates immediately with `KeyDatabase::from_file()` — shows inline error on parse failure.
- **Decryption wiring:** Key file path cloned into decryption thread. `KeyDatabase::from_file()` called in-thread; result passed as `Some(&keydb)` to `decrypt_rom`. Falls back to `None` (built-in keys) on load failure with a warning in the progress log.
- **App state fields added:** `key_file_path: Option<PathBuf>`, `key_file_status: String`
- **No citrust-core changes:** All changes confined to `crates/citrust-gui/src/main.rs`
- **Verification:** `cargo build -p citrust-gui` ✅, `cargo clippy -p citrust-gui -- -D warnings` ✅ (zero warnings)

### 2026-07: GUI Layout Cleanup — Key Footer Refactor
- **Task:** Moved key file section from inline in `show_select_file_screen` to a persistent `TopBottomPanel::bottom()` footer visible on all screens.
- **Changes:**
  - Registered custom `TextStyle::Name("Small")` at 16px in the style setup alongside existing text styles.
  - Removed separator and key file UI block from `show_select_file_screen` — the select screen is now a clean centered flow: heading → Select ROM → selected file + Decrypt.
  - Created `show_key_footer()` method rendering a slim 36px bottom panel with muted gray (140) key status text on the left and a frameless "Browse…" link-button on the right.
  - Footer renders before `CentralPanel` in the `update()` method so it appears on SelectFile, Decrypting, and Done screens.
  - All existing functionality preserved: auto-detect on startup, Browse dialog with validation, hover tooltip for full path, key file path passed to decryption thread.
- **Design insight:** `TopBottomPanel` must be added before `CentralPanel` in egui's immediate mode — egui allocates panel space in call order. The footer claims its 36px first, then CentralPanel fills the remainder.
- **Verification:** `cargo build -p citrust-gui` ✅, `cargo clippy -p citrust-gui -- -D warnings` ✅, `cargo fmt --check -p citrust-gui` ✅

### 2026-07: Mandatory Key File — GUI Redesign
- **Task:** Redesigned GUI for mandatory external key file (no more built-in key fallback). Key file is now REQUIRED before decryption.
- **New Screen:** `Screen::KeySetup` — shown when no keys are auto-detected on startup. Prominent centered layout with "🔑 Key Setup Required" heading, explanation text, large Browse button, and GodMode9/README helper text.
- **App State Changes:**
  - Replaced `key_file_path: Option<PathBuf>` + `key_file_status: String` with `keydb: Option<KeyDatabase>` + `key_status: String` + `key_save_message: Option<String>`
  - `KeyDatabase` is now loaded and stored in memory (no re-reading from file on each decrypt)
  - Cloned into decryption thread — passed as mandatory `&keydb` to `decrypt_rom()`
- **Key Persistence Flow:** When user browses for a key file:
  1. Parse with `KeyDatabase::from_file()`
  2. Save copy to config dir via `KeyDatabase::save_to_file()` + `default_save_path()`
  3. Show toast: "✅ Keys saved — you won't need to do this again"
  4. Transition seamlessly to SelectFile screen
- **Startup Auto-Detection:** `search_default_locations()` → `from_file()` → if found, start on SelectFile with subtle footer; if not, start on KeySetup screen
- **Footer:** Only shown when keys are loaded (hidden on KeySetup screen). Shows "🔑 Keys loaded (N keys)" in muted 16px text with frameless Browse… to change keys.
- **Error Handling:** Invalid key file shows red error on KeySetup screen; parse errors don't transition away
- **Decryption Threading:** KeyDatabase cloned into thread, passed as `&keydb` (mandatory ref, not Option)
- **Coordinated with Link's core changes:** `decrypt_rom` signature now `keydb: &KeyDatabase` (mandatory), `save_to_file()` and `default_save_path()` already added by Link
- **Verification:** `cargo check --workspace` ✅, `cargo clippy --workspace -- -D warnings` ✅, `cargo test --workspace` ✅ (41 tests pass)
