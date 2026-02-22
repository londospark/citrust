# Link's Code Analysis: b3DSDecrypt.py → Rust Port

## Executive Summary

b3DSDecrypt.py is a 235-line Python 2 utility for decrypting Nintendo 3DS game ROMs. It:
1. Parses NCSD (Nintendo Card Save Data) container format
2. Iterates 8 partitions, each containing NCCH (Nintendo Content Container Header) sections
3. Derives AES-128 keys from KeyX/KeyY using bitwise rotation and XOR
4. Decrypts ExeFS (executable filesystem), RomFS (read-only filesystem), and extended headers
5. Writes decrypted data back to the same file in-place, toggling crypto flags

---

## Part 1: Code Analysis

### 1.1 Python Function Mapping → Rust Equivalents

#### `rol(val, r_bits, max_bits)` — Rotate-Left Function
**Python (line 7-9):**
```python
rol = lambda val, r_bits, max_bits: \
    (val << r_bits%max_bits) & (2**max_bits-1) | \
    ((val & (2**max_bits-1)) >> (max_bits-(r_bits%max_bits)))
```

**Purpose:** Bitwise rotate-left. Used in key derivation for 128-bit integers.

**Rust Implementation:**
```rust
// For u128: native support, no crate needed
fn rol(val: u128, r_bits: u32, max_bits: u32) -> u128 {
    let r_bits = r_bits % max_bits;
    let mask = (1u128 << max_bits) - 1;
    ((val << r_bits) & mask) | ((val & mask) >> (max_bits - r_bits))
}
```

**Key insight:** Rust's `u128` native type eliminates need for `long()` conversions in Python.

---

#### `to_bytes(num)` — Integer to Big-Endian Bytes
**Python (line 11-17):**
```python
def to_bytes(num):
    numstr = ''
    tmp = num
    while len(numstr) < 16:
        numstr += chr(tmp & 0xFF)
        tmp >>= 8
    return numstr[::-1]
```

**Purpose:** Converts 128-bit integer to 16-byte big-endian array for AES key input.

**Rust Implementation:**
```rust
fn to_bytes(num: u128) -> [u8; 16] {
    num.to_be_bytes()  // Native Rust method
}
```

**Note:** Rust's built-in byte conversion is far simpler and more efficient.

---

### 1.2 Counter/IV Construction

**Python (lines 20-22):**
```python
plain_counter = struct.unpack('>Q', '\x01\x00\x00\x00\x00\x00\x00\x00')
exefs_counter = struct.unpack('>Q', '\x02\x00\x00\x00\x00\x00\x00\x00')
romfs_counter = struct.unpack('>Q', '\x03\x00\x00\x00\x00\x00\x00\x00')
```

These are tuples of 64-bit big-endian integers:
- `plain_counter = (1,)` → 0x0100000000000000 as big-endian
- `exefs_counter = (2,)` → 0x0200000000000000 as big-endian
- `romfs_counter = (3,)` → 0x0300000000000000 as big-endian

**Rust:**
```rust
const PLAIN_COUNTER: u64 = 0x01_00_00_00_00_00_00_00;
const EXEFS_COUNTER: u64 = 0x02_00_00_00_00_00_00_00;
const ROMFS_COUNTER: u64 = 0x03_00_00_00_00_00_00_00;
```

---

#### Hardware Constant
**Python (line 23):**
```python
Constant = struct.unpack('>QQ', '\x1F\xF9\xE9\xAA\xC5\xFE\x04\x08\x02\x45\x91\xDC\x5D\x52\x76\x8A')
```

This is a 128-bit constant: `0x1FF9E9AAC5FE040802459DC5D52768A` (big-endian tuple of two u64s).

**Rust:**
```rust
const HARDWARE_CONSTANT: u128 = 0x1FF9E9AAC5FE040802459DC5D52768A;
```

---

### 1.3 Key Storage

**Python (lines 26-29):** Four KeyX slots (retail keys), each 128-bit:
- KeyX0x2C (< 6.x, oldest)
- KeyX0x25 (> 7.x)
- KeyX0x18 (New 3DS 9.3)
- KeyX0x1B (New 3DS 9.6)

Each unpacked as `struct.unpack('>QQ', bytes)` → tuple of two u64s, later converted to `long` (u128).

