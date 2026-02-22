# citrust — Port Plan

> Python 2 → Rust port of b3DSDecrypt.py (3DS ROM decryption tool)
> Author: Samus (Lead) | Date: 2026-02-22

---

## 1. Source Analysis

### What b3DSDecrypt.py Does

The Python script decrypts Nintendo 3DS ROM files (`.3ds`) in-place. The file format is:

```
NCSD Container (header at 0x100)
├── Partition Table (0x120): 8 slots × 8 bytes (offset + length in sectors)
├── NCSD Flags (0x188): sector size = 0x200 * 2^flags[6]
└── Partitions 0–7 (NCCH format each)
    ├── RSA-2048 signature (first 16 bytes = KeyY)
    ├── NCCH magic at +0x100
    ├── Title ID at +0x108 (used as IV base)
    ├── Partition flags at +0x188 (crypto method + NoCrypto bit)
    ├── Section offsets: ExHeader, Plain, Logo, ExeFS, RomFS
    └── Section hashes (SHA-256) for ExHeader, ExeFS, RomFS
```

### Crypto Pipeline Per Partition

1. **Key Derivation** — 128-bit rotate-XOR-add scrambler:
   ```
   NormalKey = ROL128((ROL128(KeyX, 2) ^ KeyY) + Constant, 87)
   ```
   - `KeyY` = first 16 bytes of partition signature
   - `KeyX` = selected by crypto method (flags[3]): 0x2C, 0x25, 0x18, or 0x1B
   - `NormalKey2C` = always derived from KeyX0x2C (base key for ExHeader/ExeFS table)
   - `Constant` = 3DS hardware constant [REDACTED]

2. **Decryption Sections** (all AES-128-CTR):
   | Section | Key | IV |
   |---------|-----|----|
   | ExHeader (0x800 bytes) | NormalKey2C | TitleID ∥ 0x0100000000000000 |
   | ExeFS filename table (1 sector) | NormalKey2C | TitleID ∥ 0x0200000000000000 |
   | ExeFS .code (7.x/9.x only) | Double-layer: decrypt(NormalKey) then encrypt(NormalKey2C) | ExeFS IV + counter offset |
   | ExeFS data (rest) | NormalKey2C | ExeFS IV + sector offset |
   | RomFS (bulk data) | NormalKey | TitleID ∥ 0x0300000000000000 |

3. **Flag Patching**: After decryption, set NoCrypto bit (0x04), clear FixedCryptoKey (0x01) and CryptoUsingNewKeyY (0x20), zero crypto-method byte.

### Key Implementation Details

- **In-place modification**: File opened for both read and write simultaneously
- **Chunked processing**: RomFS processed in 16 MB blocks, ExeFS in 1 MB blocks
- **Zero-key mode**: If FixedCryptoKey flag set, all keys = 0
- **128-bit arithmetic**: Python uses arbitrary-precision `long`; Rust has native `u128`
- **Partition iteration**: All 8 partition slots checked; empty ones skipped

### Encrypt Script (b3DSEncrypt.py)

The encrypter is the inverse — takes decrypted ROMs and re-encrypts them. This is **critical for testing**: we'll use it to create encrypted test fixtures from our known-good decrypted ROMs.

Key difference from decrypt: encrypt reads `backup_flags` at 0x1188 to recover the original crypto method. Also, for partitions 1+, RomFS always uses Key0x2C.

---

## 2. Architecture

### Phase 1 Structure (Single Crate)

```
citrust/
├── Cargo.toml              # single crate: lib + [[bin]]
├── src/
│   ├── lib.rs              # public API re-exports
│   ├── keys.rs             # key constants (retail + dev), key derivation (ROL128 scrambler)
│   ├── crypto.rs           # AES-128-CTR wrapper, 128-bit rotate/XOR/add
│   ├── ncsd.rs             # NCSD header + partition table parsing
│   ├── ncch.rs             # NCCH partition header parsing (flags, offsets, IVs)
│   ├── decrypt.rs          # decryption orchestrator (per-partition, per-section)
│   └── main.rs             # CLI entry point
├── benches/
│   └── decrypt_bench.rs    # criterion benchmarks
└── tests/
    └── round_trip.rs       # integration tests using test ROMs
```

### Later Phases (Workspace)

When adding GUI (Phase 4+), convert to workspace:

```
citrust/
├── Cargo.toml              # [workspace]
├── crates/
│   ├── citrust-core/       # lib crate (crypto, parsing, decryption)
│   ├── citrust-cli/        # CLI binary
│   └── citrust-gui/        # SteamOS GUI binary (egui/eframe)
├── benches/
└── tests/
```

