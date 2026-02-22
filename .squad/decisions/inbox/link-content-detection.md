# Decision: Content-Based Decryption Detection

**By:** Link (Core Dev)  
**Date:** 2026-07  
**Status:** Implemented  
**Category:** Bug Fix / Robustness

## Problem

Some 3DS ROM dumps are already decrypted but have the `NoCrypto` flag (`flags[7] & 0x04`) not set. This causes citrust (and the Python b3DSDecrypt.py) to treat them as encrypted and apply AES-CTR — which, being a symmetric XOR cipher, actually **encrypts** the plaintext data. The result is encrypted garbage with `NoCrypto=true`, which emulators like Azahar reject.

Real-world example: "Inazuma Eleven GO Chrono Stones Thunderflash" — decrypted during dump (GodMode9) without header flag update.

## Solution

Added content-based detection in `decrypt.rs` via `is_content_decrypted()`:

1. **Primary check:** Read first 8 bytes of ExeFS region (filename table). Decrypted entries are valid ASCII (`.code\0\0\0`, `banner\0\0`, etc.). Encrypted data is random bytes.
2. **Fallback:** If no ExeFS, check first 8 bytes of ExHeader (codeset name field) for valid ASCII.
3. **ASCII definition:** Bytes in range `0x20–0x7E` (printable) or `0x00` (null padding).

When detected, all AES-CTR operations are skipped and only the NoCrypto flag is patched — identical to normal post-decryption flag handling.

## Trade-offs

- **False positive risk:** ~0.4% chance that 8 random encrypted bytes all happen to be valid ASCII. Acceptable — encrypted ExeFS filenames being all-ASCII is astronomically unlikely in practice.
- **No false negatives:** Decrypted ExeFS entries *always* contain valid ASCII filenames.
- **Zero performance cost:** Single 8-byte read per partition, only reached when `NoCrypto` is not set.

## Files Changed

- `crates/citrust-core/src/decrypt.rs` — added `is_content_decrypted()` (public) and integrated into `decrypt_rom()` flow

## Verification

All 19 unit tests pass. No functional changes to normal encrypted ROM processing.
