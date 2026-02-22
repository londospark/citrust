# Decision: GitHub Actions CI & Release Pipeline (Issue #21)

**By:** Samus (Lead)  
**Date:** 2026-02-22  
**Status:** Implemented  
**Category:** Infrastructure

## Summary

Implemented complete GitHub Actions CI and release automation for citrust:

### CI Workflow (`.github/workflows/ci.yml`)

Triggered on every push to `master` and pull requests to `master`.

**Jobs:**
1. **check** — `cargo check` (early validation)
2. **test** — `cargo test` (unit tests)
3. **clippy** — `cargo clippy -- -D warnings` (warnings as errors)
4. **fmt** — `cargo fmt --check` (code formatting validation)

All run on `ubuntu-latest` with:
- `dtolnay/rust-toolchain@stable` — latest stable Rust
- `Swatinem/rust-cache@v2` — dependency caching for speed
- `actions/checkout@v4` — repository checkout

### Release Workflow (`.github/workflows/release.yml`)

Triggered on push of version tags matching `v*` (e.g., `v0.1.0`, `v1.0.0`).

**Jobs:**
1. **build-linux** — Builds release binary for `x86_64-unknown-linux-gnu`
   - Runs on `ubuntu-latest`
   - Outputs: `target/x86_64-unknown-linux-gnu/release/citrust`
   - Artifact name: `citrust-linux-x86_64`

2. **build-windows** — Builds release binary for `x86_64-pc-windows-msvc`
   - Runs on `windows-latest` (for native MSVC toolchain)
   - Outputs: `target\x86_64-pc-windows-msvc\release\citrust.exe`
   - Artifact name: `citrust-windows-x86_64.exe`

3. **release** — Creates GitHub Release with attached binaries
   - Depends on: `build-linux`, `build-windows` (parallel execution until both complete)
   - Downloads both artifacts to `artifacts/` directory
   - Uses `softprops/action-gh-release@v2` to create release
   - Attaches both binaries
   - Auto-generates release notes from commit history

## Technical Decisions

1. **Use latest stable Rust:** Provides best security updates and features without version pinning complexity
2. **dtolnay/rust-toolchain:** Lightweight, well-maintained action; simplifies setup vs. official action
3. **Separate Linux/Windows builds:** Ensures native toolchain and binary format correctness
4. **Windows build on windows-latest:** Uses native MSVC compiler (not cross-compile), avoiding complexity
5. **Artifact naming:** Follows `citrust-{platform}-{arch}` pattern for clarity in releases
6. **Swatinem/rust-cache:** Speeds up CI by caching dependencies between runs
7. **Hard-coded paths vs. wildcards:** Windows requires backslash paths; Linux uses forward slashes; no abstraction layer needed

## Scope Boundaries

- **Own:** `.github/workflows/` directory (ci.yml, release.yml)
- **Do NOT modify:** `Cargo.toml`, `src/`, or any source files

## Next Steps

1. Push workflows to repository
2. Create first version tag (`v0.1.0`) to test release workflow
3. Verify both Linux and Windows binaries are attached to release
4. Update README with CI badge (optional, post-Phase 1)