**Rust:** Store as `u128` constants or in a lookup table (enum):
```rust
const KEY_X_0x2C: u128 = 0xB98E95CECA3E4D171F76A94DE934C053;
const KEY_X_0x25: u128 = 0xCEE7D8AB30C00DAE850EF5E382AC5AF3;
const KEY_X_0x18: u128 = 0x82E9C9BEBFB8BDB875ECC0A07D474374;
const KEY_X_0x1B: u128 = 0x45AD0495399C7C893372C49A7BCE6182;

enum KeySlot {
    ZeroKey,
    Key0x2C,
    Key0x25,
    Key0x18,
    Key0x1B,
}
```

---

### 1.4 Key Derivation Algorithm

**Python (lines 102-129):**

```python
# Convert tuples of u64s to single u128 via hex string intermediate
KeyX2C = long(str("%016X%016X") % (KeyX0x2C[::]), 16)
KeyY = long(str("%016X%016X") % (part_keyy[::]), 16)
Const = long(str("%016X%016X") % (Constant[::]), 16)

# Derive NormalKey
NormalKey2C = rol((rol(KeyX2C, 2, 128) ^ KeyY) + Const, 87, 128)

# Alternative keys based on partition flags[3]
if partition_flags[3] == 0x00:
    KeyX = KeyX0x2C
elif partition_flags[3] == 0x01:
    KeyX = KeyX0x25
elif partition_flags[3] == 0x0A:
    KeyX = KeyX0x18
elif partition_flags[3] == 0x0B:
    KeyX = KeyX0x1B

NormalKey = rol((rol(KeyX, 2, 128) ^ KeyY) + Const, 87, 128)
```

**Derivation Formula (from NCCH spec):**
```
NormalKey = rol((rol(KeyX, 2, 128) XOR KeyY) + Constant, 87, 128)
```

Where:
- `KeyX` is determined by firmware/partition type
- `KeyY` is first 16 bytes of partition RSA-2048 SHA-256 signature (at offset 0x00 in NCCH)
- `Constant` is the 3DS hardware constant
- Both `rol()` operations are 128-bit rotate-left
- Operations are done on 128-bit integers with automatic wrap-around

**Rust Implementation:**
```rust
fn derive_normal_key(key_x: u128, key_y: u128) -> u128 {
    const HARDWARE_CONSTANT: u128 = 0x1FF9E9AAC5FE040802459DC5D52768A;
    
    let step1 = rol(key_x, 2, 128);
    let step2 = step1 ^ key_y;
    let step3 = step2.wrapping_add(HARDWARE_CONSTANT);
    let key = rol(step3, 87, 128);
    
    key
}
```

**Important:** The `.wrapping_add()` in Rust handles overflow correctly for 128-bit arithmetic.

---

### 1.5 Binary Struct Parsing

| Python Code | Structure | Rust Type | Endian |
|---|---|---|---|
| `struct.unpack('<L', bytes)` | u32 offset/length | `u32` | Little |
| `struct.unpack('<Q', bytes)` | u64 (TitleID) | `u64` | Little |
| `struct.unpack('>Q', bytes)` | u64 (KeyY upper half) | `u64` | Big |
| `struct.unpack('>QQ', bytes)` | 128-bit (Key or Const) | `u128` | Big |
| `struct.unpack('<BBBBBBBB', bytes)` | 8 flag bytes | `[u8; 8]` | N/A |

**Rust parsing approach:**

Option 1: Use `byteorder` crate for manual reads
```rust
use byteorder::{LittleEndian, BigEndian, ReadBytesExt};
use std::io::Cursor;

let mut cursor = Cursor::new(bytes);
let offset: u32 = cursor.read_u32::<LittleEndian>()?;
let title_id: u64 = cursor.read_u64::<LittleEndian>()?;
```

Option 2: Use `binrw` crate for declarative struct parsing (recommended for complex headers)
```rust
#[derive(BinRead)]
#[br(little)]
struct NCCHHeader {
    #[br(seek_before = 0x100)]
    magic: [u8; 4],
    // ... fields
}
```

**For this project, `byteorder` is sufficient and minimal-dependency.**

---

### 1.6 IV Construction for AES-CTR

**Python (lines 70-72, 102-104):**

