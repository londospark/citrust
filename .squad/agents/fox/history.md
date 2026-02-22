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
