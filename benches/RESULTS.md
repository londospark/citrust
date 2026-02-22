# Benchmark Results

Baseline performance data for citrust decryption across optimization phases.

## Test Environment

- **CPU:** AMD Ryzen (AES-NI, AVX2, SSE4.2 supported)
- **OS:** Windows
- **Build:** `cargo build --release` with `target-cpu=native` (Phase 2+)

## Performance Across Phases

| ROM | Size | Phase 1 | Phase 2 (AES-NI) | Phase 3 (mmap+rayon) | Python | Speedup vs Python |
|-----|------|---------|-------------------|----------------------|--------|-------------------|
| Pokemon Y | 1.75 GB | 1.64s | 0.92s* | ~1.0s | ~4.0s | ~4.0x |
| Omega Ruby | 1.84 GB | 2.23s | ~3.7s | 1.16s | 4.73s | 4.1x |
| Alpha Sapphire | 1.84 GB | 2.33s | — | 1.20s | 4.73s | 3.9x |

\* Phase 2 Pokemon Y measurement was warm cache; cold-cache Phase 2 is ~3.7s.

## Verified SHA256 Hashes (Decrypted Output)

All phases produce byte-identical output, confirmed by SHA256:

| ROM | SHA256 |
|-----|--------|
| Pokemon Y | `360173B4E7B1C76D8C83ACBBA17C575C1E646A506AFB8A41DB762F4ABDAEEF99` |
| Omega Ruby | `D38FC1031F33380B0297C955488843DF5592DC0459A90E1C0E880560F90174B9` |
| Alpha Sapphire | `77B5BE23BA109B56A4254C04F408E08F66E847B3A900B2BB264D63F4217BDA8F` |

## Phase Optimizations

1. **Phase 1:** Baseline Rust release build, no special flags
2. **Phase 2:** AES-NI via `target-cpu=native` + 4 MB RomFS chunks
3. **Phase 3:** memmap2 MmapMut (zero-copy in-place decryption) + rayon par_chunks_mut (parallel AES-CTR)

### Key Learnings

- **Allocation matters:** `vec![0u8; 64MB]` inside a loop caused a 2x regression; allocate once and reuse
- **mmap eliminates I/O overhead:** OS handles paging, no read/write syscalls, no buffer allocation
- **Phase 2 warm-cache was misleading:** True Phase 2 cold-cache is ~3.7s, making Phase 3's 1.16s a genuine 3.2x improvement
- **AES-NI is not the bottleneck:** With hardware AES, I/O (file reads/writes) dominates; mmap removes that entirely

## Notes

- Integration tests in `tests/integration_tests.rs` double as regression tests for all phases
- Micro-benchmarks available via `cargo bench` (criterion, see `benches/crypto_bench.rs`)
- Cross-tool comparison via `benches/compare.ps1`