```python
tid = struct.unpack('<Q', f.read(0x8))  # TitleID at offset 0x108 in NCCH

# Construct 128-bit IVs by tuple concatenation + counter conversion
plain_iv = (tid[::] + plain_counter[::])      # Tuple concat
plainIV = long(str("%016X%016X") % (plain_iv[::]), 16)

exefs_iv = (tid[::] + exefs_counter[::])
exefsIV = long(str("%016X%016X") % (exefs_iv[::]), 16)

romfs_iv = (tid[::] + romfs_counter[::])
romfsIV = long(str("%016X%016X") % (romfs_iv[::]), 16)
```

**Behavior:**
- `tid[::]]` extracts the u64 from tuple → `(TitleID_u64,)`
- `plain_counter[::]]` → `(0x0100000000000000,)` as u64
- Concatenating tuples → `(TitleID_u64, counter_u64)`
- Converting via hex string → 128-bit integer

**Rust equivalent:**
```rust
fn construct_iv(title_id: u64, counter: u64) -> u128 {
    // Combine as (titleid << 64) | counter
    ((title_id as u128) << 64) | (counter as u128)
}

let plain_iv = construct_iv(title_id, PLAIN_COUNTER);
let exefs_iv = construct_iv(title_id, EXEFS_COUNTER);
let romfs_iv = construct_iv(title_id, ROMFS_COUNTER);
```

---

### 1.7 AES-CTR Decryption

**Python (lines 136-147):**

```python
exefsctr2C = Counter.new(128, initial_value=(plainIV))
exefsctrmode2C = AES.new(to_bytes(NormalKey2C), AES.MODE_CTR, counter=exefsctr2C)
g.write(exefsctrmode2C.decrypt(f.read(exhdr_filelen)))
```

**Flow:**
1. Create CTR counter object with 128-bit initial IV
2. Create AES cipher with NormalKey (as 16-byte array), CTR mode, and counter object
3. Read encrypted data, decrypt, write result

**Rust equivalent:**
```rust
use aes::Aes128;
use ctr::Ctr128BE;
use cipher::{KeyIvInit, StreamCipher};

let key = to_bytes(normal_key);
let iv = iv.to_be_bytes();

let mut cipher = Ctr128BE::<Aes128>::new((&key).into(), (&iv).into());
let mut ciphertext = encrypted_data.to_vec();
cipher.apply_keystream(&mut ciphertext);  // In-place decryption
```

**Note on Counter offset (lines 162-164):**
```python
ctroffset = ((code_fileoff[0] + sectorsize) / 0x10)
exefsctr = Counter.new(128, initial_value=(exefsIV + ctroffset))
```

AES-CTR works in 16-byte blocks. The counter is incremented per block. For offset reads in the middle of a file, add block count to IV.

**Rust:**
```rust
let block_offset = (file_offset + sector_size) / 16;
let adjusted_iv = exefs_iv.wrapping_add(block_offset as u128);
```

---

### 1.8 NCSD/NCCH Header Navigation

**NCSD Header (lines 40-46):**
```python
f.seek(0x100)  # NCSD header at offset 0x100
magic = f.read(0x04)
if magic == "NCSD":
    f.seek(0x188)
    ncsd_flags = struct.unpack('<BBBBBBBB', f.read(0x8))
    sectorsize = 0x200 * (2**ncsd_flags[6])
```

**Offsets in NCSD:**
- `0x100`: NCSD magic ("NCSD")
- `0x120` + (partition_index * 0x08): Partition table (8 partitions × 8 bytes each)
  - Bytes 0-3: Partition offset (in sectors)
  - Bytes 4-7: Partition length (in sectors)
- `0x188`: NCSD flags [7 flag bytes]
  - `flags[6]`: Sector size multiplier (2^N * 0x200)

**NCCH Header (at partition offset, offset 0x100 into partition for magic):**
```python
f.seek(((part_off[0]) * sectorsize) + 0x100)  # NCCH magic
magic = f.read(0x04)
if magic == "NCCH":
    f.seek(((part_off[0]) * sectorsize) + 0x0)
    part_keyy = struct.unpack('>QQ', f.read(0x10))  # KeyY at 0x00
    
    f.seek(((part_off[0]) * sectorsize) + 0x108)
    tid = struct.unpack('<Q', f.read(0x08))  # TitleID
    
    f.seek(((part_off[0]) * sectorsize) + 0x188)
    partition_flags = struct.unpack('<BBBBBBBB', f.read(0x8))
```

