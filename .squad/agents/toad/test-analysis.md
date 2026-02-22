# Test Analysis: b3DSDecrypt.py → Rust Port

**Analyst:** Toad 🧪  
**Date:** 2026-02-22  
**Status:** Initial Analysis  

---

## 1. Correctness Concerns

### 1.1 Off-by-One Errors & Sector Offsets

**Risk Areas Identified:**

| Line(s) | Expression | Risk | Notes |
|---------|-----------|------|-------|
| 40 | `f.seek(0x100)` | Boundary | NCSD header must start at exactly 0x100 |
| 49 | `f.seek((0x120) + (p*0x08))` | Boundary | 8 partitions × 8 bytes each = 0x40 bytes partition table |
| 53 | `f.seek(((part_off[0]) * sectorsize) + 0x188)` | **HIGH** | Partition flags at NCCH+0x188; sectorsize multiplier is critical |
| 61 | `f.seek(((part_off[0]) * sectorsize) + 0x100)` | Boundary | NCCH magic at partition_start + 0x100 |
| 74, 96, 99 | `0x160`, `0x1C0`, `0x1E0` | Moderate | Hash offsets; must align correctly |
| 81-82 | `plain_off`, `plain_len` | **HIGH** | Plain sector offset used in decryption; off-by-one here breaks decryption |
| 162, 180 | `ctroffset = ((code_fileoff[0] + sectorsize) / 0x10)` | **HIGH** | Counter offset must account for 0x200-byte AES block boundaries |
| 162 | `ctroffset = ... / 0x10` | **HIGH** | Integer division! If not exactly divisible, truncation occurs |

**Critical Offset Pattern:**
- Sector size = 0x200 * 2^(ncsd_flags[6]) — can be 0x200, 0x400, 0x800, 0x1000 bytes
- All seek operations multiply by sectorsize; values must be in sector units
- **Test this:** Verify partition offset calculations for sectorsize = 0x200, 0x400, 0x800

### 1.2 Integer Overflow in 128-bit Key Derivation

**Line 109, 129:**
```python
NormalKey = rol((rol(KeyX, 2, 128) ^ KeyY) + Const, 87, 128)
```

**Risks:**
- `rol()` function masks to 128 bits: `(2**128-1)` = `0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF`
- XOR operation on two 128-bit values is safe
- Addition `(rol(...) ^ KeyY) + Const` could overflow 128 bits without the mask
- Fortunately, the mask is applied at the end of `rol()`

**Action:** Verify Rust implementation uses u128 with proper wrapping arithmetic.

### 1.3 Endianness Handling

| Line | Operation | Endian | Risk |
|------|-----------|--------|------|
| 20-22 | Counter setup | Big-endian `'>Q'` | Format string enforces big-endian; hardcoded values |
| 23 | Constant | Big-endian `'>QQ'` | 3DS hardware constant, fixed |
| 50, 69, 78 | Partition metadata | **Little-endian `'<L'`, `'<Q'`** | NCCH header uses LE; critical! |
| 75, 97, 100 | Hash unpacking | Big-endian `'>QQQQ'` | Hashes formatted as big-endian strings; suspicious |
| 102-106 | IV/Key conversion | Big-endian in string | `"%016X%016X"` produces big-endian string, then `long(..., 16)` converts to int |

**Endianness Gotchas:**
1. **Partition metadata (offsets/lengths):** Little-endian
2. **Counter initial values:** Big-endian constants
3. **Key derivation:** Unpacked as big-endian tuples, then converted to 128-bit integers via hex strings
4. **Hash values:** Unpacked as big-endian, but only used for display (not decryption)

### 1.4 Conditional Branches (Encryption Methods)

**Decision Tree:**
```
IF partition has NoCrypto flag (0x04) set:
  → Skip partition (already decrypted)
ELSE IF NoCrypto flag NOT set:
  → Valid partition to decrypt
  
  IF FixedCryptoKey flag (0x01) set:
    → NormalKey = 0x00 (zero-key encryption)
  ELSE:
    → Use partition_flags[3] to select KeyX:
      - 0x00 → KeyX0x2C (Original Key)
      - 0x01 → KeyX0x25 (7.x Key)
      - 0x0A → KeyX0x18 (New3DS 9.3)
      - 0x0B → KeyX0x1B (New3DS 9.6)
      - (else?) → **UNDEFINED BEHAVIOR**
  
  → Decrypt ExHeader if exhdr_len > 0
  → Decrypt ExeFS if exefs_len > 0
    → If partition_flags[3] ∈ {0x01, 0x0A, 0x0B}: Re-encrypt .code with NormalKey2C
    → Decrypt rest of ExeFS with NormalKey2C
  → Decrypt RomFS if romfs_off != 0
```

