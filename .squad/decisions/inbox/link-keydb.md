# Decision: External Key Database Support (`keydb.rs`)

**By:** Link (Core Dev)  
**Date:** 2026-07  
**Status:** Implemented  
**Category:** Architecture  
**Branch:** feature/external-keys

## Summary

Added Citra-compatible `aes_keys.txt` parser (`keydb.rs`) and wired it into `decrypt_rom()` as an optional external key source with hardcoded fallback.

## API Change

`decrypt_rom` signature changed from:
```rust
pub fn decrypt_rom(path: &Path, on_progress: impl FnMut(&str)) -> Result<(), Error>
```
to:
```rust
pub fn decrypt_rom(path: &Path, keydb: Option<&KeyDatabase>, on_progress: impl FnMut(&str)) -> Result<(), Error>
```

Passing `None` preserves exact existing behavior (hardcoded keys). Passing `Some(&keydb)` uses external keys with `Error::KeyNotFound` if a required key is missing.

## Impact

- **CLI:** Updated — `--keys <path>` argument + auto-discovery via `search_default_locations()`.
- **GUI:** Needs update by Fox — `decrypt_rom` call at line 268 of `citrust-gui/src/main.rs` needs the new `keydb` parameter (pass `None` for now).
- **Integration tests:** Updated to pass `None`.
- **Backward compatibility:** Zero behavioral change when no key file is provided.

## Files Changed

- **Created:** `crates/citrust-core/src/keydb.rs` (17 unit tests)
- **Modified:** `crates/citrust-core/src/lib.rs` (added `pub mod keydb`)
- **Modified:** `crates/citrust-core/src/decrypt.rs` (new signature + resolve helpers + `KeyNotFound` error)
- **Modified:** `crates/citrust-cli/src/main.rs` (`--keys` arg + key loading logic)
- **Modified:** `crates/citrust-core/tests/integration_tests.rs` (updated calls)