**Key NCCH offsets:**
| Offset | Content | Type | Endian |
|---|---|---|---|
| 0x00 | KeyY (RSA sig first 16 bytes) | 16 bytes | Big |
| 0x100 | Magic "NCCH" | 4 bytes | ASCII |
| 0x108 | TitleID | u64 | Little |
| 0x160 | ExHeader hash | 32 bytes | Big |
| 0x180 | ExHeader length | u32 | Little |
| 0x188 | Partition flags | 8 bytes | N/A |
| 0x190 | Plain offset | u32 | Little |
| 0x194 | Plain length | u32 | Little |
| 0x198 | Logo offset | u32 | Little |
| 0x19C | Logo length | u32 | Little |
| 0x1A0 | ExeFS offset | u32 | Little |
| 0x1A4 | ExeFS length | u32 | Little |
| 0x1B0 | RomFS offset | u32 | Little |
| 0x1B4 | RomFS length | u32 | Little |
| 0x1C0 | ExeFS hash | 32 bytes | Big |
| 0x1E0 | RomFS hash | 32 bytes | Big |

---

### 1.9 Partition Flags Analysis

**Python (lines 56, 112-128):**

Flags are 8 bytes at partition_offset + 0x188. Key flags:
- `flags[3]`: Encryption key slot
  - 0x00 = KeyX 0x2C (< 6.x)
  - 0x01 = KeyX 0x25 (7.x+)
  - 0x0A = KeyX 0x18 (New 3DS 9.3)
  - 0x0B = KeyX 0x1B (New 3DS 9.6)

- `flags[7]` (bit flags):
  - Bit 0 (0x01) = FixedCryptoKey (zero-key encryption)
  - Bit 2 (0x04) = NoCrypto (unencrypted)
  - Bit 5 (0x20) = CryptoUsingNewKeyY (seed-based derivation, newer games)

**Logic:**
```python
if flags[7] & 0x04:
    # Already decrypted, skip
elif flags[7] & 0x01:
    # Use zero-key (NormalKey = 0)
else:
    # Select KeyX based on flags[3], derive NormalKey
```

**Flag updates after decryption (lines 219-225):**
```python
g.seek((part_off[0] * sectorsize) + 0x18B)
g.write(struct.pack('<B', int(0x00)))  # Clear encryption method at 0x18B

g.seek((part_off[0] * sectorsize) + 0x18F)
flag = int(partition_flags[7])
flag = (flag & ((0x01|0x20)^0xFF))  # Turn off 0x01 and 0x20
flag = (flag | 0x04)                 # Turn on 0x04 (NoCrypto)
g.write(struct.pack('<B', int(flag)))
```

---

### 1.10 Decryption Flow

1. **Open file** (read + write mode)
2. **Seek to NCSD header** (0x100)
3. **For each partition** (0-7):
   a. Read partition offset/length from partition table (0x120 + p*8)
   b. Check partition flags at (offset * sectorsize + 0x188)
   c. If encrypted and partition exists:
      - Read NCCH header to validate magic "NCCH"
      - Extract KeyY, TitleID, dimensions
      - Derive NormalKey from KeyX[slot], KeyY, Constant
      - **Decrypt ExeFS:**
        - Decrypt ExeFS filename table (first sector)
        - Parse filenames to find ".code" binary
        - If found and newer games (flags[3] in 0x01, 0x0A, 0x0B):
          - Decrypt .code section with re-encryption (using NormalKey vs NormalKey2C)
        - Decrypt rest of ExeFS
      - **Decrypt RomFS** (if present)
      - **Update flags** to mark partition as decrypted

---

### 1.11 Python 2 → Rust Translation Issues

| Python 2 | Rust Issue | Solution |
|---|---|---|
| `long()` for large integers | No limit | Use `u128` native type |
| `xrange()` | Doesn't exist | Use `for i in 0..n` or `.range()` |
| `chr()` for byte construction | Byte type differences | Use `u8` and `as u8` casts |
| String literals as bytes | Python 2 strings are bytes | Use `b"string"` syntax or `[u8; 4]` |
| Implicit tuple unpacking `(x,)` | Explicit tuple type | Use explicit types or destructuring |
| `struct.unpack()` returning tuples | Manual field access | Use `byteorder` crate or custom parsing |
| String formatting `"%016X"` | Verbose | Use `format!()` or `u128::to_be_bytes()` |
| In-place file modification | Two file handles | Use `seek()` on single mutable handle in Rust |

