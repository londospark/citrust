use std::fs::File;
use std::io::{self, Cursor};
use std::path::Path;

use memmap2::MmapMut;
use rayon::prelude::*;

use crate::crypto::aes_ctr_decrypt;
use crate::keydb::KeyDatabase;
use crate::keys::{CryptoMethod, Key128};
use crate::ncch::NcchHeader;
use crate::ncsd::NcsdHeader;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a 3DS ROM (invalid NCSD magic)")]
    NotNcsd,
    #[error("partition {0}: invalid NCCH header")]
    InvalidNcch(u8),
    #[error("key not found in database: {0}")]
    KeyNotFound(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// Chunk size for rayon parallel decryption
const CHUNK_SIZE: usize = 4 * 1024 * 1024;

/// Check if a byte is valid ASCII (printable 0x20-0x7E or null 0x00).
fn is_valid_ascii_byte(b: u8) -> bool {
    b == 0x00 || (0x20..=0x7E).contains(&b)
}

/// Detect if a partition's content is already decrypted despite NoCrypto not being set.
///
/// Checks the ExeFS filename table (first 8 bytes of the ExeFS region). Decrypted ExeFS
/// entries contain valid ASCII names like ".code\0\0\0", "banner\0\0", "icon\0\0\0\0".
/// Encrypted data will be random bytes that fail this check.
///
/// If there is no ExeFS but an ExHeader exists, falls back to checking if the first 8
/// bytes of the ExHeader are valid ASCII (decrypted ExHeaders start with the codeset name).
pub fn is_content_decrypted(
    data: &[u8],
    ncch: &NcchHeader,
    sector_size: u32,
    part_offset: usize,
) -> bool {
    let ss = sector_size as usize;

    // Primary check: ExeFS filename table
    if ncch.exefs_length > 0 {
        let exefs_base = part_offset + ncch.exefs_offset as usize * ss;
        if exefs_base + 8 <= data.len() {
            return data[exefs_base..exefs_base + 8]
                .iter()
                .all(|&b| is_valid_ascii_byte(b));
        }
    }

    // Fallback: ExHeader (starts 0x200 bytes into the partition)
    if ncch.exheader_length > 0 {
        let exheader_off = part_offset + ss;
        if exheader_off + 8 <= data.len() {
            return data[exheader_off..exheader_off + 8]
                .iter()
                .all(|&b| is_valid_ascii_byte(b));
        }
    }

    false
}

/// Decrypt a slice in-place using parallel AES-CTR.
fn decrypt_slice(
    data: &mut [u8],
    key: &Key128,
    key_second: Option<&Key128>,
    base_iv: u128,
    chunk_size: usize,
) {
    data.par_chunks_mut(chunk_size)
        .enumerate()
        .for_each(|(i, chunk)| {
            let blocks_before = (i * chunk_size) as u128 / 16;
            let chunk_iv = base_iv + blocks_before;
            aes_ctr_decrypt(key, chunk_iv, chunk);
            if let Some(k2) = key_second {
                aes_ctr_decrypt(k2, chunk_iv, chunk);
            }
        });
}

/// Resolve KeyX for a given crypto method from the key database.
fn resolve_key_x(method: CryptoMethod, keydb: &KeyDatabase) -> Result<u128, Error> {
    let slot = match method {
        CryptoMethod::Original => 0x2C,
        CryptoMethod::Key7x => 0x25,
        CryptoMethod::Key93 => 0x18,
        CryptoMethod::Key96 => 0x1B,
    };
    keydb
        .get_key_x(slot)
        .ok_or_else(|| Error::KeyNotFound(format!("slot0x{:02X}KeyX", slot)))
}

/// Resolve the generator constant from the key database.
fn resolve_constant(keydb: &KeyDatabase) -> Result<u128, Error> {
    keydb
        .generator()
        .ok_or_else(|| Error::KeyNotFound("generator".to_string()))
}

/// Resolve KeyX for slot 0x2C specifically (used for ExHeader/ExeFS base layer).
fn resolve_key_x_2c(keydb: &KeyDatabase) -> Result<u128, Error> {
    keydb
        .get_key_x(0x2C)
        .ok_or_else(|| Error::KeyNotFound("slot0x2CKeyX".to_string()))
}

pub fn decrypt_rom(
    path: &Path,
    keydb: &KeyDatabase,
    mut on_progress: impl FnMut(&str),
) -> Result<(), Error> {
    on_progress(&format!(
        "Using external key database ({} keys loaded)",
        keydb.len()
    ));

    let file = File::options().read(true).write(true).open(path)?;
    // SAFETY: we are the sole accessor of this file during decryption
    let mut mmap = unsafe { MmapMut::map_mut(&file)? };

    // Parse NCSD header from the mapped memory
    let ncsd = {
        let mut cursor = Cursor::new(&mmap[..]);
        NcsdHeader::parse(&mut cursor).map_err(|_| Error::NotNcsd)?
    };
    let sector_size = ncsd.sector_size;

    for p in 0..8u8 {
        let part = &ncsd.partitions[p as usize];
        if part.is_empty() {
            on_progress(&format!("Partition {p} Not found... Skipping..."));
            continue;
        }

        let part_off = part.offset_bytes(sector_size) as usize;

        // Verify NCCH magic
        if &mmap[part_off + 0x100..part_off + 0x104] != b"NCCH" {
            on_progress(&format!("Partition {p} Unable to read NCCH header"));
            continue;
        }

        let ncch = {
            let mut cursor = Cursor::new(&mmap[..]);
            NcchHeader::parse(&mut cursor, part_off as u64)?
        };

        let ncch = if ncch.is_no_crypto() {
            if is_content_decrypted(&mmap, &ncch, sector_size, part_off) {
                on_progress(&format!("Partition {p}: Already Decrypted ✓"));
                continue;
            }
            // NoCrypto flag set but content is actually encrypted
            on_progress(&format!(
                "Partition {p}: Flagged as decrypted but content is encrypted, decrypting..."
            ));
            // Clear NoCrypto bit and recover backup crypto_method from NCSD header
            mmap[part_off + 0x18F] &= !0x04;
            let backup_offset = 0x1188 + (p as usize * 8) + 3;
            if backup_offset < mmap.len() {
                let backup_crypto = mmap[backup_offset];
                if backup_crypto != 0 && CryptoMethod::from_flag(backup_crypto).is_some() {
                    mmap[part_off + 0x18B] = backup_crypto;
                }
            }
            // Re-parse NCCH with corrected flags
            let mut cursor = Cursor::new(&mmap[..]);
            NcchHeader::parse(&mut cursor, part_off as u64)?
        } else {
            ncch
        };

        // Content-based detection: check if data is already plaintext despite NoCrypto not set
        if is_content_decrypted(&mmap, &ncch, sector_size, part_off) {
            on_progress(&format!(
                "Partition {p}: Content already decrypted (mis-flagged ROM), setting NoCrypto flag..."
            ));
            // Skip decryption, just patch the flags
            mmap[part_off + 0x18B] = 0x00;
            let mut flag = ncch.partition_flags[7];
            flag &= !0x01;
            flag &= !0x20;
            flag |= 0x04;
            mmap[part_off + 0x18F] = flag;
            continue;
        }

        let key_y = ncch.key_y;
        let (normal_key_2c, normal_key) = if ncch.is_fixed_key() {
            if p == 0 {
                on_progress("Encryption Method: Zero Key");
            }
            (0u128, 0u128)
        } else {
            let constant = resolve_constant(keydb)?;
            let key_x_2c = resolve_key_x_2c(keydb)?;
            let nk2c = crate::crypto::derive_normal_key(key_x_2c, key_y, constant);
            let method = ncch.crypto_method().unwrap_or(CryptoMethod::Original);
            let key_x = resolve_key_x(method, keydb)?;
            let nk = crate::crypto::derive_normal_key(key_x, key_y, constant);
            if p == 0 {
                on_progress(&format!("Encryption Method: {method:?}"));
            }
            (nk2c, nk)
        };

        let key_2c: Key128 = normal_key_2c.to_be_bytes();
        let key_main: Key128 = normal_key.to_be_bytes();

        // ======= DECRYPT EXHEADER (2KB, too small to parallelize) =======
        if ncch.exheader_length > 0 {
            let off = (part.offset_sectors as usize + 1) * sector_size as usize;
            aes_ctr_decrypt(&key_2c, ncch.plain_iv(), &mut mmap[off..off + 0x800]);
            on_progress(&format!("Partition {p} ExeFS: Decrypting: ExHeader"));
        }

        // ======= DECRYPT EXEFS =======
        if ncch.exefs_length > 0 {
            let exefs_base =
                (part.offset_sectors as usize + ncch.exefs_offset as usize) * sector_size as usize;
            let exefs_iv = ncch.exefs_iv();
            let ss = sector_size as usize;

            // Step 1: Decrypt filename table (first sector, small)
            aes_ctr_decrypt(&key_2c, exefs_iv, &mut mmap[exefs_base..exefs_base + ss]);
            on_progress(&format!(
                "Partition {p} ExeFS: Decrypting: ExeFS Filename Table"
            ));

            // Step 2: .code double-layer (only for 7.x/9.x keys)
            let method = ncch.crypto_method().unwrap_or(CryptoMethod::Original);
            if matches!(
                method,
                CryptoMethod::Key7x | CryptoMethod::Key93 | CryptoMethod::Key96
            ) {
                // Read .code entry from the now-decrypted filename table
                for j in 0..10u32 {
                    let slot = exefs_base + (j as usize * 0x10);
                    if slot + 0x10 > exefs_base + ss {
                        break;
                    }
                    if &mmap[slot..slot + 8] == b".code\x00\x00\x00" {
                        let code_file_off =
                            u32::from_le_bytes(mmap[slot + 8..slot + 12].try_into().unwrap());
                        let code_file_len =
                            u32::from_le_bytes(mmap[slot + 12..slot + 16].try_into().unwrap())
                                as usize;

                        if code_file_len > 0 {
                            let ctr_offset = (code_file_off as u128 + ss as u128) / 0x10;
                            let code_iv = exefs_iv + ctr_offset;
                            let code_start = exefs_base + ss + code_file_off as usize;

                            on_progress(&format!(
                                "Partition {p} ExeFS: Decrypting: .code ({} mb)",
                                code_file_len / (1024 * 1024)
                            ));
                            decrypt_slice(
                                &mut mmap[code_start..code_start + code_file_len],
                                &key_main,
                                Some(&key_2c),
                                code_iv,
                                1024 * 1024,
                            );
                            on_progress(&format!(
                                "Partition {p} ExeFS: Decrypting: .code... Done!"
                            ));
                        }
                        break;
                    }
                }
            }

            // Step 3: Decrypt remaining ExeFS data (after first sector)
            let exefs_data_sectors = ncch.exefs_length.saturating_sub(1) as usize;
            if exefs_data_sectors > 0 {
                let data_size = exefs_data_sectors * ss;
                let data_iv = exefs_iv + (ss as u128 / 0x10);
                let data_start = exefs_base + ss;

                on_progress(&format!("Partition {p} ExeFS: Decrypting: data"));
                decrypt_slice(
                    &mut mmap[data_start..data_start + data_size],
                    &key_2c,
                    None,
                    data_iv,
                    1024 * 1024,
                );
                on_progress(&format!("Partition {p} ExeFS: Decrypting: Done"));
            }
        } else {
            on_progress(&format!("Partition {p} ExeFS: No Data... Skipping..."));
        }

        // ======= DECRYPT ROMFS (bulk — parallel) =======
        if ncch.romfs_offset != 0 {
            let romfs_total = ncch.romfs_length as usize * sector_size as usize;
            let romfs_iv = ncch.romfs_iv();
            let romfs_start =
                (part.offset_sectors as usize + ncch.romfs_offset as usize) * sector_size as usize;

            on_progress(&format!(
                "Partition {p} RomFS: Decrypting: {} mb",
                romfs_total / (1024 * 1024)
            ));
            decrypt_slice(
                &mut mmap[romfs_start..romfs_start + romfs_total],
                &key_main,
                None,
                romfs_iv,
                CHUNK_SIZE,
            );
            on_progress(&format!("Partition {p} RomFS: Decrypting: Done"));
        } else {
            on_progress(&format!("Partition {p} RomFS: No Data... Skipping..."));
        }

        // ======= PATCH FLAGS (direct byte writes, zero-copy) =======
        mmap[part_off + 0x18B] = 0x00;
        let mut flag = ncch.partition_flags[7];
        flag &= !0x01;
        flag &= !0x20;
        flag |= 0x04;
        mmap[part_off + 0x18F] = flag;
    }

    mmap.flush()?;
    on_progress("Done...");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ncch::NcchHeader;

    /// Helper: create a minimal NcchHeader with specified ExeFS parameters.
    fn make_ncch(exefs_offset: u32, exefs_length: u32) -> NcchHeader {
        NcchHeader {
            key_y: 0,
            title_id: 0x0004000000055D00,
            partition_flags: [0u8; 8],
            exheader_length: 0,
            plain_offset: 0,
            plain_length: 0,
            logo_offset: 0,
            logo_length: 0,
            exefs_offset,
            exefs_length,
            romfs_offset: 0,
            romfs_length: 0,
        }
    }

    /// Helper: build a mock data buffer with an ExeFS filename entry at the correct offset.
    fn build_exefs_data(
        part_offset: usize,
        sector_size: u32,
        exefs_offset_sectors: u32,
        first_entry: &[u8; 8],
    ) -> Vec<u8> {
        let exefs_base = part_offset + (exefs_offset_sectors as usize) * (sector_size as usize);
        let total_size = exefs_base + sector_size as usize;
        let mut data = vec![0u8; total_size];
        data[exefs_base..exefs_base + 8].copy_from_slice(first_entry);
        data
    }

    // -----------------------------------------------------------------
    // Tests for is_content_decrypted()
    // NOTE: These depend on Link's implementation of is_content_decrypted().
    // If that function doesn't exist yet, these tests won't compile — that's
    // expected. They define the contract for the function.
    // -----------------------------------------------------------------

    /// Valid plaintext ExeFS with ".code\0\0\0" as first entry → detected as decrypted.
    #[test]
    fn test_is_content_decrypted_with_plaintext_exefs() {
        let sector_size = 0x200u32;
        let part_offset = 0usize;
        let exefs_off = 4u32;
        let ncch = make_ncch(exefs_off, 2);

        let data = build_exefs_data(part_offset, sector_size, exefs_off, b".code\x00\x00\x00");

        assert!(is_content_decrypted(&data, &ncch, sector_size, part_offset));
    }

    /// Random/encrypted bytes in ExeFS first entry → not detected as decrypted.
    #[test]
    fn test_is_content_decrypted_with_encrypted_exefs() {
        let sector_size = 0x200u32;
        let part_offset = 0usize;
        let exefs_off = 4u32;
        let ncch = make_ncch(exefs_off, 2);

        let entry: [u8; 8] = [0xFF, 0xA3, 0x7B, 0x92, 0xDE, 0x01, 0xC4, 0x88];
        let data = build_exefs_data(part_offset, sector_size, exefs_off, &entry);

        assert!(!is_content_decrypted(
            &data,
            &ncch,
            sector_size,
            part_offset
        ));
    }

    /// NCCH with exefs_length == 0 → can't determine, return false (proceed with decryption).
    #[test]
    fn test_is_content_decrypted_no_exefs() {
        let sector_size = 0x200u32;
        let part_offset = 0usize;
        let ncch = make_ncch(0, 0);

        let data = vec![0u8; 0x1000];

        assert!(!is_content_decrypted(
            &data,
            &ncch,
            sector_size,
            part_offset
        ));
    }

    /// Other known ExeFS filenames ("banner", "icon", "logo") should also be detected.
    #[test]
    fn test_is_content_decrypted_with_known_names() {
        let sector_size = 0x200u32;
        let part_offset = 0usize;
        let exefs_off = 4u32;
        let ncch = make_ncch(exefs_off, 2);

        let known_names: [[u8; 8]; 3] = [
            *b"banner\x00\x00",
            *b"icon\x00\x00\x00\x00",
            *b"logo\x00\x00\x00\x00",
        ];

        for name in &known_names {
            let data = build_exefs_data(part_offset, sector_size, exefs_off, name);
            assert!(
                is_content_decrypted(&data, &ncch, sector_size, part_offset),
                "Expected true for {:?}",
                String::from_utf8_lossy(name)
            );
        }
    }

    fn make_test_keydb() -> KeyDatabase {
        use std::io::Cursor;
        let keys_text = "generator=FEDCBA9876543210FEDCBA9876543210\nslot0x2CKeyX=00000000000000000000000000000001\n";
        KeyDatabase::from_reader(Cursor::new(keys_text)).unwrap()
    }

    /// End-to-end: decrypt_rom on a minimal synthetic ROM with already-decrypted content.
    /// Verifies data is not corrupted and NoCrypto flag is set.
    /// Depends on Link's content-detection logic in decrypt_rom().
    #[test]
    fn test_decrypt_rom_skips_already_decrypted() {
        use std::io::Write;

        let sector_size = 0x200u32;
        let part_sector: u32 = 1;
        let part_offset = part_sector as usize * sector_size as usize;
        let exefs_off_sectors: u32 = 4;
        let exefs_len_sectors: u32 = 2;

        let total_size = part_offset
            + (exefs_off_sectors as usize + exefs_len_sectors as usize) * sector_size as usize;
        let mut rom = vec![0u8; total_size];

        // --- NCSD header ---
        rom[0x100..0x104].copy_from_slice(b"NCSD");
        rom[0x18E] = 0; // flags[6]=0 → sector_size = 0x200
        let part_len = exefs_off_sectors + exefs_len_sectors + 1;
        rom[0x120..0x124].copy_from_slice(&part_sector.to_le_bytes());
        rom[0x124..0x128].copy_from_slice(&part_len.to_le_bytes());

        // --- NCCH header at partition ---
        rom[part_offset + 0x100..part_offset + 0x104].copy_from_slice(b"NCCH");
        rom[part_offset + 0x108..part_offset + 0x110]
            .copy_from_slice(&0x0004000000055D00u64.to_le_bytes());
        // No exheader
        rom[part_offset + 0x180..part_offset + 0x184].copy_from_slice(&0u32.to_le_bytes());
        // FixedKey (zero-key) but NOT NoCrypto — forces entry into decryption path
        rom[part_offset + 0x18B] = 0x00;
        rom[part_offset + 0x18F] = 0x01;
        // ExeFS
        rom[part_offset + 0x1A0..part_offset + 0x1A4]
            .copy_from_slice(&exefs_off_sectors.to_le_bytes());
        rom[part_offset + 0x1A4..part_offset + 0x1A8]
            .copy_from_slice(&exefs_len_sectors.to_le_bytes());
        // No RomFS
        rom[part_offset + 0x1B0..part_offset + 0x1B4].copy_from_slice(&0u32.to_le_bytes());
        rom[part_offset + 0x1B4..part_offset + 0x1B8].copy_from_slice(&0u32.to_le_bytes());

        // --- ExeFS plaintext filename table ---
        let exefs_base = part_offset + exefs_off_sectors as usize * sector_size as usize;
        rom[exefs_base..exefs_base + 8].copy_from_slice(b".code\x00\x00\x00");

        // Write to temp file
        let tmp_dir = std::path::PathBuf::from("test-fixtures");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let tmp_path = tmp_dir.join("temp_content_detect.3ds");
        {
            let mut f = std::fs::File::create(&tmp_path).expect("create temp file");
            f.write_all(&rom).expect("write temp file");
        }

        let result = decrypt_rom(&tmp_path, &make_test_keydb(), |msg| {
            eprintln!("  [content-detect] {msg}");
        });

        let output = std::fs::read(&tmp_path).expect("read result");
        let _ = std::fs::remove_file(&tmp_path);

        assert!(result.is_ok(), "decrypt_rom failed: {:?}", result.err());

        // ExeFS filename should be untouched (no decryption applied)
        assert_eq!(
            &output[exefs_base..exefs_base + 8],
            b".code\x00\x00\x00",
            "ExeFS filename corrupted — content detection didn't skip decryption"
        );

        // ExeFS data (second sector) should still be zeros
        let data_start = exefs_base + sector_size as usize;
        assert!(
            output[data_start..data_start + sector_size as usize]
                .iter()
                .all(|&b| b == 0),
            "ExeFS data sectors corrupted — content detection didn't skip decryption"
        );

        // NoCrypto flag should now be set
        assert_eq!(
            output[part_offset + 0x18F] & 0x04,
            0x04,
            "NoCrypto flag should be set after content detection"
        );
    }

    /// NoCrypto flag is set but ExeFS content is encrypted (random bytes).
    /// Verifies that decryption proceeds instead of blindly trusting the flag.
    #[test]
    fn test_decrypt_detects_encrypted_despite_nocrypto_flag() {
        use std::io::Write;
        use std::sync::Mutex;

        let sector_size = 0x200u32;
        let part_sector: u32 = 1;
        let part_offset = part_sector as usize * sector_size as usize;
        let exefs_off_sectors: u32 = 4;
        let exefs_len_sectors: u32 = 2;

        let total_size = part_offset
            + (exefs_off_sectors as usize + exefs_len_sectors as usize) * sector_size as usize;
        // Need at least 0x1190 bytes for backup NCSD flags
        let total_size = total_size.max(0x1200);
        let mut rom = vec![0u8; total_size];

        // --- NCSD header ---
        rom[0x100..0x104].copy_from_slice(b"NCSD");
        rom[0x18E] = 0; // sector_size = 0x200
        let part_len = exefs_off_sectors + exefs_len_sectors + 1;
        rom[0x120..0x124].copy_from_slice(&part_sector.to_le_bytes());
        rom[0x124..0x128].copy_from_slice(&part_len.to_le_bytes());

        // --- NCCH header at partition ---
        rom[part_offset + 0x100..part_offset + 0x104].copy_from_slice(b"NCCH");
        rom[part_offset + 0x108..part_offset + 0x110]
            .copy_from_slice(&0x0004000000055D00u64.to_le_bytes());
        rom[part_offset + 0x180..part_offset + 0x184].copy_from_slice(&0u32.to_le_bytes());
        rom[part_offset + 0x18B] = 0x00; // crypto_method = Original
        // NoCrypto + FixedKey: flags[7] = 0x04 | 0x01 = 0x05
        rom[part_offset + 0x18F] = 0x05;
        // ExeFS offset/length
        rom[part_offset + 0x1A0..part_offset + 0x1A4]
            .copy_from_slice(&exefs_off_sectors.to_le_bytes());
        rom[part_offset + 0x1A4..part_offset + 0x1A8]
            .copy_from_slice(&exefs_len_sectors.to_le_bytes());
        // No RomFS
        rom[part_offset + 0x1B0..part_offset + 0x1B4].copy_from_slice(&0u32.to_le_bytes());
        rom[part_offset + 0x1B4..part_offset + 0x1B8].copy_from_slice(&0u32.to_le_bytes());

        // --- ExeFS with encrypted (random) bytes ---
        let exefs_base = part_offset + exefs_off_sectors as usize * sector_size as usize;
        let encrypted_bytes: [u8; 8] = [0xFF, 0xA3, 0x7B, 0x92, 0xDE, 0x01, 0xC4, 0x88];
        rom[exefs_base..exefs_base + 8].copy_from_slice(&encrypted_bytes);

        // Save original ExeFS content for comparison
        let original_exefs = rom[exefs_base..exefs_base + sector_size as usize].to_vec();

        // Write to temp file
        let tmp_dir = std::path::PathBuf::from("test-fixtures");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let tmp_path = tmp_dir.join("temp_nocrypto_encrypted.3ds");
        {
            let mut f = std::fs::File::create(&tmp_path).expect("create temp file");
            f.write_all(&rom).expect("write temp file");
        }

        let messages: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let result = decrypt_rom(&tmp_path, &make_test_keydb(), |msg| {
            messages.lock().unwrap().push(msg.to_string());
        });

        let output = std::fs::read(&tmp_path).expect("read result");
        let _ = std::fs::remove_file(&tmp_path);

        assert!(result.is_ok(), "decrypt_rom failed: {:?}", result.err());

        // Verify the warning message was logged
        let msgs = messages.lock().unwrap();
        assert!(
            msgs.iter()
                .any(|m| m.contains("Flagged as decrypted but content is encrypted")),
            "Expected warning about encrypted content despite NoCrypto flag. Messages: {:?}",
            *msgs
        );

        // Verify decryption was applied (content changed from the encrypted input)
        let output_exefs = &output[exefs_base..exefs_base + sector_size as usize];
        assert_ne!(
            output_exefs,
            &original_exefs[..],
            "ExeFS content unchanged — decryption was not applied despite encrypted content"
        );

        // NoCrypto flag should be set after decryption completes
        assert_eq!(
            output[part_offset + 0x18F] & 0x04,
            0x04,
            "NoCrypto flag should be set after decryption"
        );
    }
}
