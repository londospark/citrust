# Samus — History

## Project Context
- **Project:** citrust — Rust port of b3DSDecrypt.py (3DS ROM decryption tool) with SteamOS GUI
- **Stack:** Rust, AES-CTR crypto, binary parsing, GUI framework, SteamOS packaging
- **User:** Gareth
- **Source:** b3DS-master/b3DSDecrypt.py — Python 2 script that decrypts 3DS NCSD/NCCH ROMs using AES-128-CTR

## Learnings

### 2026-02-22: GitHub Actions CI & Release Pipeline (Issue #21)
- **CI workflow** (`.github/workflows/ci.yml`): 4 jobs on push/PR to master
  - `check`: cargo check (fast fail)
  - `test`: cargo test (unit tests)
  - `clippy`: cargo clippy with deny-warnings
  - `fmt`: cargo fmt --check
  - All use dtolnay/rust-toolchain@stable, Swatinem/rust-cache@v2, ubuntu-latest
- **Release workflow** (`.github/workflows/release.yml`): Triggered on version tags (v*)
  - `build-linux`: x86_64-unknown-linux-gnu, uploads citrust binary
  - `build-windows`: x86_64-pc-windows-msvc on windows-latest, uploads citrust.exe
  - `release`: Creates GitHub Release with both artifacts, auto-generated release notes via softprops/action-gh-release@v2
  - Proper artifact naming, Windows path handling (backslashes in paths), dependency chaining
- **Key decisions**: Use latest stable Rust, leverage dtolnay toolchain for simplicity, Swatinem cache for speed, actions/v4 versions for stability
- **Scope**: No Cargo.toml or src/ changes; workflows fully own .github/workflows/