---

## Part 2: 3DS ROM Format Research

### 2.1 NCSD (Nintendo Card Secure Data)

**Purpose:** Top-level container for 3DS game ROMs. Can hold up to 8 logical partitions.

**Header Structure:**
```
Offset  Size    Name                Description
0x00    4       Magic               "NCSD"
0x04    4       Size of NCSD        In media units (1 unit = 0x200 bytes)
0x08    8       PartitionFS-type    Cryptographic partition types
0x10    8       PartitionCrypt-type Encryption flags per partition
0x100   8*8     Partition Info      8 partition slots, 8 bytes each
0x160   32      Developer Flags     (Reserved)
0x180   4       Backup Header Offset
0x188   8       Flags               flags[6] = sector size multiplier
0x200   ... Partition data begins ...
```

**Partition Table Entry (8 bytes each):**
```
Offset  Size    Name            Description
+0      4       Partition Offset In sectors from file start
+4      4       Partition Size   In sectors
```

**Sector Size Calculation:**
```
sector_size = 0x200 * (2 ** flags[6])
```
Typically `flags[6] = 3` → sector_size = 0x1000 (4 KB).

**Common Partition Layout:**
- Partition 0: Main game/app (CXI, executable)
- Partition 1: Manual/DLC update (CFA, non-executable)
- Partition 7: Update partition
- Others: Reserved

---

### 2.2 NCCH (Nintendo Content Container Header)

**Purpose:** Individual partition format. Each NCSD partition is an NCCH file. CXI = executable, CFA = data-only.

**Header Overview (simplified):**
```
Offset   Size    Name                Value/Description
0x00     16      RSA-2048 Signature  
0x100    4       Magic               "NCCH"
0x104    4       Content Size        In media units
0x108    8       TitleID             Unique identifier
0x110    4       Maker Code          Developer ID
0x118    2       Format Version      
...
0x160    32      Extended Header Hash (SHA-256)
0x180    4       Extended Header Size In bytes
0x188    8       Flags               Encryption/content flags
0x190    4       Plain Region Offset In sectors
0x194    4       Plain Region Length In sectors
0x198    4       Logo Offset         In sectors
0x19C    4       Logo Length         In sectors
0x1A0    4       ExeFS Offset        In sectors
0x1A4    4       ExeFS Length        In sectors
0x1B0    4       RomFS Offset        In sectors
0x1B4    4       RomFS Length        In sectors
0x1C0    32      ExeFS Hash          (SHA-256)
0x1E0    32      RomFS Hash          (SHA-256)
```

**Key Note:** The RSA-2048 signature's first 16 bytes (0x00-0x0F) are used as **KeyY** in key derivation.

---

### 2.3 Encryption Layers

**Extended Header (ExHeader):**
- Encrypted with "Plain sector IV" (TitleID + 0x01 counter)
- Always present if exheader_len > 0
- Contains executable metadata, ARM9/ARM11 executable sections

**ExeFS (Executable FileSystem):**
- Encrypted with "ExeFS IV" (TitleID + 0x02 counter)
- Contains 10 file slots (filename + offset + size)
- Common files: `.code` (executable), `.rodata` (read-only data), `.data`, `.bss` (heap)
- Filename table is in first sector (always encrypted)
- For games with KeyX 0x25/0x18/0x1B, the `.code` section undergoes "re-encryption":
  - Decrypt with current NormalKey (0x2C)
  - Re-encrypt with new NormalKey (0x25/0x18/0x1B)
  - This is a key migration strategy for firmware updates

**RomFS (Read-Only FileSystem):**
- Encrypted with "RomFS IV" (TitleID + 0x03 counter)
- Contains game data, textures, audio, etc.
- Full file system with directories and file entries

---

### 2.4 Key Derivation Revisited

**Formula (confirmed from specs):**
```
NormalKey = rol((rol(KeyX, 2, 128) XOR KeyY) + Constant, 87, 128)
```

**Hardware Constant:** `0x1FF9E9AAC5FE040802459DC5D52768A`

**KeyX Selection (by firmware/partition):**
- **0x2C** (retail, < 6.x): Oldest games
- **0x25** (retail, > 7.x): Standard games
- **0x18** (New 3DS 9.3+): New 3DS exclusive
- **0x1B** (New 3DS 9.6+): Newest New 3DS games
- **0x01** (fixed/zero-key): Some special partitions

