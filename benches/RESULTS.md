# Benchmark Results

Baseline performance data for citrust decryption across optimization phases.

## Test Environment

- **CPU:** AMD Ryzen (AES-NI, AVX2, SSE4.2 supported)
- **OS:** Windows
- **Build:** `cargo build --release` with `target-cpu=native` (Phase 2+)

## Phase 1 → Phase 2 → Python Comparison

| ROM | Size | Phase 1 (no SIMD) | Phase 2 (AES-NI) | Speedup (P1→P2) | Python | Speedup vs Python |
|-----|------|-------------------|-------------------|------------------|--------|-------------------|
| Pokemon Y | 1.75 GB | 1.64s | 0.92s | 1.78x | ~4.0s | ~4.3x |
| Omega Ruby | ~1.8 GB | 2.23s | ~0.97s | 2.30x | 4.41s | ~4.5x |
| Alpha Sapphire | ~1.8 GB | 2.33s | ~0.97s | 2.40x | 4.76s | ~4.9x |

## Verified SHA256 Hashes (Decrypted Output)

All phases produce byte-identical output, confirmed by SHA256:

| ROM | SHA256 |
|-----|--------|
| Pokemon Y | `360173B4E7B1C76D8C83ACBBA17C575C1E646A506AFB8A41DB762F4ABDAEEF99` |
| Omega Ruby | `D38FC1031F33380B0297C955488843DF5592DC0459A90E1C0E880560F90174B9` |

## Phase 2 Optimizations Applied

1. **AES-NI hardware acceleration** — `.cargo/config.toml` sets `target-cpu=native`, enabling AES-NI, AVX, SSE4.2 with zero code changes
2. **Chunk size tuning** — RomFS chunk size reduced from 16 MB → 4 MB for better L3 cache utilization (benchmarked 2–64 MB range; 2–8 MB optimal, 4 MB selected for lowest variance)

## Notes

- Phase 1 baseline: default Rust release build, no target-cpu flags
- Phase 2: `.cargo/config.toml` with `target-cpu=native` + 4 MB RomFS chunks
- Phase 3 (multi-threading with rayon): pending — this table serves as the baseline
- Integration tests in `tests/integration_tests.rs` double as regression tests for all phases
- Micro-benchmarks available via `cargo bench` (criterion, see `benches/crypto_bench.rs`)
- Cross-tool comparison via `benches/compare.ps1`
