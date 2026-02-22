use std::fs::File;
use std::io::{self, Cursor};
use std::path::Path;

use memmap2::MmapMut;
use rayon::prelude::*;

use crate::crypto::aes_ctr_decrypt;
use crate::keys::{self, CryptoMethod, Key128, CONSTANT, KEY_X_2C};
use crate::ncch::NcchHeader;
use crate::ncsd::NcsdHeader;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a 3DS ROM (invalid NCSD magic)")]
    NotNcsd,
    #[error("partition {0}: invalid NCCH header")]
    InvalidNcch(u8),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// Chunk size for rayon parallel decryption
const CHUNK_SIZE: usize = 4 * 1024 * 1024;

/// Decrypt a slice in-place using parallel AES-CTR.
fn decrypt_slice(
    data: &mut [u8],
    key: &Key128,
    key_second: Option<&Key128>,
    base_iv: u128,
    chunk_size: usize,
) {
    data.par_chunks_mut(chunk_size).enumerate().for_each(|(i, chunk)| {
        let blocks_before = (i * chunk_size) as u128 / 16;
        let chunk_iv = base_iv + blocks_before;
        aes_ctr_decrypt(key, chunk_iv, chunk);
        if let Some(k2) = key_second {
            aes_ctr_decrypt(k2, chunk_iv, chunk);
        }
    });
}

pub fn decrypt_rom(path: &Path, mut on_progress: impl FnMut(&str)) -> Result<(), Error> {
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

        if ncch.is_no_crypto() {
            on_progress(&format!("Partition {p}: Already Decrypted?..."));
            continue;
        }

        let key_y = ncch.key_y;
        let (normal_key_2c, normal_key) = if ncch.is_fixed_key() {
            if p == 0 {
                on_progress("Encryption Method: Zero Key");
            }
            (0u128, 0u128)
        } else {
            let nk2c = crate::crypto::derive_normal_key(KEY_X_2C, key_y, CONSTANT);
            let method = ncch.crypto_method().unwrap_or(CryptoMethod::Original);
            let key_x = keys::key_x_for_method(method);
            let nk = crate::crypto::derive_normal_key(key_x, key_y, CONSTANT);
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
            let exefs_base = (part.offset_sectors as usize
                + ncch.exefs_offset as usize)
                * sector_size as usize;
            let exefs_iv = ncch.exefs_iv();
            let ss = sector_size as usize;

            // Step 1: Decrypt filename table (first sector, small)
            aes_ctr_decrypt(&key_2c, exefs_iv, &mut mmap[exefs_base..exefs_base + ss]);
            on_progress(&format!("Partition {p} ExeFS: Decrypting: ExeFS Filename Table"));

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
                        let code_file_off = u32::from_le_bytes(
                            mmap[slot + 8..slot + 12].try_into().unwrap(),
                        );
                        let code_file_len = u32::from_le_bytes(
                            mmap[slot + 12..slot + 16].try_into().unwrap(),
                        ) as usize;

                        if code_file_len > 0 {
                            let ctr_offset =
                                (code_file_off as u128 + ss as u128) / 0x10;
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
            let romfs_total =
                ncch.romfs_length as usize * sector_size as usize;
            let romfs_iv = ncch.romfs_iv();
            let romfs_start = (part.offset_sectors as usize
                + ncch.romfs_offset as usize)
                * sector_size as usize;

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