**Missing Edge Cases in Code:**
- **Line 117-128:** No default case for partition_flags[3]. If value is not 0x00, 0x01, 0x0A, 0x0B, `KeyX` remains uninitialized → **crash or undefined behavior**
- The code assumes partition_flags[3] value is always one of these four

### 1.5 Flag Manipulation at End (Lines 219-225)

```python
g.seek((part_off[0] * sectorsize) + 0x18B)
g.write(struct.pack('<B', int(0x00)))  # Set crypto method to 0x00
g.seek((part_off[0] * sectorsize) + 0x18F)
flag = int(partition_flags[7])
flag = (flag & ((0x01|0x20)^0xFF))     # Turn OFF 0x01 (FixedCryptoKey) and 0x20 (CryptoUsingNewKeyY)
flag = (flag | 0x04)                   # Turn ON 0x04 (NoCrypto)
g.write(struct.pack('<B', flag))
```

**Analysis:**
- Seeks to partition NCCH header offset (part_off[0]*sectorsize)
- Writes to offset 0x18B: crypto method byte
- Writes to offset 0x18F: flags byte
- The flag AND operation: `(0x01|0x20)^0xFF` = `0x21 ^ 0xFF` = `0xDE` (preserves all bits except 0x01 and 0x20)
- **Correct approach:** Turn off specific bits, then set NoCrypto bit

---

## 2. Code Path Coverage & Test Strategy

### 2.1 Critical Code Paths

**Path 1: Zero-Key Encryption (FixedCryptoKey flag 0x01)**
- Lines 112-115
- NormalKey = 0x00
- Need a ROM encrypted with zero-key to test
- Action: Check if any test ROM uses this

**Path 2: Standard KeyX Selection (Lines 117-129)**
- KeyX0x2C (0x00) – Original Key
- KeyX0x25 (0x01) – 7.x Key
- KeyX0x18 (0x0A) – New3DS 9.3
- KeyX0x1B (0x0B) – New3DS 9.6
- Each requires a different ROM

**Path 3: ExHeader Decryption (Lines 131-139)**
- Only if exhdr_len > 0
- Uses plainIV and NormalKey2C
- Seeks to (part_off[0] + 1) * sectorsize

**Path 4: ExeFS Decryption (Lines 141-194)**
- Only if exefs_len > 0
- Two sub-paths:
  - **4a:** If partition_flags[3] ∈ {0x01, 0x0A, 0x0B}: Re-encrypt .code with NormalKey2C (Lines 150-175)
  - **4b:** Standard ExeFS decryption with NormalKey2C (Lines 177-191)

**Path 5: RomFS Decryption (Lines 196-214)**
- Only if romfs_off != 0
- Uses romfsIV and NormalKey

### 2.2 Test ROMs Available

| ROM | File | Size | Decrypted? | Expected Keys |
|-----|------|------|-----------|----------------|
| Pokemon Y | 0451 - Pokemon Y (Europe)... | 1.71 GB | YES ✓ | Unknown |
| Pokemon Alpha Sapphire | 1324 - Pokemon Alpha Sapphire... | 1.80 GB | YES ✓ | Unknown |
| Pokemon Omega Ruby | 1325 - Pokemon Omega Ruby... | 1.80 GB | YES ✓ | Unknown |

**Limitation:** All test ROMs are DECRYPTED. Need to:
1. Inspect headers to determine which KeyX was used
2. Re-encrypt one as a test fixture (reversible operation)
3. Verify decryption produces identical output

### 2.3 Test Fixture Strategy

**Option A: Re-encryption (Recommended)**
- Take a decrypted ROM (e.g., Pokemon Y)
- Extract NCCH and partition metadata
- Re-encrypt using the known key derivation
- Verify decryption returns to original

**Option B: Synthetic Minimal Test Files**
- Create tiny NCSD headers with single partition
- Populate with known plaintext + ciphertext pairs
- Test each code path with minimal I/O

