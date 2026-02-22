# Decision: Workspace Conversion (Issue #17)

**By:** Link (Core Dev)  
**Date:** 2026-02-22  
**Status:** Implemented  
**Category:** Architecture

## Summary

Converted citrust from a single crate to a Cargo workspace with 3 member crates:

- **citrust-core** — library crate (crypto, parsing, decryption logic)
- **citrust-cli** — binary crate producing `citrust` CLI executable
- **citrust-gui** — binary crate producing `citrust-gui` GUI executable

## Structure

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

## Key Decisions

1. **Benchmarks and integration tests live in citrust-core**, not workspace root — criterion requires `[[bench]]` in a specific crate's Cargo.toml.
2. **Import path is `citrust_core::`** (underscore, per Rust convention for hyphenated crate names).
3. **CI uses `--workspace` flags** for check/test/clippy; release uses `-p citrust-cli` for targeted builds.
4. **Resolver = "3"** (edition 2024 default).

## Verification

- All 19 unit tests pass
- 5 integration tests compile (ignored, require ROM files)
- GUI binary compiles successfully
- CLI binary compiles successfully
