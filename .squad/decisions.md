# Decisions

> Team decisions log. Append-only. Managed by Scribe.

---

## 2026-02-22: Workspace Conversion (Issue #17)

**By:** Link (Core Dev)  
**Date:** 2026-02-22  
**Status:** Implemented  
**Category:** Architecture

Converted citrust from a single crate to a Cargo workspace with 3 member crates:

- **citrust-core** — library crate (crypto, parsing, decryption logic)
- **citrust-cli** — binary crate producing `citrust` CLI executable
- **citrust-gui** — binary crate producing `citrust-gui` GUI executable

**Structure:**
```
crates/
├── citrust-core/   # lib: aes, ctr, cipher, rayon, memmap2, thiserror
│   ├── src/        # lib.rs, keys.rs, crypto.rs, ncsd.rs, ncch.rs, decrypt.rs
│   ├── benches/    # criterion benchmarks
│   └── tests/      # integration tests
├── citrust-cli/    # bin: depends on citrust-core, clap, indicatif
│   └── src/main.rs
└── citrust-gui/    # bin: depends on citrust-core, eframe, rfd
    └── src/main.rs
```

**Key Decisions:**
1. Benchmarks and integration tests live in citrust-core (criterion requires `[[bench]]` in crate Cargo.toml)
2. Import path is `citrust_core::` (underscore, per Rust convention for hyphenated crate names)
3. CI uses `--workspace` flags for check/test/clippy; release uses `-p citrust-cli` for targeted builds
4. Resolver = "3" (edition 2024 default)

**Verification:**
- All 19 unit tests pass
- 5 integration tests compile (ignored, require ROM files)
- GUI binary compiles successfully
- CLI binary compiles successfully

---

## 2026-02-22: Distribution Strategy for citrust

**Lead:** Samus  
**Status:** Recommended  
**Category:** Infrastructure  
**Complexity:** Extensive analysis (360+ lines)

**Executive Summary:**

citrust is a Rust-based 3DS ROM decryption tool with CLI + GUI (gamepad-optimized, 1280x800). To maximize adoption, prioritize **Flatpak (Flathub) for SteamOS/Steam Deck** and **AppImage for portability**, with **winget for Windows** as secondary. These three channels cover the largest audience with manageable effort.

**Recommended Channels (Priority Ranked):**

| Channel | Rating | Effort | Priority | Notes |
|---------|--------|--------|----------|-------|
| **Flatpak** | 🟢 Recommended | Medium | #1 | SteamOS/Steam Deck native; Discover integration. Start here. |
| **AppImage** | 🟢 Recommended | Low–Med | #2 | Portable, zero-install; works everywhere. Do immediately after Flatpak. |
| **winget** | 🟢 Recommended | Low | #2b | Windows users; minimal effort. Do in parallel or after AppImage. |
| **AUR** | 🟡 Nice-to-have | Low | #3 | Arch enthusiasts; read-only rootfs issue limits appeal. Defer. |
| **Scoop** | 🟡 Nice-to-have | Very Low | #4 | Windows devs; minimal effort. After winget. |
| **Homebrew** | 🟡 Nice-to-have | Low–Med | #5 | Niche macOS audience. Defer unless explicit demand. |
| **Snap** | 🔴 Skip | Medium | — | SteamOS doesn't use Snap; inferior to Flatpak. Skip. |
| **Steam Store** | 🔴 Skip | N/A | — | Not feasible for indie project. Use AppImage + Steam non-Steam game. |

**Key Insights:**
1. Flatpak is table-stakes for SteamOS/Steam Deck adoption
2. AppImage essential for portability and zero-install UX
3. Gamepad support works seamlessly once sandboxing permissions are correct
4. SteamOS read-only rootfs eliminates AUR as primary channel
5. Windows gamers served via winget with minimal effort

**Implementation Timeline:**
- **Phase 1 (Week 1–2):** Flatpak manifest, AppStream metadata, icon, desktop file
- **Phase 2 (Week 2–3):** AppImage CI/CD + winget manifests (parallel)
- **Phase 3 (Week 4+):** AUR + Scoop (if demand)

**Full analysis in original decision document — covers channel requirements, SteamOS considerations, resource links, and comprehensive next-steps checklist.**

---

## 2026-02-22: Integration Test Architecture

**By:** Toad (Tester)  
**Date:** 2026-02-22  
**Status:** Implemented  
**Category:** Testing