**Seed-Based Derivation (KeyX 0x25+):**
- Newer games may use an additional "seed" combined with KeyY
- Requires SeedDB dump from console
- Not implemented in b3DSDecrypt (hardcoded static keys assumed)

---

### 2.5 Counter Mode (CTR) in AES

**How It Works:**
- IV is a 128-bit starting value
- Counter increments per 16-byte block (AES block size)
- `ciphertext_block[i] = plaintext_block[i] XOR AES_encrypt(IV + i)`

**Counter Construction (128-bit):**
```
IV[upper 64 bits] = TitleID (little-endian)
IV[lower 64 bits] = Counter value (0x01, 0x02, or 0x03) followed by zeros
```

In Python tuple form: `(title_id_u64, counter_u64)`

**Counter Adjustment for Mid-File Offsets:**
- If decrypting from offset N into a file, add `N / 16` (block count) to IV
- Example: If reading `.code` at offset 0x1000 within ExeFS:
  - Base IV = `construct_iv(title_id, exefs_counter)`
  - Adjusted IV = `base_iv + (0x1000 / 16) = base_iv + 256`

---

## Part 3: Rust Implementation Notes

### 3.1 Crate Selection

| Functionality | Crate | Rationale |
|---|---|---|
| AES-128 encryption | `aes` | Lightweight, audited |
| CTR mode | `ctr` | Pairs with `aes`, efficient |
| Byte order conversion | `byteorder` | Minimal, no dependencies |
| 128-bit support | Native `u128` | Built-in, no crate needed |
| Error handling | `anyhow` or `thiserror` | For CLI error reporting |
| File I/O | `std::fs` | Standard library |
| Logging | `log` + `env_logger` | For progress reporting |
| CLI args | `clap` | Standard Rust CLI framework |

**Minimal Dependencies (for core library):**
```toml
[dependencies]
aes = "0.8"
ctr = "0.9"
byteorder = "1.5"

[dev-dependencies]
anyhow = "1.0"
```

### 3.2 Module Structure

**Proposed Rust project layout:**
```
src/
├── lib.rs              # Public library API
├── bin/
│   └── b3dsdecrypt.rs  # CLI binary (main entry point)
├── ncsd.rs             # NCSD header parsing
├── ncch.rs             # NCCH header parsing
├── crypto.rs           # Key derivation, AES-CTR
├── partition.rs        # Partition decryption logic
└── error.rs            # Custom error types
```

---

### 3.3 Error Handling Strategy

**Custom Error Types:**
```rust
#[derive(Debug)]
pub enum DecryptError {
    InvalidNCSD,
    InvalidNCCH(usize), // partition index
    IoError(std::io::Error),
    CryptoError(String),
    FlagMismatch { expected: u8, found: u8 },
}

impl From<std::io::Error> for DecryptError {
    fn from(e: std::io::Error) -> Self {
        DecryptError::IoError(e)
    }
}
```

**Result Type:**
```rust
pub type DecryptResult<T> = Result<T, DecryptError>;
```

---

### 3.4 In-Place File Modification

**Challenge:** Python opens file twice (read + read/write). Rust uses a single mutable handle.

**Solution:**
```rust
use std::fs::OpenOptions;

let mut file = OpenOptions::new()
    .read(true)
    .write(true)
    .open("rom.3ds")?;

// Read from position A
file.seek(SeekFrom::Start(0x100))?;
let magic = file.read_exact(&mut buffer)?;

// Write to position B
file.seek(SeekFrom::Start(0x200))?;
file.write_all(&decrypted_data)?;
```

**Buffer Strategy (for large files):**
- For ExeFS/RomFS (large), use streaming decryption with a buffer ring
- For header/metadata (small), read entire section into memory

---

### 3.5 Integer Arithmetic

**128-bit Operations in Rust:**
```rust
// Addition with wrap-around (standard in crypto)
let result = key_x.wrapping_add(constant);

// XOR (no wrap-around needed)
let result = key_x ^ key_y;

// Rotate-left (custom function)
fn rol(val: u128, r_bits: u32, max_bits: u32) -> u128 {
    let r_bits = r_bits % max_bits;
    let mask = (1u128 << max_bits) - 1;
    ((val << r_bits) & mask) | ((val & mask) >> (max_bits - r_bits))
}
```

