# SKILL: AES-NI Hardware Acceleration in Rust

## Context
Optimizing AES-CTR decryption performance using hardware acceleration (AES-NI) and cache-aware chunk sizing.

## Pattern

### 1. Enable Native CPU Features
Create `.cargo/config.toml`:
```toml
[build]
rustflags = ["-C", "target-cpu=native"]
```

This enables:
- AES-NI instructions (x86_64)
- AVX/AVX2 vector operations
- SSE4.2 instructions
- All other CPU-native features

### 2. Verify Runtime Detection
Check if your crypto crate uses `cpufeatures`:
```bash
grep cpufeatures Cargo.lock
```

RustCrypto crates (`aes`, `sha2`, etc.) automatically detect and use hardware acceleration when available.

### 3. Benchmark Chunk Sizes
For streaming AES-CTR operations, chunk size matters:

**Test methodology:**
```rust
// Test powers of 2 from 1MB to 64MB
let chunk_sizes = [1, 2, 4, 8, 16, 32, 64];
for size_mb in chunk_sizes {
    let chunk_size = size_mb * 1024 * 1024;
    // Benchmark decryption with this chunk
}
```

**Expected results:**
- 2-8 MB: Optimal (fits L3 cache on modern CPUs)
- 16+ MB: Degraded (cache pressure)
- 32+ MB: Significant degradation (memory bandwidth bottleneck)

### 4. Measure Consistently
Run multiple iterations and calculate standard deviation:
```powershell
for ( = 1;  -le 5; ++) {
    # Time operation
     += 
}
 = ( | Measure-Object -Average).Average
 = [math]::Sqrt(...)
```

Select configuration with **lowest variance**, not just fastest single run.

## Typical Speedups
- `target-cpu=native` alone: **1.15-1.25x** (AES-NI + vectorization)
- Chunk size optimization: **1.2-1.4x** (cache locality)
- Combined: **1.5-1.7x** speedup over baseline

## Verification
Always verify correctness with cryptographic hash (SHA256) after changes:
```powershell
 = (Get-FileHash  -Algorithm SHA256).Hash
```

## When to Apply
- ✅ AES-CTR, AES-GCM, or other hardware-accelerated crypto
- ✅ Large file processing (>100 MB)
- ✅ Desktop/server targets (not embedded)
- ❌ Cross-compilation targets without AES-NI
- ❌ Web/WASM targets (use portable build)

## Trade-offs
**Pros:**
- Zero code changes (config-only)
- Significant speedup (1.5x+)
- No dependencies added

**Cons:**
- Binary not portable to older CPUs (pre-2010 x86_64)
- Larger binary size (~5-10%)
- Must test on target hardware

## Related
- **Crates:** `aes`, `ctr`, `cipher`, `cpufeatures`
- **Flags:** `-C target-cpu=native`, `-C target-feature=+aes`
- **Tools:** `cargo bench`, PowerShell `Measure-Command`
