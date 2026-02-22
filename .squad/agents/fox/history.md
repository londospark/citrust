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