### 2026-02-22: Port Plan Created
- **Architecture**: Single crate (lib + bin) for Phases 1–3; workspace split deferred to Phase 4 (GUI)
- **Modules**: `keys.rs`, `crypto.rs`, `ncsd.rs`, `ncch.rs`, `decrypt.rs`, `main.rs`
- **Crypto**: RustCrypto (`aes` + `ctr` + `cipher`). Native `u128` for 128-bit key math. AES-NI via target-feature.
- **Key insight**: 3DS key scrambler is `ROL128((ROL128(KeyX, 2) ^ KeyY) + Constant, 87)` — maps directly to `u128::rotate_left()` + `wrapping_add()`
- **Test strategy**: Round-trip via Python encrypter (b3DSEncrypt.py → Rust decrypt → diff against decrypted originals)
- **Test files**: 3 Pokémon ROMs in `Test Files\` (~1.7–1.8 GB each, already decrypted)
- **Python caveat**: b3DS scripts are Python 2 + pycrypto (EOL) — may need Python 3 port or Docker
- **SteamOS target**: Static musl binary (CLI), AppImage (GUI), egui/eframe for GUI framework
- **Decisions written**: `.squad/decisions/inbox/samus-port-architecture.md` (8 decisions)
- **Port plan**: `.squad/agents/samus/port-plan.md`
- **Gareth preference**: Three phases benchmarked against Python — (1) simple, (2) SIMD, (3) multi-threaded
- **Encrypt support**: Include in Phase 1 — validates crypto, replaces Python encrypter for test fixtures
- **Double-layer crypto**: ExeFS `.code` section uses two keys (NormalKey + NormalKey2C) for 7.x/9.x crypto — most subtle part of the port

### 2026-02-22: GitHub Project Infrastructure Setup
- **Labels created** (20 total): 4 phase labels, 7 module labels, 4 type labels, 5 squad labels
- **Issues created** (#1–#21): Phase 1 (#1–#9), Phase 2 (#10–#12), Phase 3 (#13–#16), Phase 4 (#17–#20), CI (#21)
- **Milestones created** (4): Phase 1 (milestone 1, issues #1–#9), Phase 2 (milestone 2, #10–#12), Phase 3 (milestone 3, #13–#16), Phase 4 (milestone 4, #17–#20)
- **Project board**: "citrust Roadmap" (project #6) — all 21 issues added
- **Issue assignments**: Link (core dev) owns #1–#6, #10–#11, #13–#15; Toad (tester) owns #7–#9, #12, #16; Fox (GUI) owns #18–#19; Samus (lead) owns #17, #20, #21
- **Dependencies tracked in issue bodies**: #5 depends on #1–#4; #6 depends on #5; #7–#9 depend on #1–#6; Phase 2/3/4 issues chain correctly
- **Key note**: Test ROMs are ENCRYPTED (NoCrypto=False) despite "Decrypted" filenames — documented in test issues #7, #8

### 2026-02-22: GitHub Actions CI & Release Automation (Issue #21)
- **CI workflow** (`.github/workflows/ci.yml`): 4 parallel jobs (check/test/clippy/fmt) on push/PR to master
  - Infrastructure: ubuntu-latest, dtolnay/rust-toolchain@stable, Swatinem/rust-cache@v2
- **Release workflow** (`.github/workflows/release.yml`): Triggered on version tags (v*)
  - build-linux: x86_64-unknown-linux-gnu on ubuntu-latest
  - build-windows: x86_64-pc-windows-msvc on windows-latest
  - release: Creates GitHub Release with both binaries, auto-generated notes via softprops/action-gh-release@v2
- **Key decisions**: Latest stable Rust, dtolnay toolchain for simplicity, separate platform builds for native toolchains, Swatinem cache for speed, artifact naming pattern (citrust-{platform}-{arch})
- **Scope**: Owns .github/workflows/ only; no Cargo.toml or src/ changes
- **Next**: First version tag (v0.1.0) to validate release automation

### 2026-02-22: Distribution Strategy Analysis Complete
- **Recommended channels:** Flatpak (Priority #1, SteamOS native), AppImage (Priority #2, portable), winget (Priority #2b, Windows)
- **Nice-to-have:** AUR (#3, limited by read-only rootfs), Scoop (#4, Windows devs), Homebrew (#5, niche)
- **Skip:** Snap (SteamOS doesn't use), Steam Store (unfeasible for indie project)
- **Key deliverables:** Comprehensive 360+ line decision document with requirements, effort estimates, SteamOS considerations, resource links, 3-phase implementation roadmap
- **Timeline:** Phase 1 (Week 1–2) Flatpak, Phase 2 (Week 2–3) AppImage + winget, Phase 3 (Week 4+) AUR/Scoop
- **Documentation:** Full analysis merged into `.squad/decisions.md` (lines 8–90 approx). Orchestration log written. Decisions inbox cleared.

### 2026-02-22: Team Batch Orchestration Complete
- **Agents deployed:** Link (sync, workspace conversion ✅), Fox (background, GUI implementation ✅), Samus (background, distribution strategy ✅)
- **Deliverables:** Orchestration logs written for each agent. Session log created. All inbox decisions merged into decisions.md. Inbox directory cleared. Agent history.md files updated with cross-team context. Git commit pending.

### 2026-02-22: AppImage Build in Release Workflow (Issue #20)
- **Release workflow updated** (`.github/workflows/release.yml`): Added AppImage build job
- **New job `build-appimage`:** Builds citrust-gui, uses linuxdeploy to package as AppImage
  - Downloads linuxdeploy-x86_64.AppImage from continuous release
  - Uses `packaging/citrust-gui.desktop` and `packaging/citrust.png` for desktop integration
  - Outputs `citrust*.AppImage` as artifact `citrust-gui-appimage`
- **`build-linux` updated:** Now builds both `citrust-cli` AND `citrust-gui` bare binaries
  - Uploads separate artifacts: `citrust-linux-x86_64` (CLI) and `citrust-gui-linux-x86_64` (GUI)
  - GUI deps installed via `$LINUX_GUI_DEPS` env var (same pattern as CI workflow)
- **`release` job updated:** `needs: [build-linux, build-windows, build-appimage]`, downloads all 4 artifacts
- **Packaging files:** `packaging/citrust-gui.desktop` (pre-existing), `packaging/citrust.png` (placeholder icon created)
- **Key pattern:** `env.LINUX_GUI_DEPS` keeps GUI dependency list DRY across CI and release workflows
- **Release artifacts (total 4):** CLI binary (Linux), GUI binary (Linux), CLI .exe (Windows), GUI AppImage (Linux)

### 2026-07-16: v0.2.0 Release Cut (Issue #20 Closed)
- **Issue #20 closed:** AppImage packaging delivered via PR #23, commented with deliverables and follow-up items
- **Version bumped:** All 3 crates (citrust-core, citrust-cli, citrust-gui) from 0.1.0 → 0.2.0
- **Metainfo updated:** Added v0.2.0 release entry to `packaging/io.github.londospark.citrust.metainfo.xml`
- **Git flow executed:** develop → master (--no-ff merge), tagged v0.2.0, master → develop sync
- **Tag pushed:** `v0.2.0` triggers release workflow building CLI (Linux/Windows), GUI (Linux), AppImage
- **Follow-ups:** Real icon needed (placeholder functional), SteamOS hardware testing pending
- **Release includes:** AppImage packaging, content-based decryption detection, dependency updates, CI fixes
