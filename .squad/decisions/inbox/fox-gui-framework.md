# GUI Framework Recommendation for citrust

**Date:** 2025  
**Author:** Fox (GUI Dev)  
**Status:** Recommendation  

---

## Executive Summary

After researching five major Rust GUI frameworks, I recommend **egui (via eframe)** as the best choice for citrust's SteamOS GUI. It offers the simplest development experience, minimal dependencies, fast controller input integration, and excellent Linux/Wayland compatibility—all critical for a single-ROM decryption UI that must work on Steam Deck.

---

## Frameworks Evaluated

### 1. **egui (via eframe)** ⭐ RECOMMENDED

**Strengths:**
- **Simplicity:** Immediate-mode paradigm is perfect for a simple 3-step flow (pick → decrypt → done)
- **Lightweight:** Minimal dependencies, ~2MB baseline binary. No heavy toolkit bloat.
- **Linux/Wayland:** First-class support via eframe. Works on X11 and Wayland seamlessly.
- **Controller Input:** No native support, but trivial to add via `gilrs` library. Immediate mode makes mapping buttons to UI actions straightforward—no retained state complexity.
- **Developer Experience:** Simple API, fast prototyping, clear separation of logic and UI.
- **File Picker:** Use `rfd` (Rusty File Dialog) for native file choosers. Works on all Linux DEs.
- **Gamepad UX:** Large buttons, keyboard-driven navigation—immediate mode makes this easy to implement.

**Weaknesses:**
- No native OS widgets (not an issue for our simple UI)
- Requires manual gamepad integration (not a blocker—this is intentional design)

**Binary Size & Performance:**
- ~2–4MB with all features (release mode)
- Immediate redraw model is ideal for responsive, glitch-free UI updates during decryption progress

---

### 2. **iced**

**Strengths:**
- Elm-inspired, reactive architecture is clean and maintainable
- Cross-platform, Linux/Wayland support is solid
- Better default widget library than egui
- Growing community and examples