Integration tests live in `tests/integration_tests.rs` (standard Rust integration test location), separate from inline unit tests in `src/`. They use `#[ignore]` to skip by default since they require ~1.7 GB ROM files.

**Key Choices:**
1. Temp files in `test-fixtures/` — avoids adding `tempfile` crate; directory is gitignored so temp files are safe
2. SHA256 verification — `sha2` added to `[dev-dependencies]` only; used to verify byte-identical output against known hashes
3. No-op test — verifies that decrypt → decrypt produces identical output (NoCrypto flag works correctly)
4. Dual purpose — these tests also serve as Phase 2 regression tests; same hashes confirm AES-NI optimizations preserve correctness

**Running:**
```bash
cargo test                # 19 unit tests (fast, no ROM files needed)
cargo test -- --ignored   # 5 integration tests (requires ROM files in Test Files/)
```

**Impact:** All agents: if you change crypto, decryption, or header parsing code, integration tests will catch regressions. Run `cargo test -- --ignored` before merging crypto changes.

---

## 2026-02-22: Copilot Directive (Implementation Phases)

**By:** Gareth (via Copilot)  
**Date:** 2026-02-22T01:28:48Z  
**Status:** Directive  
**Category:** Planning

Implementation phases: (1) Get it working simply, (2) Use SIMD, (3) Use multiple threads. All phases must be benchmarked against the Python implementation. Test using files in `Test Files\` — decrypt with Python first, then verify Rust produces identical output.

---

## 2026-02-22: GUI Framework — egui/eframe Recommended

**By:** Fox (GUI Dev)  
**Date:** 2026-02-22  
**Status:** Recommendation  
**Category:** Architecture

After evaluating 5 major Rust GUI frameworks (egui, iced, slint, gtk4, libcosmic), **egui (via eframe)** recommended for citrust SteamOS GUI:

**Why egui:**
- **Simplicity:** Immediate-mode for simple 3-step flow (pick → decrypt → done)
- **Lightweight:** ~2MB binary, minimal dependencies
- **Linux/Wayland:** First-class support via eframe
- **Controller Input:** Trivial via `gilrs` library; immediate mode simplifies button mapping
- **File Picker:** Use `rfd` for native file choosers
- **Gamepad UX:** Large buttons, keyboard-driven navigation

**Recommended Tech Stack:**
- egui + eframe (GUI framework)
- gilrs (gamepad input)
- rfd (native file picker)
- citrust core library (via crate)
- Target: Linux/SteamOS, X11/Wayland
- Binary size: ~2–4MB (release)

**Recommendation confidence:** 9/10 — egui is the clear winner for this specific use case. Immediate-mode paradigm, minimal dependencies, and trivial controller integration make it ideal.

**Next Steps:**
1. Share with Samus (architect) for approval
2. Build proof-of-concept: file picker + gamepad nav + progress bar
3. Once prototype validates UX, begin full implementation

---

## 2026-02-22: Crypto Crate Selection

**By:** Link (Core Dev)  
**Date:** 2024 (Analysis phase), approved 2026-02-22  
**Status:** Approved  
**Category:** Architecture

Use **`aes` + `ctr` crates** for AES-128-CTR decryption:

```toml
aes = "0.8"        # NIST standardized AES, well-audited
ctr = "0.9"        # CTR mode wrapper
byteorder = "1.5"  # Struct parsing
```

**Rationale:**
- ✅ RustCrypto ecosystem (highly trusted)
- ✅ Integrates seamlessly via `cipher` trait
- ✅ Minimal code surface (2 core crates)
- ✅ Hardware AES-NI support when available
- ✅ No alternative: openssl adds complexity, ring is overkill

**Implementation notes:**
- Native Rust `u128` type replaces Python's `long()`
- Standard `cipher::StreamCipher` trait for encryption/decryption
- No homegrown crypto implementations

---

## 2026-02-22: GitHub Project Management Structure

**By:** Samus (Lead)  
**Date:** 2026-02-22  
**Status:** Implemented  
**Category:** Infrastructure

The citrust repo (londospark/citrust) has full GitHub project management infrastructure:

**Structure:**
- **Labels:** Three-axis taxonomy: `phase:*` (4 phases), `module:*` (7 modules), `type:*` (4 types), `squad:*` (agent assignment)
- **Issues:** 21 total — one per module per phase, plus cross-cutting test/benchmark/infra issues
- **Milestones:** Four, matching the four phases. Issue #21 (CI) spans all phases (unassigned)
- **Project Board:** Single "citrust Roadmap" (project #6) with all 21 issues, using default columns (Todo/In Progress/Done)

**Agent Assignment:**
- **Link** (Core Dev): #1–#6 (Phase 1), #10–#11 (Phase 2 SIMD), #13–#15 (Phase 3 threading)
- **Toad** (Tester): #7–#9 (Phase 1 tests/benchmarks), #12 (Phase 2 regression), #16 (Phase 3 regression)
- **Fox** (GUI Dev): #18–#19 (Phase 4 GUI + gamepad)
- **Samus** (Lead): #17 (workspace split), #20 (AppImage), #21 (CI)

**Issue Reference:**
| # | Title | Phase | Owner |
|---|-------|-------|-------|
| 1 | keys.rs — key constants and derivation | 1 | Link |
| 2 | crypto.rs — ROL128 + AES-CTR wrapper | 1 | Link |
| 3 | ncsd.rs — NCSD header parser | 1 | Link |
| 4 | ncch.rs — NCCH partition parser | 1 | Link |
| 5 | decrypt.rs — decryption orchestrator | 1 | Link |
| 6 | main.rs — CLI entry point | 1 | Link |
| 7 | Unit tests for all Phase 1 modules | 1 | Toad |
| 8 | Integration tests — round-trip decryption | 1 | Toad |
| 9 | Benchmark Phase 1 vs Python | 1 | Toad |
| 10 | Enable AES-NI via .cargo/config.toml | 2 | Link |
| 11 | Profile and tune AES-CTR chunk sizes | 2 | Link |
| 12 | Regression tests + benchmark Phase 1 vs Phase 2 | 2 | Toad |
| 13 | Refactor decrypt.rs for parallel I/O | 3 | Link |
| 14 | Chunk-level RomFS parallelism with rayon | 3 | Link |
| 15 | Section-level parallelism | 3 | Link |
| 16 | Regression tests + benchmark Phase 2 vs Phase 3 | 3 | Toad |
| 17 | Convert to workspace (core/cli/gui crates) | 4 | Samus |
| 18 | egui/eframe GUI — file picker, progress, done | 4 | Fox |
| 19 | Gamepad input support with gilrs | 4 | Fox |
| 20 | AppImage packaging for SteamOS | 4 | Samus |
| 21 | GitHub Actions CI workflow | — | Samus |

---

## 2026-02-22: Port Architecture

**By:** Samus (Lead)  
**Date:** 2026-02-22  
**Status:** Proposed  
**Category:** Architecture

Full project architecture for citrust Rust port:

**Phases 1–3: Single Crate**
- `lib.rs` (public API) + `main.rs` (CLI binary)
- Modules: `keys.rs`, `crypto.rs`, `ncsd.rs`, `ncch.rs`, `decrypt.rs`
- Rationale: Reduces friction during critical "get it working" phase

**Phase 4+: Workspace Conversion**
- `citrust-core`, `citrust-cli`, `citrust-gui` crates
- Mechanical conversion triggered when GUI work begins

**Crypto: RustCrypto (`aes` + `ctr` + `cipher`)**
- Pure Rust, no C dependencies (important for musl static builds)
- Automatic AES-NI via `target-feature=+aes` — no code changes for SIMD
- Well-maintained, audited, widely used
- Rejected: `ring` (doesn't expose raw AES-CTR easily), `openssl` (C dependency)

**Key Arithmetic: Native `u128`**
- `u128::rotate_left()` maps directly to Python's `rol()`
- `u128::wrapping_add()` handles overflow correctly
- No external bignum crate needed

**Test Strategy: Round-Trip via Python Encrypter**
1. Use `b3DSEncrypt.py` to encrypt copies of decrypted test ROMs → encrypted fixtures
2. Run Rust decrypter on encrypted copies
3. Binary-diff against original decrypted ROMs — must be identical
4. Test fixtures are gitignored (1.7–1.8 GB each); generated by script

**Benchmarking: criterion + Shell Timing**
- Rust: `criterion` for micro and macro benchmarks
- Python: `Measure-Command` (PowerShell) / `time` (Linux)
- Results tracked in `benches/results/` as JSON

**SteamOS Packaging**
- CLI: Static musl binary — maximum portability, zero dependencies
- GUI (future): AppImage — portable, no root, desktop integration
- Future: Flatpak for SteamOS Discover store integration
- GUI framework: egui/eframe (pure Rust, gamepad-friendly, GPU-rendered)

**Progress Callback API**
- `decrypt_rom()` accepts `impl FnMut(ProgressEvent)` callback
- Events: `PartitionStart`, `SectionProgress`, `SectionDone`, etc.
- CLI uses `indicatif` progress bars; GUI will use egui widgets

**Encryption Support in Phase 1**
- Implement alongside decryption
- Validates crypto correctness (encrypt → decrypt → original)
- Needed for test fixture creation (can replace Python encrypter)
- Shares 95% of code with decryption

---

## 2026-02-22: Test Strategy — Fixture Generation & Encryption Path Coverage

**By:** Toad (Tester)  
**Date:** 2026-02-22  
**Status:** Recommended  
**Category:** Testing

**Problem:** The three available test ROMs are all **decrypted**. Python script requires encrypted ROMs as input.

**Decision: Hybrid Approach**

**Option A (Primary): Re-encrypt Pokemon Y**
- Reverse the decryption: use known key derivation to re-encrypt sections
- Create a ~100 MB encrypted test ROM
- Verify Rust decryption recovers original plaintext
- Reusable for all decryption tests

**Option B (Supplementary): Create Synthetic Minimal Fixtures**
- Tiny 1–10 MB ROMs with known headers
- Test each encryption method variant (KeyX 0x2C, 0x25, 0x18, 0x1B)
- Test edge cases: zero-key, empty regions, corrupted headers
- Faster CI/CD execution

**Rationale:**
- Re-encryption is necessary: can't test decryption without encrypted input
- Synthetic fixtures are cheaper: easier to test edge cases without 1.7 GB files
- Hybrid gives coverage: real ROM for integration tests, synthetic for unit tests and edge cases
- Original plaintext still available for verification

**Action Items:**
1. Implement re-encryption utility (mirror of Python logic)
2. Create ~100 MB encrypted Pokemon Y test fixture
3. Store synthetic fixture templates (10 MB each, one per encryption method)
4. Document exact plaintext/ciphertext pairs

---

## 2026-02-22: Unit Test Architecture Decision

**By:** Toad (Tester)  
**Date:** 2026-02-22  
**Status:** Implemented  
**Category:** Testing

Unit tests for Phase 1 modules (keys, crypto, ncsd, ncch) are implemented as inline `#[cfg(test)]` modules within each source file, not in a separate `tests/` directory.

