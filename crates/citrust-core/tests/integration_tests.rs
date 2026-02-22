//! Integration tests for end-to-end decryption correctness.
//!
//! These tests require actual ROM files in `Test Files/` and are marked `#[ignore]`.
//! Run with: `cargo test -- --ignored`
//!
//! These also serve as **Phase 2 regression tests**: identical SHA256 hashes confirm
//! that AES-NI acceleration and chunk-size tuning preserve byte-identical output.

use std::fs;
use std::io::Read;
use std::path::PathBuf;

use citrust_core::keydb::KeyDatabase;
use sha2::{Digest, Sha256};

const POKEMON_Y: &str = "0451 - Pokemon Y (Europe) (En,Ja,Fr,De,Es,It,Ko) Decrypted.3ds";
const OMEGA_RUBY: &str = "1325 - Pokemon Omega Ruby (Europe) (En,Ja,Fr,De,Es,It,Ko) Decrypted.3ds";

const POKEMON_Y_HASH: &str = "360173B4E7B1C76D8C83ACBBA17C575C1E646A506AFB8A41DB762F4ABDAEEF99";
const OMEGA_RUBY_HASH: &str = "D38FC1031F33380B0297C955488843DF5592DC0459A90E1C0E880560F90174B9";

/// Check if a ROM file exists in the `Test Files/` directory.
fn test_rom_path(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from("Test Files").join(name);
    if path.exists() { Some(path) } else { None }
}

/// Compute SHA256 hash of a file, returning uppercase hex string.
fn sha256_file(path: &PathBuf) -> String {
    let mut file = fs::File::open(path).expect("Failed to open file for hashing");
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).expect("Failed to read file");
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    format!("{:X}", hasher.finalize())
}

// ---------------------------------------------------------------------------
// Issue #8: Round-trip decryption tests
// ---------------------------------------------------------------------------

/// Decrypt Pokemon Y and verify SHA256 matches known-good hash.
/// Exercises the Original crypto method (KeyX 0x2C) path.
#[test]
#[ignore]
fn decrypt_pokemon_y_matches_known_hash() {
    let src = test_rom_path(POKEMON_Y).expect("Pokemon Y ROM not found in Test Files/");

    let tmp = PathBuf::from("test-fixtures").join("temp_pokemon_y.3ds");
    fs::copy(&src, &tmp).expect("Failed to copy ROM to temp file");

    let result = citrust_core::decrypt::decrypt_rom(&tmp, &make_test_keydb(), |msg| {
        eprintln!("{msg}");
    });
    assert!(result.is_ok(), "Decryption failed: {:?}", result.err());

    let hash = sha256_file(&tmp);
    let _ = fs::remove_file(&tmp);

    assert_eq!(hash, POKEMON_Y_HASH, "Pokemon Y decrypted hash mismatch");
}

/// Decrypt Omega Ruby and verify SHA256 matches known-good hash.
/// Exercises the Key7x crypto method (KeyX 0x25) and .code double-layer decryption.
#[test]
#[ignore]
fn decrypt_omega_ruby_matches_known_hash() {
    let src = test_rom_path(OMEGA_RUBY).expect("Omega Ruby ROM not found in Test Files/");

    let tmp = PathBuf::from("test-fixtures").join("temp_omega_ruby.3ds");
    fs::copy(&src, &tmp).expect("Failed to copy ROM to temp file");

    let result = citrust_core::decrypt::decrypt_rom(&tmp, &make_test_keydb(), |msg| {
        eprintln!("{msg}");
    });
    assert!(result.is_ok(), "Decryption failed: {:?}", result.err());

    let hash = sha256_file(&tmp);
    let _ = fs::remove_file(&tmp);

    assert_eq!(hash, OMEGA_RUBY_HASH, "Omega Ruby decrypted hash mismatch");
}

/// Decrypt a ROM, then decrypt again — second pass should be a no-op.
/// Verifies that the NoCrypto flag is set correctly after first decryption.
#[test]
#[ignore]
fn decrypt_already_decrypted_is_noop() {
    let src = test_rom_path(POKEMON_Y).expect("Pokemon Y ROM not found in Test Files/");

    let tmp = PathBuf::from("test-fixtures").join("temp_noop_test.3ds");
    fs::copy(&src, &tmp).expect("Failed to copy ROM to temp file");

    let keydb = make_test_keydb();

    // First decryption
    citrust_core::decrypt::decrypt_rom(&tmp, &keydb, |_| {}).expect("First decryption failed");
    let hash_after_first = sha256_file(&tmp);

    // Second decryption — should detect NoCrypto flag and skip all partitions
    citrust_core::decrypt::decrypt_rom(&tmp, &keydb, |_| {}).expect("Second decryption failed");
    let hash_after_second = sha256_file(&tmp);

    let _ = fs::remove_file(&tmp);

    assert_eq!(
        hash_after_first, hash_after_second,
        "Second decryption modified an already-decrypted ROM"
    );
}

