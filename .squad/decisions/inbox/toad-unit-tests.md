# Unit Test Architecture Decision

**By:** Toad (Tester)  
**Date:** 2026-02-22  
**Status:** Implemented  
**Category:** Testing

## Decision

Unit tests for Phase 1 modules (keys, crypto, ncsd, ncch) are implemented as inline `#[cfg(test)]` modules within each source file, not in a separate `tests/` directory.

## Rationale

**Why inline modules:**
- **Locality:** Tests are adjacent to the code they test, making them easier to maintain
- **Access:** Can test private functions and internal implementation details
- **Convention:** Standard Rust practice for unit tests (integration tests go in `tests/`)

**Coverage approach:**
- **NIST test vectors:** AES-128-CTR validated against NIST SP 800-38A published test vectors
- **Synthetic fixtures:** NCSD/NCCH headers constructed in-memory (512 bytes) for parsing tests
- **Known values:** Crypto functions validated with manual calculations for test vectors

## Test Statistics

- **Total tests:** 19 (all passing)
- **keys.rs:** 3 tests (method mapping, constants validation, flag conversion)
- **crypto.rs:** 8 tests (rol128 edge cases, key derivation, AES-CTR, conversions)
- **ncsd.rs:** 4 tests (header parsing, magic validation, sector size, partition helpers)
- **ncch.rs:** 4 tests (header parsing, crypto detection, flags, IV construction)

## Benchmark Infrastructure

**criterion benchmarks (benches/crypto_bench.rs):**
- rol128 throughput (small/large shifts)
- derive_normal_key throughput
- aes_ctr_decrypt throughput (1MB and 16MB buffers)
- Full key derivation pipeline (KeyX → NormalKey)

**PowerShell comparison script (benches/compare.ps1):**
- Takes ROM path as argument
- Copies ROM twice (Rust/Python)
- Times both decrypters
- SHA256-verifies outputs match
- Reports speedup ratio
- Cleans up temporary files

## Next Steps

- Issue #8 (integration tests): Create round-trip encryption/decryption tests
- Use benches/compare.ps1 to validate Phase 1 performance vs Python baseline
- Consider adding property-based tests for crypto functions (quickcheck/proptest)
