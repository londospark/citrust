# Decision: Crypto Crate Selection for Rust Port

**Author:** Link (Core Dev)  
**Date:** 2024 (Analysis phase)  
**Status:** Proposed  
**Category:** Architecture

## Problem
Need to select minimal-dependency crates for AES-128-CTR decryption in Rust port of b3DSDecrypt.py.

## Analysis
- **Core requirement:** AES-128 encryption in CTR (counter) mode
- **Python source:** Uses `PyCryptodome` (AES + Counter utilities)
- **Rust ecosystem:** Multiple options with different maturity levels

## Proposal
Use **`aes` + `ctr` crates** for crypto:
```toml
aes = "0.8"      # NIST standardized AES implementation, well-audited
ctr = "0.9"      # CTR mode wrapper, pairs directly with aes crate
byteorder = "1.5" # For struct parsing (not crypto, but essential)
```

## Rationale
- ✅ `aes` is part of the RustCrypto ecosystem (highly trusted)
- ✅ `ctr` integrates seamlessly with `aes` via the `cipher` trait
- ✅ Together they replace Python's Counter + AES imports
- ✅ Minimal total code surface (2 core crates)
- ✅ Performance: hardware AES-NI support when available
- ✅ No alternative: `openssl` crate adds complexity; `ring` is overkill

## Alternative Considered
- **`openssl`**: Heavier, requires system OpenSSL installation
- **`ring`**: Designed for TLS; overkill for single-cipher use case

## Implementation Notes
- Native Rust `u128` type replaces Python's `long()` entirely
- No homegrown crypto implementations
- Standard `cipher::StreamCipher` trait for encryption/decryption

## Decision
🔒 **APPROVED** - Proceed with `aes` + `ctr` + `byteorder`

---
**Next:** Fox (GUI) should use same crates if needing to decrypt tiles in preview mode.