/// Parse NCSD header from a real ROM and verify structural properties.
#[test]
#[ignore]
fn ncsd_header_from_real_rom() {
    let src = test_rom_path(POKEMON_Y).expect("Pokemon Y ROM not found in Test Files/");

    let mut file = fs::File::open(&src).expect("Failed to open ROM");
    let ncsd =
        citrust_core::ncsd::NcsdHeader::parse(&mut file).expect("Failed to parse NCSD header");

    assert!(ncsd.sector_size >= 0x200, "Sector size too small");
    assert!(
        ncsd.sector_size.is_power_of_two(),
        "Sector size not a power of 2"
    );

    let partition_count = ncsd.partitions.iter().filter(|p| !p.is_empty()).count();
    assert!(partition_count > 0, "No partitions found in ROM");
}

/// Parse first NCCH partition header from a real ROM and verify key fields.
#[test]
#[ignore]
fn ncch_header_from_real_rom() {
    let src = test_rom_path(POKEMON_Y).expect("Pokemon Y ROM not found in Test Files/");

    let mut file = fs::File::open(&src).expect("Failed to open ROM");
    let ncsd =
        citrust_core::ncsd::NcsdHeader::parse(&mut file).expect("Failed to parse NCSD header");

    let part = &ncsd.partitions[0];
    assert!(!part.is_empty(), "First partition is empty");

    let part_offset = part.offset_bytes(ncsd.sector_size);
    let ncch = citrust_core::ncch::NcchHeader::parse(&mut file, part_offset)
        .expect("Failed to parse NCCH header");

    assert_ne!(ncch.key_y, 0, "KeyY should be non-zero");
    assert_ne!(ncch.title_id, 0, "TitleID should be non-zero");
}

// ---------------------------------------------------------------------------
// Key database integration tests
// ---------------------------------------------------------------------------

/// Try to load a KeyDatabase from test fixtures or default locations.
fn make_test_keydb() -> KeyDatabase {
    // Try to load from test fixture file (gitignored, user must provide)
    let fixture_path = PathBuf::from("test-fixtures/aes_keys.txt");
    if fixture_path.exists() {
        return KeyDatabase::from_file(&fixture_path)
            .expect("Failed to parse test-fixtures/aes_keys.txt");
    }
    // Try default locations
    if let Some(path) = KeyDatabase::search_default_locations() {
        return KeyDatabase::from_file(&path)
            .expect("Failed to parse key file from default location");
    }
    panic!(
        "No key file available for integration tests. Place aes_keys.txt in test-fixtures/ or a default location."
    );
}

/// Decrypt Pokemon Y with external keys and verify the hash matches the known-good value.
/// This proves the external key path produces byte-identical output to built-in keys.
#[test]
#[ignore]
fn decrypt_with_external_keys_matches_builtin() {
    let src = test_rom_path(POKEMON_Y).expect("Pokemon Y ROM not found in Test Files/");
    let keydb = make_test_keydb();

    let tmp = PathBuf::from("test-fixtures").join("temp_keydb_test.3ds");
    fs::copy(&src, &tmp).expect("Failed to copy ROM to temp file");

    let result = citrust_core::decrypt::decrypt_rom(&tmp, &keydb, |msg| {
        eprintln!("{msg}");
    });
    assert!(
        result.is_ok(),
        "Decryption with external keys failed: {:?}",
        result.err()
    );

    let hash = sha256_file(&tmp);
    let _ = fs::remove_file(&tmp);

    assert_eq!(
        hash, POKEMON_Y_HASH,
        "Decryption with external keys produced different output than built-in keys"
    );
}

/// Verify KeyDatabase has the expected structure when loaded.
#[test]
#[ignore] // Requires aes_keys.txt
fn keydb_has_expected_keys() {
    let db = make_test_keydb();
    assert!(db.len() >= 5, "Key database should have at least 5 keys");
    assert!(db.generator().is_some(), "Missing generator key");
    assert!(db.get_key_x(0x2C).is_some(), "Missing slot0x2CKeyX");
    assert!(db.get_key_x(0x25).is_some(), "Missing slot0x25KeyX");
    assert!(db.get_key_x(0x18).is_some(), "Missing slot0x18KeyX");
    assert!(db.get_key_x(0x1B).is_some(), "Missing slot0x1BKeyX");
}
