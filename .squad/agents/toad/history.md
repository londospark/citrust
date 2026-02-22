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