### Module Responsibilities

| Module | Responsibility | Key Types |
|--------|---------------|-----------|
| `keys.rs` | Key constants, `KeySlot` enum, `derive_normal_key()` | `KeySlot`, `Key128` (type alias for `[u8; 16]`) |
| `crypto.rs` | `rol128()`, `scramble_key()`, `AesCtr128` wrapper | — |
| `ncsd.rs` | Parse NCSD header, partition table, flags | `NcsdHeader`, `PartitionEntry` |
| `ncch.rs` | Parse NCCH header, section offsets, crypto flags | `NcchHeader`, `CryptoMethod`, `SectionInfo` |
| `decrypt.rs` | Orchestrate decryption: iterate partitions, decrypt sections, patch flags | `decrypt_rom()`, `DecryptProgress` callback |
| `main.rs` | CLI: parse args, call `decrypt_rom()`, print progress | — |

### Error Handling

Use `thiserror` for the library error type:

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a 3DS ROM (missing NCSD magic at 0x100)")]
    NotNcsd,
    #[error("partition {0}: invalid NCCH header")]
    InvalidNcch(u8),
    #[error("partition {0}: already decrypted")]
    AlreadyDecrypted(u8),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

### Progress Reporting

Library exposes a callback-based progress API so both CLI and GUI can consume it:

```rust
pub enum ProgressEvent {
    PartitionStart { index: u8, crypto_method: CryptoMethod },
    SectionStart { name: String, size_bytes: u64 },
    SectionProgress { bytes_done: u64, bytes_total: u64 },
    SectionDone,
    PartitionDone { index: u8 },
    AlreadyDecrypted { index: u8 },
    PartitionEmpty { index: u8 },
}

pub fn decrypt_rom(
    path: &Path,
    on_progress: impl FnMut(ProgressEvent),
) -> Result<(), Error>;
```

---

## 3. Dependencies

### Runtime
| Crate | Purpose | Version |
|-------|---------|---------|
| `aes` | AES-128 block cipher (RustCrypto) | latest |
| `ctr` | CTR mode stream cipher | latest |
| `cipher` | StreamCipher + KeyIvInit traits | latest |
| `clap` | CLI argument parsing (derive) | 4.x |
| `thiserror` | Ergonomic error types | 2.x |
| `indicatif` | Terminal progress bars | latest |

### Dev / Bench
| Crate | Purpose |
|-------|---------|
| `criterion` | Benchmarking framework |
| `sha2` | Verify section hashes match (test assertions) |
| `tempfile` | Temp copies for integration tests |

### NOT Needed
- `byteorder`: Rust std has `u32::from_le_bytes()` etc.
- `num` / `num-bigint`: Rust has native `u128` for 128-bit key math

---

## 4. Phase 1 — Simple Working Implementation

**Goal**: Byte-for-byte identical output to Python decrypter. No optimizations.

### Tasks (ordered)

1. **`keys.rs`** — Define key constants as `[u8; 16]` arrays. Implement `KeySlot` enum (X2C, X25, X18, X1B). Both retail and dev key sets.

2. **`crypto.rs`** — Implement:
   - `rol128(val: u128, bits: u32) -> u128` — 128-bit rotate left
   - `scramble_key(key_x: u128, key_y: u128, constant: u128) -> u128` — the 3DS key derivation
   - `u128_to_bytes(val: u128) -> [u8; 16]` and `bytes_to_u128(bytes: &[u8; 16]) -> u128`
   - Thin wrapper around `aes`/`ctr` for AES-128-CTR encrypt/decrypt

3. **`ncsd.rs`** — Parse:
   - Magic validation at 0x100
   - NCSD flags at 0x188 → sector size
   - Partition table at 0x120 → 8 × (offset, length)

4. **`ncch.rs`** — Parse per-partition:
   - NCCH magic at partition_offset + 0x100
   - KeyY from signature (first 16 bytes)
   - Title ID at +0x108
   - Partition flags at +0x188 (crypto method, NoCrypto, FixedCrypto, NewKeyY)
   - Section offsets/lengths: ExHeader, Plain, Logo, ExeFS, RomFS
   - Section hashes

5. **`decrypt.rs`** — Orchestrate:
   - Open file read + write (two handles or `File` with seek)
   - Iterate 8 partitions
   - Per partition: derive keys, compute IVs, decrypt sections in order
   - Handle .code double-layer decryption for 7.x/9.x keys
   - Patch flags after decryption
   - Chunked reads (1 MB for ExeFS, 16 MB for RomFS) matching Python