**Endianness Conversions:**
```rust
use byteorder::{BigEndian, LittleEndian, ByteOrder};

// Read from bytes
let u32_le = LittleEndian::read_u32(&bytes[0..4]);
let u64_be = BigEndian::read_u64(&bytes[0..8]);

// Write to bytes
let mut buf = [0u8; 4];
LittleEndian::write_u32(&mut buf, value);
```

---

### 3.6 struct Parsing Example

**Approach 1: Manual with byteorder**
```rust
struct NCCHFlags {
    crypto_method: u8,
    _reserved: [u8; 2],
    key_slot: u8,
    _reserved2: [u8; 3],
    crypto_flags: u8,
}

fn read_ncch_flags(data: &[u8]) -> NCCHFlags {
    NCCHFlags {
        crypto_method: data[0],
        reserved: [data[1], data[2]],
        key_slot: data[3],
        reserved2: [data[4], data[5], data[6]],
        crypto_flags: data[7],
    }
}
```

**Approach 2: Bit-level for flags**
```rust
#[derive(Clone, Copy)]
pub struct CryptoFlags(u8);

impl CryptoFlags {
    pub fn is_zero_key(self) -> bool { self.0 & 0x01 != 0 }
    pub fn is_no_crypto(self) -> bool { self.0 & 0x04 != 0 }
    pub fn uses_new_key_y(self) -> bool { self.0 & 0x20 != 0 }
    
    pub fn set_no_crypto(&mut self) { self.0 |= 0x04; }
    pub fn clear_zero_key(&mut self) { self.0 &= !0x01; }
    pub fn clear_new_key_y(&mut self) { self.0 &= !0x20; }
}
```

---

### 3.7 Progress Reporting

**Python uses:** `print()` with `%` formatting and `\r` for in-place updates.

**Rust equivalent:**
```rust
use std::io::{self, Write};

// In-place progress
print!("\rPartition {}: Decrypting... {:.1}%", p, percent);
io::stdout().flush()?;

// Or use logging
log::info!("Partition {}: Decrypting ExeFS... {}/{} MB", p, current, total);
```

---

### 3.8 Key Implementation Workflow

1. **Parse NCSD header** → Get sector size, partition table
2. **For each partition:**
   - Parse NCCH header (KeyY, TitleID, dimensions)
   - Check flags → Determine key slot
   - Derive NormalKey
   - **Decrypt ExeFS filename table** → Identify sections
   - **Decrypt ExeFS data** (including .code re-encryption if needed)
   - **Decrypt RomFS data** (streaming for large files)
   - **Update partition flags** (clear crypto bits, set NoCrypto)
3. **Close file**

---

## Summary of Key Findings

1. **Language-specific pain points:**
   - Python's `long()` → Rust's native `u128` (simpler!)
   - `xrange()` → Rust ranges
   - Byte handling and struct unpacking → `byteorder` crate
   - String/bytes confusion → Clear with Rust's type system

2. **Crypto primitives:**
   - AES-128-CTR is industry standard → `aes` + `ctr` crates
   - Key derivation is pure bitwise ops → Easy to port
   - Counter IV construction is straightforward

3. **File format complexity:**
   - 3 nested levels: NCSD → NCCH → ExeFS/RomFS
   - Multiple encryption keys per partition based on firmware
   - Special handling for `.code` re-encryption in newer firmware
   - Offset/size pairs require multiplication by sector size

4. **Performance considerations:**
   - Large files (1+ GB) require streaming, not full buffering
   - Rust's zero-copy semantics help here
   - Counter offsets are critical for mid-file decryption

5. **Testing strategy:**
   - Unit tests for `rol()`, `derive_normal_key()`, IV construction
   - Integration tests with small test ROMs (if available)
   - Flag parsing validation
   - Endianness verification with known test vectors

---

## Next Steps for Implementation

1. Set up Rust project with dependencies (`Cargo.toml`)
2. Implement `crypto.rs` (key derivation, rol, IV construction)
3. Implement `ncsd.rs` and `ncch.rs` (header parsing)
4. Implement `partition.rs` (decryption orchestration)
5. Write binary `b3dsdecrypt.rs` with CLI interface
6. Integration testing with actual ROM (after validation)
7. Expose library API for Fox's GUI integration

---

**Analysis completed:** Ready for implementation phase.
