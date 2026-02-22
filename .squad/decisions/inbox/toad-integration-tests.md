# Decision: Integration Test Architecture

**By:** Toad (Tester)
**Date:** 2026-02-22
**Status:** Implemented
**Category:** Testing

## Decision

Integration tests live in `tests/integration_tests.rs` (standard Rust integration test location), separate from inline unit tests in `src/`. They use `#[ignore]` to skip by default since they require ~1.7 GB ROM files.

## Key Choices

1. **Temp files in `test-fixtures/`** — avoids adding `tempfile` crate; directory is gitignored so temp files are safe
2. **SHA256 verification** — `sha2` added to `[dev-dependencies]` only; used to verify byte-identical output against known hashes
3. **No-op test** — verifies that decrypt → decrypt produces identical output (NoCrypto flag works correctly)
4. **Dual purpose** — these tests also serve as Phase 2 regression tests; same hashes confirm AES-NI optimizations preserve correctness

## Running

```bash
cargo test                # 19 unit tests (fast, no ROM files needed)
cargo test -- --ignored   # 5 integration tests (requires ROM files in Test Files/)
```

## Impact

All agents: if you change crypto, decryption, or header parsing code, integration tests will catch regressions. Run `cargo test -- --ignored` before merging crypto changes.
