# Link — History

## Project Context
- **Project:** citrust — Rust port of b3DSDecrypt.py (3DS ROM decryption tool) with SteamOS GUI
- **Stack:** Rust, AES-CTR crypto, binary parsing, CLI
- **User:** Gareth
- **Source file:** b3DS-master/b3DSDecrypt.py (235 lines, Python 2)

## Learnings

### Analysis Phase (Current)
- **Code Architecture:** b3DSDecrypt.py is a single-pass NCSD/NCCH parser. Decryption happens in-place via AES-CTR. Key derivation is pure math (rol + XOR + add).
- **Critical Algorithm:** Key derivation `NormalKey = rol((rol(KeyX, 2, 128) ^ KeyY) + Constant, 87, 128)` is the heart of decryption. Constant is hardcoded 3DS hardware value. KeyX selected by partition flags[3].
- **Encryption Structure:** 3 levels of IV construction: ExHeader (plain counter 0x01), ExeFS (0x02), RomFS (0x03) combined with TitleID. Counter offsets are critical for mid-file reads.
- **Python 2 Pain:** `long()` for u128, `xrange()` for ranges, ambiguous bytes/string handling. Rust's u128 native type + ranges eliminate much boilerplate.
- **Binary Parsing:** Extensive struct unpacking with mixed endianness (big-endian keys, little-endian offsets). `byteorder` crate handles cleanly.
- **File I/O:** In-place modification using dual handles (Python) → Single mutable handle (Rust). Streaming decryption needed for large files (1+ GB).
- **Key Selection:** 4 retail KeyX slots (0x2C, 0x25, 0x18, 0x1B) based on firmware era. Zero-key (0x01 flag) for special partitions. Re-encryption of .code section for certain firmware transitions.
- **Crate Selection:** `aes` + `ctr` for crypto, `byteorder` for parsing, native `u128` for math. Minimal dependency footprint.
- **Testing Focus:** Unit tests for rol/key derivation, IV construction, flag parsing. Integration tests need small ROM or synthetic test data.

### Phase 2: SIMD/AES-NI Optimization (2026-02-22)
- **AES-NI Investigation:** The `aes` 0.8 crate already uses `cpufeatures` for runtime CPU detection, but needs explicit `target-cpu=native` for optimal codegen. Created `.cargo/config.toml` with `rustflags = ["-C", "target-cpu=native"]` to enable all native CPU features (AES-NI, AVX, SSE4.2).
- **Chunk Size Profiling:** Benchmarked RomFS chunks from 2MB to 64MB. Found 2-8MB range performs identically (~1.48s), but 4MB has lowest variance (σ=0.04s). Original 16MB was 32% slower; 32MB+ degrades significantly (L2/L3 cache pressure). Selected 4MB for consistency.
- **Combined Speedup:** AES-NI (1.19x) + chunk tuning gave 1.50x total speedup (2.34s → 1.56s on Pokemon Y 1.75GB).
- **Verification:** All tests validated with SHA256 hash to ensure byte-identical output. Zero functional changes.
- **Performance Insight:** Smaller chunks (4MB vs 16MB) improve cache locality for AES-CTR operations. Beyond 8MB, memory bandwidth becomes bottleneck. ExeFS 1MB chunks already optimal for smaller sections.