6. **`main.rs`** — CLI:
   - `clap` with positional arg for ROM path
   - Call `decrypt_rom()` with progress callback → `indicatif` progress bar
   - Exit codes: 0 = success, 1 = error

### Acceptance Criteria
- `cargo run -- "encrypted_copy.3ds"` produces byte-identical output to Python decrypter
- All 3 test ROMs pass round-trip test (encrypt with Python → decrypt with Rust → diff = 0)

---

## 5. Phase 2 — SIMD Optimization

**Goal**: Use hardware AES-NI instructions for faster AES-CTR. Measure improvement.

### Approach

The `aes` crate from RustCrypto **already uses AES-NI** when compiled with `target-feature=+aes`. Phase 2 is mostly about:

1. **Ensuring AES-NI is enabled** in release builds:
   ```toml
   # .cargo/config.toml
   [target.'cfg(target_arch = "x86_64")']
   rustflags = ["-C", "target-feature=+aes,+ssse3"]
   ```

2. **Parallel CTR block generation**: The `ctr` crate can process multiple blocks at once. Tune buffer sizes to maximize throughput (process 8 or 16 AES blocks per iteration instead of streaming byte-by-byte).

3. **Benchmark gating**: Must show measurable improvement over Phase 1 baseline on the test ROMs.

4. **Optional**: If `aes`/`ctr` don't saturate AES-NI throughput, consider using `aesni` crate directly or writing inline SIMD with `std::arch`.

### Tasks

1. Add `.cargo/config.toml` with AES-NI + SSSE3 flags for x86_64
2. Profile Phase 1 to find hotspots (should be AES-CTR)
3. Tune chunk sizes for CTR mode — align to AES block boundaries (multiples of 16 bytes, ideally 4096+ bytes per call)
4. Benchmark: Phase 1 vs Phase 2 on all 3 test ROMs
5. Verify byte-identical output (regression test)

### Expected Gains
AES-NI processes ~1 byte/cycle. At 3 GHz, that's ~3 GB/s theoretical. Python with pycrypto (C extension) is maybe 100-200 MB/s. We should see **10-30× improvement** over Python just from Phase 1 with AES-NI enabled, so Phase 2 is about *ensuring* we're getting that and not leaving performance on the table.

---

## 6. Phase 3 — Multi-threaded Processing

**Goal**: Decrypt multiple partitions and/or sections in parallel. Measure improvement.

### Approach

3DS ROMs have up to 8 partitions, and each partition has independent sections (ExHeader, ExeFS, RomFS). These can be decrypted independently since:
- Each has its own key and IV
- They occupy non-overlapping byte ranges in the file

### Strategy

**Option A — Partition-level parallelism** (simpler):
- Use `rayon` or `std::thread::scope` to process partitions in parallel
- Each thread gets its own read handle + write handle (or uses `pwrite`/`pread`)
- Most ROMs only have 1-2 non-empty partitions, so limited speedup

**Option B — Section-level parallelism** (better):
- Within a single partition, decrypt ExHeader, ExeFS, and RomFS in parallel
- RomFS is the biggest section (often 1+ GB) — split it into chunks across threads

**Option C — Chunk-level parallelism for RomFS** (best):
- AES-CTR is seekable — we can compute the keystream at any offset
- Split RomFS into N chunks (one per core)
- Each thread decrypts its chunk independently
- This is the big win since RomFS dominates runtime

### Recommended: Option B + C combined

```
Main thread:
├── Parse NCSD + NCCH headers (sequential, tiny)
├── For each partition:
│   ├── Derive keys (sequential, instant)
│   └── Spawn parallel tasks:
│       ├── Thread 1: ExHeader (0x800 bytes — trivial)
│       ├── Thread 2: ExeFS table + .code + ExeFS data
│       └── Threads 3–N: RomFS chunks (split by core count)
└── Patch flags (sequential, trivial)
```

### Dependencies
| Crate | Purpose |
|-------|---------|
| `rayon` | Data-parallel iterators for chunk processing |

### Tasks

1. Refactor `decrypt.rs` to accept byte ranges instead of sequential file I/O
2. Use `std::fs::File` with `pread`/`pwrite` (or `seek` + `read`/`write` with per-thread file handles)
3. Split RomFS into chunks, compute per-chunk CTR offset
4. Use `rayon::scope` or `std::thread::scope` for parallel decryption
5. Benchmark: Phase 2 vs Phase 3 on all 3 test ROMs
6. Verify byte-identical output (regression test)

### Expected Gains
- On a 4-core machine: ~2-3× over Phase 2 (RomFS dominates, so chunk parallelism helps)
- On 8+ cores: diminishing returns (I/O becomes bottleneck for spinning disks; NVMe should scale further)
- SteamOS Deck has 4 cores / 8 threads — expect ~3× over single-threaded