**Rationale:**
- **Locality:** Tests are adjacent to the code they test, making them easier to maintain
- **Access:** Can test private functions and internal implementation details
- **Convention:** Standard Rust practice for unit tests (integration tests go in `tests/`)

**Test Statistics:**
- **Total tests:** 19 (all passing)
- **keys.rs:** 3 tests (method mapping, constants validation, flag conversion)
- **crypto.rs:** 8 tests (rol128 edge cases, key derivation, AES-CTR, conversions)
- **ncsd.rs:** 4 tests (header parsing, magic validation, sector size, partition helpers)
- **ncch.rs:** 4 tests (header parsing, crypto detection, flags, IV construction)

**Benchmark Infrastructure:**
- **criterion benchmarks** (benches/crypto_bench.rs):
  - rol128 throughput (small/large shifts)
  - derive_normal_key throughput
  - aes_ctr_decrypt throughput (1MB and 16MB buffers)
  - Full key derivation pipeline (KeyX → NormalKey)
- **PowerShell comparison script** (benches/compare.ps1):
  - Takes ROM path as argument
  - Copies ROM twice (Rust/Python)
  - Times both decrypters
  - SHA256-verifies outputs match
  - Reports speedup ratio
  - Cleans up temporary files

---

## 2026-02-22: SIMD/AES-NI Optimization Results (Issues #10 & #11)

