# Reverse NoCrypto Detection

**By:** Link (Core Dev)  
**Date:** 2026-07  
**Status:** Implemented  
**Category:** Robustness

## Decision

When `NoCrypto` flag is set on a partition, we no longer blindly trust it. We verify with `is_content_decrypted()`:

1. **NoCrypto + content is plaintext** → skip decryption (as before), log "Already Decrypted ✓"
2. **NoCrypto + content is encrypted** → clear NoCrypto bit, recover backup crypto_method from NCSD offset `0x1188 + (p * 8) + 3`, re-parse NCCH header, proceed with full decryption

## Rationale

ROMs that were partially processed or had flags incorrectly set can have NoCrypto=True while the actual content remains encrypted. Without this check, these ROMs would be silently skipped, leaving the user with an undecrypted file that appears to be decrypted.

This is the reverse of the existing mis-flagged ROM detection (NoCrypto=False but content is plaintext). Together, both directions are now covered.

## Backup Flags

The NCSD header stores backup partition flags at offset 0x1188 (8 bytes per partition). When NoCrypto was set, the original flags[3] (crypto_method) might have been zeroed. The backup at `0x1188 + (p * 8) + 3` preserves the original crypto_method. If valid and non-zero, it's restored before decryption.

## Impact

- **File changed:** `crates/citrust-core/src/decrypt.rs`
- **Test added:** `test_decrypt_detects_encrypted_despite_nocrypto_flag`
- **All 25 unit tests pass**
- No changes to public API or other modules
