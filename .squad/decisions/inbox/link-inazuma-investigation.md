# Inazuma ROM Double-Encryption Bug

**By:** Link (Core Dev)  
**Date:** 2026-07  
**Status:** Investigation Complete  
**Category:** Bug / Compatibility

## Summary

The decrypted Inazuma ROM fails to load in Azahar emulator because both Python and Rust decrypters RE-ENCRYPTED the already-decrypted content.

## Root Cause

The original ROM file (`Test Files\1706...VENOM Decrypted.3ds`) is a **mis-flagged dump**:
- Content is plaintext (ExHeader reads "InazumaG", ExeFS sections are `.code`, `banner`, `icon`, `logo`)
- But `NoCrypto` flag (`flags[7] & 0x04`) is **not set** — header claims content is encrypted

Both `b3DSDecrypt.py` and `citrust` check **only the flag** to decide whether to decrypt:
```python
# Python (line 56)
if (partition_flags[7] & 0x04):  # NoCrypto set → skip
```
```rust
// Rust (decrypt.rs:76)
if ncch.is_no_crypto() { continue; }
```

Since the flag says "encrypted", AES-CTR is applied to plaintext → re-encrypts it. The NoCrypto flag is then set to true. Result: content is encrypted but header says decrypted.

## Impact

- All 4 active partitions affected (P0, P1, P6, P7)
- Azahar trusts the flag, reads encrypted garbage as plaintext, fails with `Error 1`
- The logged `16384` is just the partition 0 byte offset (0x4000) — not the error itself

## Recommended Fix

Add content validation before decrypting. Options:

1. **ExeFS header check** — read the first ExeFS section name at `partition_offset + exefs_offset * media_unit + 0x00`. Valid names are `.code`, `icon`, `banner`, `logo`. If already readable ASCII → content is plaintext, skip crypto.
2. **ExHeader check** — read the codeset name at `partition_offset + 0x200`. If it's valid ASCII → already decrypted.
3. **User flag** — add `--force-decrypt` / `--skip-decrypt` CLI options for manual override.

Option 1 is most reliable because ExeFS section names have a well-defined format.

## Workaround (Immediate)

To fix the Inazuma ROM for Azahar without re-running decrypter:
- Hex-edit the ORIGINAL file at offsets 0x418F, 0xCFE2118F, 0xD02C618F, 0xD0B9C18F — change `0x00` to `0x04`
- This sets NoCrypto on all partitions without modifying content
- OR: simply load the original file in an emulator that can handle encrypted ROMs (Citra legacy)
