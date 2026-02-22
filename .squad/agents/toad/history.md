# Toad — History

## Project Context
- **Project:** citrust — Rust port of b3DSDecrypt.py with SteamOS GUI
- **Stack:** Rust tests, crypto correctness verification
- **User:** Gareth

## Learnings

### Analysis Run: 2026-02-22

**Code Analysis Summary:**
- Identified 4 critical correctness concerns in b3DSDecrypt.py: integer overflow in key derivation, counter offset precision loss, missing default case for KeyX selection, and endianness handling quirks
- All test ROMs (Pokemon Y/AlphaSapphire/OmegaRuby, 1.7–1.8 GB each) are DECRYPTED—no encrypted test fixtures available
- The code has no error handling for invalid partition_flags[3] values; a default case would prevent crashes

**3DS ROM Format Findings:**
- NCSD (outer) header has partition table at 0x120; each entry is 8 bytes (offset, length) in little-endian
- NCCH (per-partition) headers have encryption flags at +0x188/+0x18B/+0x18F; crypto method is stored separately from flags
- The code reads flags[3] to select between 4 hardware keys (0x2C, 0x25, 0x18, 0x1B)
- Pokemon Y ROM uses crypto method 0x63, flags 0x0F—flagged as DECRYPTED (NoCrypto bit already set)

**Test Strategy:**
- Option A (re-encryption) is best: take decrypted test ROM, reverse-encrypt using known key derivation, verify decryption recovers original
- Synthetic minimal fixtures (< 10 MB) recommended for unit tests of each encryption path
- Need separate test ROMs for each KeyX variant—availability unknown; may require fabrication

**Crypto Implementation Notes:**
- Key derivation uses 128-bit rotate-left (ROL) function; Rust should use u128 with wrapping arithmetic
- AES-CTR counter calculation for ExeFS .code section has integer division (line 162); division by 0x10 must preserve precise offset
- All offsets are in sectors, not bytes; multiplication by sectorsize is critical

### Unit Tests Implementation: 2026-02-22

**Issue #7 Complete:**
- Added comprehensive unit tests to src/keys.rs, src/crypto.rs, src/ncsd.rs, src/ncch.rs
- All 19 tests pass successfully
- Coverage includes:
  - keys.rs: CryptoMethod mapping, key constants validation, key_x_for_method correctness
  - crypto.rs: rol128 edge cases, derive_normal_key known vectors, AES-128-CTR NIST test vectors
  - ncsd.rs: NCSD header parsing, magic validation, sector size calculation, partition entry helpers
  - ncch.rs: NCCH header parsing, crypto method detection, flag parsing (NoCrypto/FixedKey), IV construction

**Issue #9 Complete:**
- Added criterion dev-dependency to Cargo.toml
- Created benches/crypto_bench.rs with criterion benchmarks for rol128, derive_normal_key, aes_ctr_decrypt (1MB/16MB buffers), full key derivation pipeline
- Created benches/compare.ps1 PowerShell script for Rust vs Python timing comparison with SHA256 verification
- All benchmarks compile successfully in release mode

**Test Architecture:**
- Used inline `#[cfg(test)]` modules in source files (not separate tests/ directory) for better locality
- NIST SP 800-38A test vectors used for AES-CTR validation
- Synthetic test data constructed for NCSD/NCCH header parsing (minimal 512-byte structures)

### Phase 2 Summary: 2026-02-22

Team completed Phase 2 optimization batch. Link achieved 1.50x speedup via AES-NI + 4MB chunk tuning. Phase 3 (multi-threading with rayon) pending. Fox completed Phase 4 GUI (egui/eframe, 1280x800 gamepad-friendly, 9.2 MB binary). Samus deployed GitHub Actions CI/release automation (ci.yml + release.yml for Linux/Windows binaries). All agent learnings documented in history.md; orchestration logs written to .squad/orchestration-log/.