---

## 7. Testing Strategy

### Test Data

Available test ROMs (all **already decrypted**):

| File | Size |
|------|------|
| Pokémon Y (Europe) Decrypted.3ds | 1,753 MB |
| Pokémon Alpha Sapphire (Europe) Decrypted.3ds | 1,841 MB |
| Pokémon Omega Ruby (Europe) Decrypted.3ds | 1,841 MB |

### Creating Encrypted Test Fixtures

Since we have the decrypted ROMs AND the Python encrypter (`b3DSEncrypt.py`), the process is:

1. Copy a decrypted ROM → `test-fixtures/pokemon-y-encrypted.3ds`
2. Run `python2 b3DSEncrypt.py "test-fixtures/pokemon-y-encrypted.3ds"` (encrypts in-place)
3. Now we have an encrypted file whose decrypted form is the original

⚠️ **Git**: Test fixtures are large (1.7–1.8 GB each). Add `test-fixtures/` to `.gitignore`. Create a script `scripts/create-test-fixtures.sh` to generate them.

### Test Levels

#### Unit Tests (in-module `#[cfg(test)]`)
- `keys.rs`: Verify key constants match Python hex values
- `crypto.rs`: Test `rol128` with known values, test `scramble_key` with known KeyX/KeyY/Constant → known NormalKey
- `ncsd.rs`: Parse a minimal synthetic NCSD header (hand-crafted 512-byte fixture)
- `ncch.rs`: Parse a minimal synthetic NCCH header

#### Integration Tests (`tests/round_trip.rs`)
- **Round-trip test** (requires test fixtures + Python):
  1. Copy decrypted ROM to temp file
  2. Encrypt with Python (`b3DSEncrypt.py`)
  3. Decrypt with Rust (`citrust`)
  4. Binary compare with original decrypted ROM
  5. Run for all 3 test ROMs
- Mark as `#[ignore]` — run manually with `cargo test -- --ignored`

#### Snapshot Tests
- Read header fields from decrypted ROMs with Rust parser
- Assert Title IDs, partition counts, section offsets match expected values
- These validate the parser without needing encryption/decryption

#### Regression Tests (across phases)
- After Phase 2 and 3, re-run all integration tests
- Output must remain byte-identical to Phase 1

### Test Script

Create `scripts/test-full.ps1`:
```powershell
# 1. Create test fixtures (encrypt decrypted ROMs with Python)
# 2. Run unit tests: cargo test
# 3. Run integration tests: cargo test -- --ignored
# 4. Clean up temp files
```

---

## 8. Benchmarking Strategy

### Framework

Use `criterion` for Rust benchmarks. For Python baseline, use PowerShell `Measure-Command`.

### Benchmarks

| Benchmark | What It Measures |
|-----------|-----------------|
| `bench_key_derivation` | Time to derive one NormalKey (crypto.rs) |
| `bench_aes_ctr_1mb` | AES-CTR decrypt 1 MB block (crypto.rs) |
| `bench_decrypt_full_rom` | End-to-end: decrypt entire ROM file |
| `bench_python_baseline` | Python decrypter on same ROM (shell benchmark) |

### Benchmark Script

Create `scripts/benchmark.ps1`:
```powershell
# 1. Create encrypted copies of test ROMs
# 2. Run Python decrypter, record time
# 3. Re-encrypt (restore encrypted copies)
# 4. Run Rust (Phase N) decrypter, record time
# 5. Print comparison table
```

### Tracking Results

Store benchmark results in `benches/results/`:
```
benches/results/
├── phase1-results.json
├── phase2-results.json
└── phase3-results.json
```

### Expected Performance Trajectory

| Phase | Expected Speed | vs Python |
|-------|---------------|-----------|
| Python baseline | ~100-200 MB/s | 1× |
| Phase 1 (simple Rust) | ~500-1000 MB/s | 5-10× |
| Phase 2 (SIMD/AES-NI) | ~2000-3000 MB/s | 15-30× |
| Phase 3 (multi-threaded) | ~5000-8000 MB/s | 30-60× |

(Estimates for NVMe storage. Spinning disk will be I/O-bound at ~150 MB/s.)

---

## 9. Build & Publish Strategy (SteamOS)

### Target Platform
- **SteamOS 3.x** = Arch Linux, x86_64, KDE Plasma
- Steam Deck: AMD Zen 2, 4C/8T, 16 GB RAM
- Gaming Mode: Big Picture-style UI, gamepad input

### Binary Targets

1. **CLI binary** (`citrust`): statically linked with musl for maximum portability
2. **GUI binary** (`citrust-gui`): dynamically linked (needs GPU/display libs)