**Option C: Segment-level Testing**
- Extract ExeFS/RomFS from decrypted ROM
- Re-encrypt only those sections
- Test decryption in isolation

---

## 3. Edge Cases to Test

### 3.1 Partition Boundary Cases

| Case | Trigger | Expected Behavior |
|------|---------|-------------------|
| No partition (off=0) | Check line 59 | Skip partition |
| Empty ExHeader | exhdr_len == 0 | Skip ExHeader decryption |
| Empty ExeFS | exefs_len == 0 | Skip ExeFS decryption |
| No RomFS | romfs_off == 0 | Skip RomFS decryption |
| Corrupted NCCH magic | Line 64: magic != "NCCH" | Print error, skip partition |
| NoCrypto flag set | Line 56: flags[7] & 0x04 | Skip decryption, output message |

### 3.2 Integer Boundary Cases

| Case | Computation | Risk |
|------|-----------|------|
| sectorsize overflow | 0x200 * 2^ncsd_flags[6] | Max is 0x1000; all valid |
| Counter offset overflow | (fileoff + sectorsize) / 0x10 | Could overflow if fileoff is huge |
| File size for 1MB chunks | exefs_len / (1024*1024) | Integer division truncation |
| Remainder bytes | exefs_len % (1024*1024) | Could be 0; code handles this |

### 3.3 Crypto Edge Cases

| Case | Trigger | Expected Output |
|------|---------|-----------------|
| Zero-key encryption | FixedCryptoKey flag | Plaintext appears unchanged |
| Wrong KeyX selected | Mismatch in partition_flags[3] | Garbage output |
| Corrupted partition flags | Invalid byte value | Undefined (no bounds check) |

---

## 4. 3DS ROM Format Reference (Testing Perspective)

### 4.1 Header Magic Bytes

| Offset | Magic | Purpose | Length |
|--------|-------|---------|--------|
| 0x100 | "NCSD" | Nintendo Card Secure Data (ROM header) | 4 bytes |
| Partition start + 0x100 | "NCCH" | Nintendo CTR Cart HeaDer (partition header) | 4 bytes |

### 4.2 NCSD Header Structure (Decrypted ROM)

| Offset | Field | Type | Purpose |
|--------|-------|------|---------|
| 0x000–0x0FF | — | — | Signature/header before magic |
| 0x100–0x103 | Magic | "NCSD" | |
| 0x120–0x15F | Partition Info | 8×8 bytes | Offset and length for each partition (LE) |
| 0x188–0x18F | Flags | 8 bytes | Sector size, format, etc. |
| 0x188[6] | SectorSize | 1 byte | sectorsize = 0x200 * 2^value |

### 4.3 NCCH Header Structure (Per Partition)

| Offset | Field | Type | Purpose | Endian |
|--------|-------|------|---------|--------|
| +0x000–0x0FF | RSA Signature | 256 bytes | Signature (first 16 bytes = KeyY) | — |
| +0x100–0x103 | Magic | "NCCH" | Partition magic | — |
| +0x108–0x10F | TitleID | 8 bytes | Used for IV generation | LE |
| +0x160–0x17F | ExHeader Hash | 32 bytes | SHA-256 | BE (in code) |
| +0x180–0x183 | ExHeader Len | 4 bytes | Extended header length | LE |
| +0x188–0x18B | Partition Flags | 4 bytes | Encryption/format flags | LE |
| +0x18B | Crypto Method | 1 byte | 0=original, 1=7.x, 2=9.x | LE |
| +0x18C–0x18E | Reserved | 3 bytes | — | — |
| +0x18F | Flags | 1 byte | Encryption flags (NoCrypto, FixedKey, etc.) | LE |
| +0x190–0x193 | Plain Offset | 4 bytes | Plain sector offset in sectors | LE |
| +0x194–0x197 | Plain Len | 4 bytes | Plain sector length in sectors | LE |
| +0x198–0x19B | Logo Offset | 4 bytes | Logo offset | LE |
| +0x19C–0x19F | Logo Len | 4 bytes | Logo length | LE |
| +0x1A0–0x1A3 | ExeFS Offset | 4 bytes | ExeFS offset in sectors | LE |
| +0x1A4–0x1A7 | ExeFS Len | 4 bytes | ExeFS length in sectors | LE |
| +0x1B0–0x1B3 | RomFS Offset | 4 bytes | RomFS offset in sectors | LE |
| +0x1B4–0x1B7 | RomFS Len | 4 bytes | RomFS length in sectors | LE |
| +0x1C0–0x1DF | ExeFS Hash | 32 bytes | SHA-256 | BE |
| +0x1E0–0x1FF | RomFS Hash | 32 bytes | SHA-256 | BE |

