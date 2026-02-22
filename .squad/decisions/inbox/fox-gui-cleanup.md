# GUI Layout: Key File Section → Footer Panel

**By:** Fox (GUI Dev)  
**Date:** 2026-07  
**Status:** Implemented  
**Category:** UI/UX

## Decision

Moved the key file indicator and Browse button from inline in the main select-file screen to a persistent `TopBottomPanel::bottom()` footer bar. The footer appears on all three screens (SelectFile, Decrypting, Done).

## Rationale

The inline key section with separator disrupted the clean centered ROM selection flow. The primary workflow (Select ROM → Decrypt) should be the visual hero; key file management is secondary configuration that users rarely change.

## Implementation Details

- **Footer:** 36px bottom panel, 16px "Small" text style, muted gray (`Color32::from_gray(140)`)
- **Layout:** `🔑 Keys: {status}` label left-aligned, frameless "Browse…" button right-aligned
- **Panel ordering:** Footer rendered before `CentralPanel` in `update()` — required by egui's immediate-mode space allocation
- **No functionality changes:** Auto-detect, browse with validation, hover tooltip, decryption wiring all preserved

## Impact

- Only `crates/citrust-gui/src/main.rs` changed
- No citrust-core or citrust-cli modifications
- Custom `TextStyle::Name("Small")` registered at 16px for reuse in future subtle UI elements
