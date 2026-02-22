# Decision: Remove All Hardcoded Crypto Keys

**By:** Link (Core Dev)  
**Date:** 2026-07  
**Status:** Implemented  
**Category:** Security / Architecture

## Summary

Removed all 9 hardcoded 3DS AES key constants from the codebase. `KeyDatabase` (loaded from external `aes_keys.txt`) is now mandatory for all decryption operations — no built-in fallback.

## Changes

1. **`keys.rs`**: Removed 9 constants + `key_x_for_method()`. Only `Key128` type and `CryptoMethod` enum remain.
2. **`decrypt.rs`**: `decrypt_rom()` signature: `Option<&KeyDatabase>` → `&KeyDatabase`. All `resolve_*` helpers simplified (no fallback branches).
3. **`main.rs` (CLI)**: Requires key file — searches default locations, exits with helpful error if none found.
4. **`integration_tests.rs`**: `make_test_keydb()` loads from file (no inline keys). `keydb_has_expected_keys` now `#[ignore]`.
5. **`crypto_bench.rs`**: Real key values replaced with arbitrary test values.
6. **`keydb.rs`**: Added `save_to_file()` and `default_save_path()`. All unit tests use arbitrary hex values.

## Impact

- **CLI users** must provide `aes_keys.txt` (via `--keys` flag or default location).
- **GUI** updated minimally to match new API — Fox should review for UX polish.
- **Integration tests** require `test-fixtures/aes_keys.txt` or keys in a default location.
- **Zero real key hex values** remain in source code.

## Rationale

Hardcoded crypto keys are a legal/distribution liability. External key files are the standard approach used by Citra, Azahar, and other emulators.