### 4.4 Partition Flags Byte (Offset +0x18F)

| Bit | Mask | Name | Meaning |
|-----|------|------|---------|
| 0 | 0x01 | FixedCryptoKey | Uses zero-key instead of derived key |
| 1 | 0x02 | NewKeyY | Uses different KeyY source |
| 2 | 0x04 | **NoCrypto** | Already decrypted (no encryption) |
| 3 | 0x08 | CryptoUsingNewKeyY | Uses NewKeyY for encryption |
| 4–6 | — | Crypto Method | Stored at +0x18B (separate) |
| 7–4 | — | Reserved | — |

### 4.5 Crypto Method Byte (Offset +0x18B)

| Value | Meaning |
|-------|---------|
| 0x00 | Key 0x2C (Original Key, < 6.x) |
| 0x01 | Key 0x25 (7.x and later) |
| 0x0A | Key 0x18 (New3DS 9.3) |
| 0x0B | Key 0x1B (New3DS 9.6) |

### 4.6 Valid ROM Validation Checklist

✓ Magic at 0x100 = "NCSD"  
✓ Magic at partition_start + 0x100 = "NCCH"  
✓ Partition offset > 0 (non-zero)  
✓ Partition length > 0  
✓ Sector size matches power of 2  
✓ Partition flags byte contains valid bits (0x00–0x0F, others reserved)  
✓ ExeFS/RomFS offsets within partition boundary  
✓ No overlapping regions  

---

## 5. 3DS Encryption Algorithm (AES-CTR)

### 5.1 Key Derivation

```
KeyY = First 16 bytes of partition RSA signature
KeyX = Hardware key selected by partition_flags[3]
Constant = 0x1FF9E9AAC5FE040802459 1DC5D52768A (hardware constant)

NormalKey = ROL(ROL(KeyX, 2, 128) ⊕ KeyY) + Constant, 87, 128)
```

Where ROL = rotate left on 128-bit value.

### 5.2 IV Generation

```
TitleID = 8 bytes from partition NCCH header +0x108
Counter = Content-type specific (0x01, 0x02, 0x03 for plain/exefs/romfs)

IV = TitleID || Counter (concatenate as big-endian)
```

### 5.3 AES-CTR Decryption

- **Cipher:** AES-128 in CTR mode
- **Key:** NormalKey
- **IV/Nonce:** Computed IV as above
- **Block Size:** 16 bytes (standard AES)
- **Counter Offset:** For ExeFS .code section, counter increments based on file offset

---

## 6. Test Plan Summary

### 6.1 Unit Tests

**Module: Key Derivation**
- [ ] Test ROL function with known values (e.g., ROL(0x1234, 2, 128) = 0x48D0)
- [ ] Test key derivation with test vectors (KeyX, KeyY, Constant → NormalKey)
- [ ] Test each KeyX variant (0x2C, 0x25, 0x18, 0x1B)
- [ ] Test zero-key (FixedCryptoKey flag)

**Module: IV Generation**
- [ ] Test IV construction for plain/exefs/romfs counters
- [ ] Test TitleID extraction and big-endian packing

**Module: AES-CTR**
- [ ] Encrypt/decrypt known plaintext with test key and IV
- [ ] Verify against standard AES-CTR (reference implementation)

### 6.2 Integration Tests

**Test Set 1: Pokemon Y (Example)**
- [ ] Parse NCSD header, verify magic and partition table
- [ ] Extract partition 0 NCCH header
- [ ] Determine encryption method from flags
- [ ] Verify NormalKey derivation
- [ ] Take encrypted partition (re-encrypted from current), decrypt, compare to original

**Test Set 2: Edge Cases**
- [ ] Empty ExHeader (exhdr_len = 0)
- [ ] Empty ExeFS (exefs_len = 0)
- [ ] No RomFS (romfs_off = 0)
- [ ] Zero-key partition

