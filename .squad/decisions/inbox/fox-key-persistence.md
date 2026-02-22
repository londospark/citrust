# Decision: Mandatory Key File with Persistence in GUI

**By:** Fox (GUI Dev)  
**Date:** 2026-07  
**Status:** Implemented  
**Category:** UX / Architecture

## Context

With Link stripping all hardcoded keys from citrust-core, the key file is now mandatory. The old GUI had a tiny footer with a frameless "Browse…" link — too subtle for a mandatory requirement.

## Decision

### Key Setup Screen (first-run experience)
When no keys are auto-detected on startup, the GUI shows a dedicated `KeySetup` screen front-and-center — not an error, but a natural onboarding step. Large Browse button, explanation text, GodMode9 helper text.

### Key Persistence
When the user browses and selects a valid key file:
1. Parse it immediately with `KeyDatabase::from_file()`
2. Save a copy to the config directory (`KeyDatabase::default_save_path()`) so `search_default_locations()` finds it automatically on next launch
3. User never has to do this again — single setup

### In-Memory KeyDatabase
The loaded `KeyDatabase` is stored in app state and cloned into the decryption thread. No more re-reading from disk on each decrypt.

### Conditional Footer
The key status footer only appears when keys are loaded. On KeySetup screen, the setup IS the whole screen — no redundant footer.

## Impact
- **All agents:** GUI now requires a key file. No fallback to built-in keys.
- **CLI (already updated by Link):** Exits with error and helpful message if no keys found.
- **Testing:** All 41 workspace tests pass. GUI compiles clean with zero clippy warnings.
