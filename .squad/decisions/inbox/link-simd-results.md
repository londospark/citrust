# SIMD/AES-NI Optimization Results — Issues #10 & #11

**Date:** 2026-02-22
**Test ROM:** Pokemon Y (1.75GB encrypted)
**CPU:** AMD Ryzen (native AES-NI support)
**Configuration:** Windows, Release build

## Issue #10: AES-NI Hardware Acceleration

### Investigation
- The `aes` 0.8 crate already uses runtime CPU feature detection via `cpufeatures` dependency
- However, without explicit `target-cpu=native`, the compiler doesn't emit optimal code paths
- Created `.cargo/config.toml` to enable all native CPU features including AES-NI, AVX, SSE4.2

### Results

| Configuration | Average Time | Speedup |
|--------------|--------------|---------|
| Baseline (no config) | 2.34s | 1.0x |
| **target-cpu=native** | **1.97s** | **1.19x** |

**Conclusion:** Enabling `target-cpu=native` provides a 19% speedup with zero code changes.

## Issue #11: AES-CTR Chunk Size Tuning

### Initial State
- RomFS: 16 MB chunks
- ExeFS: 1 MB chunks

### Benchmark Results (RomFS chunk sizes)

| Chunk Size | Average Time | Min | Max | Std Dev |
|-----------|--------------|-----|-----|---------|
| 2 MB | 1.48s | 1.35s | 1.57s | 0.08s |
| **4 MB** | **1.48s** | **1.42s** | **1.55s** | **0.04s** |
| 6 MB | 1.49s | 1.45s | 1.54s | 0.03s |
| 8 MB | 1.48s | 1.42s | 1.55s | 0.05s |
| 16 MB (original) | 1.96s | 1.85s | 2.03s | 0.08s |
| 32 MB | 3.01s | 2.81s | 3.21s | 0.15s |
| 64 MB | 3.10s | 2.85s | 3.53s | 0.29s |

**Key Findings:**
- 2MB, 4MB, 6MB, and 8MB perform nearly identically (~1.48s)
- **4MB selected** for most consistent performance (lowest std dev: 0.04s)
- 16MB (original) was 32% slower (1.96s vs 1.48s)
- Large chunks (32MB+) significantly degrade performance (likely L2/L3 cache pressure)

### Final Optimization

**Changed:** `decrypt.rs` line 279
`ust
// Before:
let romfs_block_size: usize = 16 * 1024 * 1024;  // 16 MB

// After:
let romfs_block_size: usize = 4 * 1024 * 1024;   // 4 MB
`

**Kept unchanged:** ExeFS 1 MB chunks (already optimal for smaller file sizes)

## Combined Results

| Phase | Configuration | Time | Improvement |
|-------|--------------|------|-------------|
| Original (Phase 1) | 16MB chunks, no target-cpu | 2.34s | baseline |
| + AES-NI | 16MB chunks, target-cpu=native | 1.97s | 1.19x |
| + Chunk tuning | 4MB chunks, target-cpu=native | 1.56s | 1.50x |

**Final speedup: 1.50x over baseline (2.34s → 1.56s)**

## Files Modified

1. **.cargo/config.toml** (created)
   - Enables `target-cpu=native` for all builds
   - Enables AES-NI, AVX, SSE4.2, and other native CPU features

2. **src/decrypt.rs** (line 279)
   - Changed RomFS chunk size from 16 MB → 4 MB
   - Verified byte-identical output (SHA256: 360173B4E7B1C76D8C83ACBBA17C575C1E646A506AFB8A41DB762F4ABDAEEF99)

## Validation

All configurations tested with 3-5 iterations each. Every run verified with SHA256 hash to ensure byte-identical decryption output.

## Next Steps (Phase 3)

- Issue #13-15: Multi-threading with rayon
- Expected speedup: 2-4x on multi-core CPUs
- Current bottleneck: Single-threaded I/O and crypto