**By:** Link (Core Dev)  
**Date:** 2026-02-22  
**Status:** Implemented  
**Category:** Performance

**Test Environment:**
- **Test ROM:** Pokemon Y (1.75GB encrypted)
- **CPU:** AMD Ryzen (native AES-NI support)
- **Configuration:** Windows, Release build

**Issue #10: AES-NI Hardware Acceleration**
- The `aes` 0.8 crate already uses runtime CPU feature detection via `cpufeatures` dependency
- Created `.cargo/config.toml` to enable all native CPU features including AES-NI, AVX, SSE4.2
- **Result:** 1.19x speedup (2.34s → 1.97s) with zero code changes

**Issue #11: AES-CTR Chunk Size Tuning**
- Benchmarked RomFS chunks from 2MB to 64MB
- **Key findings:** 2–8MB range performs identically (~1.48s); 4MB selected for consistency (σ=0.04s)
- **Original 16MB:** 32% slower (1.96s vs 1.48s)
- **Large chunks:** 32MB+ significantly degrade (cache pressure)
- **Change:** decrypt.rs line 279: `16 * 1024 * 1024` → `4 * 1024 * 1024`
- **ExeFS:** 1MB chunks unchanged (already optimal)

**Combined Results:**
| Phase | Configuration | Time | Improvement |
|-------|--------------|------|-------------|
| Original | 16MB chunks, no target-cpu | 2.34s | baseline |
| + AES-NI | 16MB chunks, target-cpu=native | 1.97s | 1.19x |
| + Chunk tuning | 4MB chunks, target-cpu=native | 1.56s | 1.50x |