**Weaknesses:**
- Larger binary (5–8MB typical), more dependencies
- Controller input requires `gilrs` + message-passing integration (more complex than egui's immediate mode)
- Not designed for tight gamepad UX; focus is on keyboard/mouse workflows
- Wayland input device support is patchy (controller input can be hit-or-miss on compositors like KDE Plasma)
- Overkill for a 3-step UI; unnecessary complexity

**Verdict:** Good framework but over-engineered for this use case. Better suited for complex, multi-panel applications.

---

### 3. **slint**

**Strengths:**
- Declarative `.slint` markup language is clean for UI designers
- Lightweight and performant on resource-constrained hardware
- Good for custom UIs on embedded Linux

**Weaknesses:**
- Controller input requires manual integration via `gilrs` + Rust callbacks
- Markup-code separation adds friction for tight integration
- Smaller community, fewer examples for gamepad UX
- Not as mature as egui/iced for desktop applications

**Verdict:** Viable but adds unnecessary UI-code separation. Better for embedded or IoT scenarios than desktop gaming.

---

### 4. **gtk4-rs**

**Strengths:**
- Native GTK4 look on Linux (fits SteamOS KDE Plasma environment)
- Mature, stable, extensive widget library
- True native file picker dialogs

**Weaknesses:**
- Heavy dependency chain (GTK4 + all bindings)
- Binary bloat (10–20MB+ typical); not ideal for simple apps
- Controller input requires `gilrs` + complex thread-safe callbacks to GTK main loop
- Gaming/gamepad focus is weak; GTK is designed for traditional desktop workflows
- Accessibility good but overkill for simple UI
- Development slower; more boilerplate

**Verdict:** Overengineered. Designed for full-featured desktop apps, not simple game UIs. Would slow down iteration.

---

### 5. **libcosmic (System76's iced-based toolkit)**

**Strengths:**
- Modern Rust desktop direction (memory-safe, performant)
- Iced-based (inherits good architecture)
- Potential for deep COSMIC integration (future-proofing for SteamOS/COSMIC)

**Weaknesses:**
- Brand new and unstable (not production-ready yet)
- Heavy dependency on COSMIC (ties us to a specific desktop environment)
- Overkill for single-ROM UI
- Community is tiny; examples and support are limited
- Not suitable for near-term project timeline

**Verdict:** Interesting long-term direction but too immature and risky for immediate delivery.

---

## Comparison Matrix

| Criterion | egui | iced | slint | gtk4 | libcosmic |
|-----------|------|------|-------|------|-----------|
| **SteamOS Compat** | ✅ Excellent | ✅ Good | ✅ Good | ✅ Good | ⚠️ Risky (new) |
| **Controller Input** | ✅ Simple (gilrs) | ⚠️ Complex (gilrs+messages) | ⚠️ Complex | ⚠️ Very complex | ⚠️ Complex |
| **Linux/Wayland** | ✅ Excellent | ⚠️ Good (IME issues) | ✅ Excellent | ✅ Excellent | ✅ Good |
| **Binary Size** | ✅ Tiny (2–4MB) | ⚠️ Medium (5–8MB) | ✅ Small (3–5MB) | ❌ Large (10–20MB) | ⚠️ Large |
| **Dev Simplicity** | ✅ High | ⚠️ Medium | ⚠️ Medium | ❌ Low | ⚠️ Medium |
| **File Picker** | ✅ rfd crate | ✅ rfd crate | ✅ rfd crate | ✅ Native GTK | ✅ iced-based |
| **Community** | ✅ Large | ✅ Large | ⚠️ Growing | ✅ Mature | ❌ Tiny |
| **Maturity** | ✅ Stable | ✅ Stable | ✅ Stable | ✅ Very stable | ❌ Pre-release |

---

## Controller/Gamepad Integration Plan

For **egui** (recommended), gamepad support is straightforward:

1. **Add `gilrs` crate** to poll gamepad state each frame
2. **Map buttons to UI actions:**
   - D-pad or left stick: navigate between buttons (file picker, decrypt, progress)
   - A button: confirm/select
   - B button: cancel/back
3. **Large hit targets:** Immediate mode makes it trivial to render large buttons with generous padding
4. **Focus state:** Simple bool state in egui app, toggled with D-pad
5. **Progress feedback:** Numeric % or progress bar, updated every frame without complexity

This approach is cleaner than iced's message-based architecture or gtk4's callback threading.

---

## File Picker Solution

Recommend **rfd** (Rusty File Dialog) for all frameworks:
- **Why:** XDG Desktop Portal backend automatically delegates to the correct desktop dialog (Zenity, KDE, GNOME, etc.)
- **Fallback:** Auto-detects environment; no manual configuration needed
- **Usage:** `rfd::FileDialog::new().pick_file()` — one line, blocks until user selects

For egui, this fits perfectly with the immediate-mode loop: show file picker, wait for result, update state.

---

## Recommended Tech Stack

```
┌─────────────────────────────────────┐
│  citrust SteamOS GUI (egui stack)   │
├─────────────────────────────────────┤
│ · egui + eframe (GUI framework)     │
│ · gilrs (gamepad input)             │
│ · rfd (native file picker)          │
│ · [Link's decryption lib] (via crate) │
├─────────────────────────────────────┤
│ Targets: Linux/SteamOS, X11/Wayland │
│ Bin size: ~2–4MB (release)          │
│ Dev loop: Cargo run, live iteration │
└─────────────────────────────────────┘
```

---

## Implementation Roadmap (High Level)

1. **Week 1:** Set up egui + eframe skeleton, integrate `gilrs` for basic button detection
2. **Week 2:** Build 3-step UI: file picker (rfd) → progress display → completion screen
3. **Week 3:** Integrate Link's decryption library, wire up decrypt flow
4. **Week 4:** Polish, gamepad UX testing, SteamOS Wayland testing

---

## Risk Assessment

| Risk | Probability | Mitigation |
|------|-------------|-----------|
| egui lacks advanced widgets | Low | Simple UI doesn't need them |
| gilrs missing a controller | Low | gilrs is mature, supports all major pads |
| Wayland input issues | Low | egui handles Wayland natively; gilrs tested on Wayland |
| rfd fails on KDE Plasma | Very low | XDG Portal works everywhere; Zenity fallback |

---

## Why NOT the Others (Short Takes)

- **iced:** Better for multi-panel apps; too heavy for 3-step flow. Gamepad integration more complex.
- **gtk4:** Native look is nice but adds 5–10× the binary size and development friction. Gamepad UX weak.
- **slint:** Markup separation is unnecessary for tightly integrated app. Growing but less proven.
- **libcosmic:** Too new, no release yet. Risk not worth it for near-term project.

---

## Recommendation Confidence

**9/10** — egui is the clear winner for this specific use case.

The immediate-mode paradigm, minimal dependencies, and trivial controller integration make it ideal for a simple, focused application. The trade-off (no native widgets) is irrelevant for our flat, button-heavy UI. Development velocity will be high, and the final binary will be small and efficient on SteamOS.

---

## Next Steps

1. **Proposal:** Share this with Samus (architect) for approval
2. **Spike:** Build proof-of-concept: file picker + gamepad nav + progress bar in egui
3. **Greenlight:** Once prototype validates controller UX and file picker, begin full implementation

---

## Appendix: Framework URLs

- egui: https://github.com/emilk/egui, https://docs.rs/egui/
- iced: https://github.com/iced-rs/iced, https://iced.rs/
- slint: https://github.com/slint-ui/slint, https://slint.dev/
- gtk4-rs: https://github.com/gtk-rs/gtk4-rs, https://gtk-rs.org/gtk4-rs/
- libcosmic: https://github.com/pop-os/libcosmic
- rfd: https://github.com/PolyMeilex/rfd
- gilrs: https://github.com/gilrs-core/gilrs