### Build Configuration

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-musl]
rustflags = ["-C", "target-feature=+aes,+ssse3"]

[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "target-feature=+aes,+ssse3"]
```

### Packaging Options

| Format | Pros | Cons | Recommendation |
|--------|------|------|----------------|
| Static binary | Simplest, no deps, runs anywhere | No desktop integration | ✅ CLI: Primary |
| AppImage | Portable, no root, desktop integration | Larger size, bundling complexity | ✅ GUI: Primary |
| Flatpak | SteamOS Discover store, sandboxed | Complex manifest, permissions | 🔜 GUI: Future (v1.1+) |
| AUR package | Arch/SteamOS native | Requires AUR account | 🔜 Future |

### Phase 1 Deliverable
- `cargo build --release` → single CLI binary
- Cross-compile with `cross` for Linux x86_64 if developing on Windows

### GUI Framework (Future)
- **egui/eframe**: Pure Rust, immediate mode, minimal dependencies, works with gamepad
- Renders via `wgpu` or `glow` — works on Steam Deck GPU
- Can run fullscreen for Gaming Mode compatibility

### CI/CD (GitHub Actions)
```yaml
# .github/workflows/ci.yml
- Build + test on ubuntu-latest
- Build musl static binary
- Build AppImage (GUI phase)
- Benchmark on each PR (criterion compare)
- Release: tag → build → GitHub Release with binaries
```

---

## 10. Implementation Order

```
Phase 1: Simple Working Implementation
├── 1.1 keys.rs — constants + derivation
├── 1.2 crypto.rs — ROL128 + AES-CTR wrapper
├── 1.3 ncsd.rs — NCSD parser
├── 1.4 ncch.rs — NCCH parser
├── 1.5 decrypt.rs — decryption orchestrator
├── 1.6 main.rs — CLI
├── 1.7 Unit tests for all modules
├── 1.8 Create test fixtures (Python encrypt)
├── 1.9 Integration tests (round-trip)
└── 1.10 Benchmark vs Python

Phase 2: SIMD Optimization
├── 2.1 .cargo/config.toml with AES-NI flags
├── 2.2 Profile and tune chunk sizes
├── 2.3 Regression tests (byte-identical)
└── 2.4 Benchmark Phase 1 vs Phase 2

Phase 3: Multi-threaded
├── 3.1 Refactor decrypt.rs for parallel I/O
├── 3.2 Chunk-level RomFS parallelism
├── 3.3 Section-level parallelism
├── 3.4 Regression tests (byte-identical)
└── 3.5 Benchmark Phase 2 vs Phase 3

Phase 4: GUI + SteamOS (future)
├── 4.1 Convert to workspace
├── 4.2 egui/eframe GUI
├── 4.3 AppImage packaging
├── 4.4 Gamepad input support
└── 4.5 Flatpak manifest
```

---

## 11. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| 128-bit arithmetic edge cases (overflow, rotate) | Wrong keys → garbage output | Extensive unit tests with known-good Python outputs |
| AES-CTR counter semantics differ between Rust and Python | Wrong decryption | Test with single-block encryption, verify keystream matches |
| Python 2 `pycrypto` hard to install for test fixture generation | Can't create test data | Use Docker image with Python 2 + pycrypto, or port encrypter to Python 3 + pycryptodome |
| Test ROMs too large for CI | CI can't run integration tests | Run integration tests locally only; CI runs unit tests + synthetic fixtures |
| In-place file modification concurrent read/write | Data corruption | Use separate read + write file handles with explicit seeking, or memory-map |
| `.code` double-layer decrypt (7.x/9.x) is subtle | Silent data corruption | Dedicated unit test with known .code section bytes |

---

## 12. Open Questions

1. **Encrypt support?** — The Python source includes `b3DSEncrypt.py`. Should Rust also support encryption? (Useful for testing, and for completeness.) **Recommendation: Yes, add in Phase 1 as it's the inverse operation and validates our crypto.**

2. **Dev keys?** — Python has commented-out dev key constants. Include them behind a feature flag? **Recommendation: Yes, `--dev-keys` CLI flag or `dev-keys` cargo feature.**

3. **Python 2 availability** — `pycrypto` requires Python 2. For test fixture creation, should we port the encrypter to Python 3 + `pycryptodome` first? **Recommendation: Yes, quick port before starting Rust work. Or just use Docker.**

4. **Memory-mapped I/O?** — For Phase 3, `mmap` could simplify parallel access. Worth exploring? **Recommendation: Consider for Phase 3, but start with file handles + seeking in Phase 1.**