**Test Set 3: Corruption Detection**
- [ ] Corrupted NCSD magic
- [ ] Corrupted partition NCCH magic
- [ ] Invalid partition flags[3] value
- [ ] Negative/invalid offsets

### 6.3 Benchmarking (Python vs Rust)

**Metrics:**
- Time to decrypt ExeFS (varies by size, 5–100 MB typical)
- Time to decrypt RomFS (varies, 100–1000 MB)
- Memory usage (AES-CTR streaming vs buffered)

**Fair Comparison:**
- Use same test ROM (e.g., Pokemon Y, ~1.7 GB)
- Measure end-to-end wall-clock time
- Report as MB/sec throughput
- Run on same hardware, same system state

---

## 7. Findings Summary

### 7.1 Critical Issues Found

| Issue | Severity | Impact | Lines |
|-------|----------|--------|-------|
| No default case for partition_flags[3] | HIGH | Undefined KeyX → incorrect decryption | 117–128 |
| Integer division in counter offset | MEDIUM | Loss of precision, off-by-one in counter | 162, 180 |
| No bounds check on partition_flags[3] | MEDIUM | Could accept invalid key index | 117–128 |
| Endianness mismatch in hashing (unused) | LOW | Hashes only for display, not crypto | 75, 97, 100 |

### 7.2 Test Coverage Gaps

- **No test for zero-key encryption** (requires ROM with FixedCryptoKey flag)
- **No test for all four KeyX variants** (requires 4 different ROMs)
- **No synthetic test fixtures** (current test ROMs are 1.7–1.8 GB each)
- **No negative/corruption test cases** (no error handling tests)

### 7.3 Recommendations for Rust Port

1. **Add enum for encryption method** instead of magic numbers (0x00, 0x01, 0x0A, 0x0B)
   ```rust
   enum EncryptionMethod {
       Original = 0x00,      // Key 0x2C
       Seventh = 0x01,       // Key 0x25
       New3DS9_3 = 0x0A,     // Key 0x18
       New3DS9_6 = 0x0B,     // Key 0x1B
   }
   ```

2. **Add match statement (not if-else) with exhaustive pattern checking**
   ```rust
   match partition_flags[3] {
       0x00 => { use Key0x2C },
       0x01 => { use Key0x25 },
       0x0A => { use Key0x18 },
       0x0B => { use Key0x1B },
       _ => return Err("Unknown encryption method"),
   }
   ```

3. **Use u128 with wrapping arithmetic for 128-bit key derivation**

4. **Create minimal test fixtures** (< 10 MB each) for unit testing:
   - Valid NCSD header + single partition
   - Each encryption method variant
   - Zero-key variant
   - Corrupted variants

5. **Test counter offset calculation** thoroughly for fractional sector boundaries

---

## 8. Available Test ROMs (Detailed)

**Pokemon Y (Europe) v1.4**
- File: `0451 - Pokemon Y (Europe) (En,Ja,Fr,De,Es,It,Ko) Decrypted.3ds`
- Size: 1,838,043,136 bytes (1.71 GB)
- Decryption Status: ✓ DECRYPTED
- Format: Decrypted, ready to re-encrypt for test fixture

**Pokemon Alpha Sapphire (Europe) v1.2**
- File: `1324 - Pokemon Alpha Sapphire (Europe) (En,Ja,Fr,De,Es,It,Ko) Decrypted.3ds`
- Size: 1,930,887,168 bytes (1.80 GB)
- Decryption Status: ✓ DECRYPTED
- Format: Decrypted, ready to re-encrypt for test fixture

**Pokemon Omega Ruby (Europe) v1.2**
- File: `1325 - Pokemon Omega Ruby (Europe) (En,Ja,Fr,De,Es,It,Ko) Decrypted.3ds`
- Size: 1,930,768,384 bytes (1.80 GB)
- Decryption Status: ✓ DECRYPTED
- Format: Decrypted, ready to re-encrypt for test fixture

---

## Next Steps (For Rust Implementation)

1. ✅ Read and analyze Python source (DONE)
2. ⬜ Extract ROM headers from test files to identify encryption methods
3. ⬜ Create minimal test fixtures (re-encrypt small portions)
4. ⬜ Implement Rust AES-CTR decryption
5. ⬜ Implement key derivation (ROL function + XOR + addition)
6. ⬜ Write integration tests using test fixtures
7. ⬜ Benchmark Python vs Rust throughput
