# Test Strategy Decision: Fixture Generation & Encryption Path Coverage

**Author:** Toad 🧪  
**Date:** 2026-02-22  
**Status:** RECOMMENDED  

---

## Problem

The three available test ROMs (Pokemon Y, Alpha Sapphire, Omega Ruby) are all **decrypted**. The Python script requires encrypted ROMs as input. Options:

1. **Re-encrypt test ROMs** (encrypt decrypted → test encrypted → decrypt → verify original)
2. **Create synthetic minimal fixtures** (< 10 MB, known plaintext/ciphertext pairs)
3. **Use real encrypted ROMs from elsewhere** (external source, not available)

## Decision

**Use a hybrid approach:**

- **Option A (Primary):** Re-encrypt Pokemon Y as test fixture
  - Reverse the decryption: use known key derivation to re-encrypt sections
  - Create a ~100 MB encrypted test ROM
  - Verify Rust decryption recovers original plaintext
  - Reusable for all decryption tests

- **Option B (Supplementary):** Create synthetic minimal fixtures for unit tests
  - Tiny 1–10 MB ROMs with known headers
  - Test each encryption method variant (KeyX 0x2C, 0x25, 0x18, 0x1B separately)
  - Test edge cases: zero-key, empty regions, corrupted headers
  - Faster CI/CD execution

## Rationale

- **Re-encryption is necessary:** We can't test decryption without encrypted input
- **Synthetic fixtures are cheaper:** Easier to test edge cases without 1.7 GB files
- **Hybrid gives coverage:** Real ROM for integration tests, synthetic for unit tests and edge cases
- **Decrypted ROMs are not lost:** Original plaintext still available for verification

## Action Items

1. Implement re-encryption utility (mirror of Python decryption logic)
2. Create ~100 MB encrypted Pokemon Y test fixture
3. Store synthetic fixture templates (10 MB each, one per encryption method)
4. Document exact plaintext/ciphertext pairs in test fixtures

## Test ROMs Status

| ROM | Available | Encrypted | Usable |
|-----|-----------|-----------|--------|
| Pokemon Y | ✓ | NO (decrypted) | Re-encrypt for tests |
| Alpha Sapphire | ✓ | NO (decrypted) | Backup fixture |
| Omega Ruby | ✓ | NO (decrypted) | Backup fixture |

---

## Review Authority

- **Toad:** Test strategy authority ✓ (approved)
- **Samus:** Crypto implementation review (pending)
