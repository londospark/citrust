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

### Phase 2 Completion: 2026-02-22

Achieved 1.50x final speedup (2.34s → 1.56s baseline) via combined AES-NI + chunk tuning. Team batch completed: Toad's 19 tests + benchmarks, Fox's full egui GUI (9.2 MB), Samus's CI/release automation. Phase 3 multi-threading with rayon estimated 2–4x additional speedup. All configurations verified byte-identical via SHA256.

### Phase 4: Workspace Conversion (Issue #17)
- **Workspace Structure:** Converted single crate to 3-crate Cargo workspace: `citrust-core` (lib), `citrust-cli` (bin), `citrust-gui` (bin) under `crates/` directory.
- **Import Migration:** All `citrust::` references changed to `citrust_core::` in CLI, GUI, integration tests, and benchmarks. Core library internal `crate::` references unchanged.
- **Benches & Tests Location:** Moved `benches/` and `tests/` into `crates/citrust-core/` since criterion benchmarks and integration tests need to live inside a specific crate, not at workspace root.
- **CI Updated:** `ci.yml` uses `--workspace` flag for check/test/clippy and `--all` for fmt. `release.yml` uses `-p citrust-cli` for targeted binary builds.
- **Key Paths:** Root `Cargo.toml` = workspace manifest only. Core lib at `crates/citrust-core/`. CLI at `crates/citrust-cli/`. GUI at `crates/citrust-gui/`.
- **All 19 unit tests pass.** Integration tests (5, ignored) and GUI binary also compile successfully.

### 2026-02-22: Team Batch Completion
- **Orchestration complete.** Fox built full egui/eframe GUI (9.2 MB, 1280x800 Steam Deck native, gamepad-friendly dark theme). Samus completed comprehensive distribution strategy analysis (Flatpak #1, AppImage #2, winget #2b, with detailed roadmap). All decisions merged into `.squad/decisions.md`. Orchestration logs written. Session log created.

### Inazuma ROM Investigation (2026-07)
- **Root Cause Found: Double-Encryption Bug.** The Inazuma ROM (`Test Files\1706...Decrypted.3ds`) has NoCrypto flag=0 (claims encrypted) but its content is ALREADY plaintext (ExHeader reads "InazumaG", ExeFS has valid `.code`/`banner`/`icon`/`logo` sections). This is a mis-flagged ROM dump — likely decrypted during dump (GodMode9 etc.) without updating the header flag.
- **Effect:** Both Python (b3DSDecrypt.py) and Rust (citrust) check only the NoCrypto flag bit (`flags[7] & 0x04`). Since it's 0, they "decrypt" the already-decrypted content, which is AES-CTR XOR — effectively RE-ENCRYPTING it. They then set NoCrypto=true. Result: flag says "decrypted" but content is encrypted garbage.
- **Azahar Error Explained:** Azahar trusts the NoCrypto flag, skips decryption, tries to parse ExHeader → gets encrypted garbage → fails with generic Error (code 1). The `16384` in the log is simply the partition 0 offset (32 MU × 0x200 = 0x4000 = 16384).
- **All 4 active partitions affected:** P0 (3326 MB main), P1 (4.6 MB), P6 (8.8 MB), P7 (37.9 MB) — all had NoCrypto=false in the original but plaintext content.
- **Both tools produce identical wrong output** because they make the same mistake — SHA256 match confirms they're identically wrong, not identically correct.
- **Headers are structurally valid:** NCSD magic ✓, all NCCH magics ✓, partition table offsets/sizes all within file bounds, ExHeader size=1024, flags[3]=0x00 (standard crypto method). No field equals 16384 unexpectedly.
- **Potential Fix:** Add content validation before decrypting — check if ExHeader starts with valid ASCII (codeset name) or ExeFS has readable section names. If content is already plaintext, skip crypto and just set the flag. This would handle mis-flagged ROM dumps gracefully.

### Content-Based Decryption Detection (2026-07)
- **Implemented `is_content_decrypted()`** — public function in `decrypt.rs` that checks if a partition's content is already plaintext despite `NoCrypto` flag not being set.
- **Primary heuristic:** Read first 8 bytes of ExeFS region (filename table entry). If all bytes are valid ASCII (0x20–0x7E) or null (0x00), content is already decrypted. Encrypted data produces random bytes that fail this check with near certainty (probability of false positive: ~(127/256)^8 ≈ 0.4%).
- **Fallback:** If no ExeFS exists, checks first 8 bytes of ExHeader (codeset name field) using the same ASCII heuristic.
- **Integration:** Added to `decrypt_rom()` flow — after `is_no_crypto()` check, before key derivation. On detection, skips all AES-CTR operations and only patches flags (same as normal post-decryption flag patching).
- **Solves:** Mis-flagged ROM dumps (e.g., Inazuma Eleven GO) that were decrypted during dump but didn't get NoCrypto flag set. Previously these would be double-encrypted into garbage.

### Reverse Detection: Encrypted Despite NoCrypto Flag (2026-07)
- **Problem:** ROMs flagged as decrypted (NoCrypto=True) but actually containing encrypted content. The previous code blindly trusted the NoCrypto flag and skipped decryption.
- **Fix:** When `is_no_crypto()` is true, now calls `is_content_decrypted()` to verify. If content is plaintext → skip with "Already Decrypted ✓". If content is encrypted → clear NoCrypto bit, recover backup crypto_method from NCSD offset 0x1188+(p*8)+3 if available, re-parse NCCH, and fall through to normal decryption.
- **Backup flags:** NCSD stores backup partition flags at 0x1188. If the backup has a valid non-zero crypto_method, it's written into the NCCH header before re-parsing. Otherwise defaults to CryptoMethod::Original (0x00).
- **Implementation:** Minimal change — replaced the 4-line `is_no_crypto()` check block with a `let ncch = if ... else` that either continues (skip) or rebinds `ncch` with corrected flags and falls through. No changes to any other decryption logic.
- **Test:** `test_decrypt_detects_encrypted_despite_nocrypto_flag` — synthetic ROM with NoCrypto=True + FixedKey + random ExeFS bytes. Verifies warning message is logged, content is modified (decryption applied), and NoCrypto flag is set post-decryption. All 25 tests pass.
