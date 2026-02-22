---
name: "3ds-rom-crypto"
description: "3DS ROM (NCSD/NCCH) encryption/decryption pipeline knowledge"
domain: "cryptography"
confidence: "high"
source: "b3DS-master/b3DSDecrypt.py analysis"
---

## Context
This skill covers the Nintendo 3DS ROM file format (NCSD/NCCH) and its AES-128-CTR encryption scheme. Essential for anyone working on citrust (the Rust port of b3DSDecrypt.py).

## Patterns

### File Structure
- NCSD header at offset 0x100 (magic: "NCSD")
- Partition table at 0x120: 8 entries × 8 bytes each (offset + length in sector units)
- Sector size: `0x200 * 2^(ncsd_flags[6])` (read flags at 0x188)
- Each partition is an NCCH container (magic "NCCH" at partition_offset + 0x100)

### Key Derivation (3DS Hardware Scrambler)
```
NormalKey = ROL128((ROL128(KeyX, 2) ^ KeyY) + Constant, 87)
```
- `KeyX`: Selected by crypto method flag (partition_flags[3])
  - 0x00 → KeyX0x2C (original, < 6.x)
  - 0x01 → KeyX0x25 (7.x)
  - 0x0A → KeyX0x18 (New3DS 9.3)
  - 0x0B → KeyX0x1B (New3DS 9.6)
- `KeyY`: First 16 bytes of partition RSA signature (offset 0x0 from partition start)
- `Constant`: [REDACTED]
- `NormalKey2C`: Always derived from KeyX0x2C (used for ExHeader + ExeFS table)
- Special case: `FixedCryptoKey` flag (flags[7] bit 0) → both keys = 0

### IV Construction
- Base: Title ID (little-endian u64 at partition_offset + 0x108)
- IV = TitleID (as big-endian) concatenated with content type counter:
  - Plain/ExHeader: `0x0100000000000000`
  - ExeFS: `0x0200000000000000`
  - RomFS: `0x0300000000000000`

### Decryption Sections (per partition)
1. **ExHeader** (0x800 bytes): AES-CTR with NormalKey2C, plain IV
2. **ExeFS filename table** (1 sector): AES-CTR with NormalKey2C, ExeFS IV
3. **ExeFS .code** (7.x/9.x only): Double-layer — decrypt with NormalKey, re-encrypt with NormalKey2C
4. **ExeFS data** (remaining sectors): AES-CTR with NormalKey2C, ExeFS IV + sector offset
5. **RomFS**: AES-CTR with NormalKey, RomFS IV

### Flag Patching (after decryption)
- Set `NoCrypto` bit (0x04) at partition_offset + 0x18F
- Clear `FixedCryptoKey` (0x01) and `CryptoUsingNewKeyY` (0x20) at same offset
- Zero the crypto-method byte at partition_offset + 0x18B

### Double-Layer .code Decryption
This is the trickiest part. For 7.x/9.x encrypted ROMs:
- The `.code` file within ExeFS is encrypted with TWO layers
- Outer layer: NormalKey (the 7.x/9.x key)
- Inner layer: NormalKey2C (the base key)
- Decryption: `plaintext = encrypt_2C(decrypt_7x(ciphertext))`
- The counter offset for .code is: `(file_offset_within_exefs + sector_size) / 16`

## Anti-Patterns
- **Don't use arbitrary-precision integers** — Rust `u128` is sufficient and much faster than bignum
- **Don't process byte-by-byte** — AES-CTR works on 16-byte blocks; process in large chunks (1 MB+)
- **Don't share file handles across threads without synchronization** — use separate handles or pread/pwrite
- **Don't forget the in-place modification pattern** — the Python tool reads and writes the same file simultaneously via two file handles