**Final speedup:** 1.50x over baseline (2.34s → 1.56s)

**Verification:** All configurations tested 3–5 iterations; every run SHA256-verified for byte-identical output.

---

## 2026-02-22: GUI Implementation Complete (Issues #18 & #19)

**By:** Fox (GUI Dev)  
**Date:** 2026-02-22  
**Status:** Implemented  
**Category:** Implementation

Completed full egui/eframe GUI implementation for citrust with gamepad-friendly design targeting SteamOS.

**Architecture:**
- Optional `gui` feature with `eframe` and `rfd` dependencies
- Separate binary `citrust-gui` (9.2 MB) — CLI unaffected (1.6 MB)
- Background threading for decryption with `mpsc::channel` progress updates
- Three-screen workflow: File Selection → Decrypting → Done

**UI Design (Gamepad-Friendly):**
- Dark theme matching SteamOS aesthetic
- Large fonts: 48px heading, 28px buttons, 24px body text
- Large hit targets: 400x80px buttons (minimum)
- 1280x800 default window (Steam Deck native resolution)
- Full keyboard navigation (Tab/Enter) for gamepad mapping via Steam Input

**Screen Flow:**
1. **Select File:** Title, "Select ROM File" button → rfd file dialog, shows selected path
2. **Decrypting:** File name, encryption method, real-time progress log, section tracker, elapsed time
3. **Done:** Success message, "Decrypt Another" and "Quit" buttons

**Files Modified:**
- **Created:** `src/gui.rs` (270+ LOC)
- **Modified:** `Cargo.toml` (added `[features]`, optional deps, `[[bin]]` section)

**Testing Status:**
- ✅ CLI build unaffected (1.6 MB)
- ✅ GUI builds with `--features gui`
- ✅ Both binaries compile successfully
- ⚠️ Manual UX testing pending (requires real ROM + hardware/SteamOS)

---

## 2026-02-22: GitHub Actions CI & Release Pipeline (Issue #21)

**By:** Samus (Lead)  
**Date:** 2026-02-22  
**Status:** Implemented  
**Category:** Infrastructure

Implemented complete GitHub Actions CI and release automation for citrust.

**CI Workflow (`.github/workflows/ci.yml`):**
- Triggered on every push to `master` and pull requests to `master`
- Jobs: **check** (cargo check), **test** (cargo test), **clippy** (cargo clippy -- -D warnings), **fmt** (cargo fmt --check)
- Infrastructure: ubuntu-latest, dtolnay/rust-toolchain@stable, Swatinem/rust-cache@v2

**Release Workflow (`.github/workflows/release.yml`):**
- Triggered on version tags (v*)
- **build-linux:** x86_64-unknown-linux-gnu on ubuntu-latest
- **build-windows:** x86_64-pc-windows-msvc on windows-latest
- **release:** Creates GitHub Release with both binaries, auto-generated release notes

**Technical Decisions:**
- Use latest stable Rust for security updates
- dtolnay/rust-toolchain for lightweight setup
- Separate Linux/Windows builds ensure native toolchains
- Windows build on windows-latest (native MSVC, no cross-compile)
- Artifact naming: `citrust-{platform}-{arch}` for clarity
- Swatinem cache speeds up CI via dependency caching

**Scope:** Owns `.github/workflows/` directory only; does not modify Cargo.toml or source files.

---
