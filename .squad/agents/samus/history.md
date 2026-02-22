# Samus — History

## Project Context
- **Project:** citrust — Rust port of b3DSDecrypt.py (3DS ROM decryption tool) with SteamOS GUI
- **Stack:** Rust, AES-CTR crypto, binary parsing, GUI framework, SteamOS packaging
- **User:** Gareth
- **Source:** b3DS-master/b3DSDecrypt.py — Python 2 script that decrypts 3DS NCSD/NCCH ROMs using AES-128-CTR

## Learnings

<!-- Append learnings below -->

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
