use std::cmp::min;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

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

/// Batch size for parallel decryption (64 MB)
const BATCH_SIZE: usize = 64 * 1024 * 1024;

/// Decrypt a contiguous region in parallel batches using a single key layer.
/// Reads `total_bytes` from `reader` at `file_offset`, decrypts with AES-CTR
/// using `key` and `base_iv`, and writes back to `writer`.
fn decrypt_parallel(
    reader: &mut File,
    writer: &mut File,
    file_offset: u64,
    total_bytes: usize,
    key: &Key128,
    base_iv: u128,
    chunk_size: usize,
    on_progress: &mut impl FnMut(&str),
    progress_label: &str,
) -> io::Result<()> {
    let total_mb = total_bytes / (1024 * 1024) + 1;
    let mut bytes_done: usize = 0;

    while bytes_done < total_bytes {
        let batch_len = min(BATCH_SIZE, total_bytes - bytes_done);
        let batch_file_offset = file_offset + bytes_done as u64;

        reader.seek(SeekFrom::Start(batch_file_offset))?;
        let mut batch = vec![0u8; batch_len];
        reader.read_exact(&mut batch)?;

        let batch_block_offset = bytes_done as u128 / 16;
        batch.par_chunks_mut(chunk_size).enumerate().for_each(|(i, chunk)| {
            let blocks_before = batch_block_offset + (i * chunk_size) as u128 / 16;
            let chunk_iv = base_iv + blocks_before;
            aes_ctr_decrypt(key, chunk_iv, chunk);
        });

        writer.seek(SeekFrom::Start(batch_file_offset))?;
        writer.write_all(&batch)?;

        bytes_done += batch_len;
        on_progress(&format!(
            "\r{progress_label}: {} / {} mb",
            bytes_done / (1024 * 1024),
            total_mb
        ));
    }
    Ok(())
}

/// Decrypt a contiguous region in parallel batches using two key layers
/// (for .code double-layer decryption: first key_bytes then key_2c_bytes).
fn decrypt_parallel_double(
    reader: &mut File,
    writer: &mut File,
    file_offset: u64,
    total_bytes: usize,
    key_first: &Key128,
    key_second: &Key128,
    base_iv: u128,
    chunk_size: usize,
    on_progress: &mut impl FnMut(&str),
    progress_label: &str,
) -> io::Result<()> {
    let total_mb = total_bytes / (1024 * 1024) + 1;
    let mut bytes_done: usize = 0;

    while bytes_done < total_bytes {
        let batch_len = min(BATCH_SIZE, total_bytes - bytes_done);
        let batch_file_offset = file_offset + bytes_done as u64;

        reader.seek(SeekFrom::Start(batch_file_offset))?;
        let mut batch = vec![0u8; batch_len];
        reader.read_exact(&mut batch)?;

        let batch_block_offset = bytes_done as u128 / 16;
        batch.par_chunks_mut(chunk_size).enumerate().for_each(|(i, chunk)| {
            let blocks_before = batch_block_offset + (i * chunk_size) as u128 / 16;
            let chunk_iv = base_iv + blocks_before;
            aes_ctr_decrypt(key_first, chunk_iv, chunk);
            aes_ctr_decrypt(key_second, chunk_iv, chunk);
        });

        writer.seek(SeekFrom::Start(batch_file_offset))?;
        writer.write_all(&batch)?;

        bytes_done += batch_len;
        on_progress(&format!(
            "\r{progress_label}: {} / {} mb...",
            bytes_done / (1024 * 1024),
            total_mb
        ));
    }
    Ok(())
}

pub fn decrypt_rom(path: &Path, mut on_progress: impl FnMut(&str)) -> Result<(), Error> {
    let mut reader = File::open(path)?;
    let mut writer = File::options().write(true).open(path)?;

    let ncsd = NcsdHeader::parse(&mut reader).map_err(|_| Error::NotNcsd)?;
    let sector_size = ncsd.sector_size;

    for p in 0..8u8 {
        let part = &ncsd.partitions[p as usize];
        if part.is_empty() {
            on_progress(&format!("Partition {p} Not found... Skipping..."));
            continue;
        }

        let part_offset = part.offset_bytes(sector_size);

        // Verify NCCH magic at partition+0x100
        reader.seek(SeekFrom::Start(part_offset + 0x100))?;
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"NCCH" {
            on_progress(&format!("Partition {p} Unable to read NCCH header"));
            continue;
        }

        let ncch = NcchHeader::parse(&mut reader, part_offset)?;

        if ncch.is_no_crypto() {
            on_progress(&format!("Partition {p}: Already Decrypted?..."));
            continue;
        }

        let key_y = ncch.key_y;
        let normal_key_2c: u128;
        let normal_key: u128;

        if ncch.is_fixed_key() {
            normal_key = 0;
            normal_key_2c = 0;
            if p == 0 {
                on_progress("Encryption Method: Zero Key");
            }
        } else {
            normal_key_2c =
                crate::crypto::derive_normal_key(KEY_X_2C, key_y, CONSTANT);

            let method = ncch.crypto_method().unwrap_or(CryptoMethod::Original);
            let key_x = keys::key_x_for_method(method);
            normal_key = crate::crypto::derive_normal_key(key_x, key_y, CONSTANT);

            if p == 0 {
                on_progress(&format!("Encryption Method: {method:?}"));
            }
        }

        let key_2c_bytes: Key128 = normal_key_2c.to_be_bytes();
        let key_bytes: Key128 = normal_key.to_be_bytes();

        // ======= DECRYPT EXHEADER =======
        if ncch.exheader_length > 0 {
            let exhdr_offset =
                (part.offset_sectors as u64 + 1) * sector_size as u64;
            reader.seek(SeekFrom::Start(exhdr_offset))?;
            let mut exhdr_data = vec![0u8; 0x800];
            reader.read_exact(&mut exhdr_data)?;
            aes_ctr_decrypt(&key_2c_bytes, ncch.plain_iv(), &mut exhdr_data);
            writer.seek(SeekFrom::Start(exhdr_offset))?;
            writer.write_all(&exhdr_data)?;
            on_progress(&format!(
                "Partition {p} ExeFS: Decrypting: ExHeader"
            ));
        }

        // ======= DECRYPT EXEFS =======
        if ncch.exefs_length > 0 {
            let exefs_base = (part.offset_sectors as u64
                + ncch.exefs_offset as u64)
                * sector_size as u64;
            let exefs_iv = ncch.exefs_iv();

            // Step 1: Decrypt ExeFS filename table (first sector)
            reader.seek(SeekFrom::Start(exefs_base))?;
            let mut table_data = vec![0u8; sector_size as usize];
            reader.read_exact(&mut table_data)?;
            aes_ctr_decrypt(&key_2c_bytes, exefs_iv, &mut table_data);
            writer.seek(SeekFrom::Start(exefs_base))?;
            writer.write_all(&table_data)?;
            on_progress(&format!(
                "Partition {p} ExeFS: Decrypting: ExeFS Filename Table"
            ));

            // Step 2: .code double-layer decrypt (only for 7.x/9.x keys)
            let method =
                ncch.crypto_method().unwrap_or(CryptoMethod::Original);
            if matches!(
                method,
                CryptoMethod::Key7x | CryptoMethod::Key93 | CryptoMethod::Key96
            ) {
                for j in 0..10u32 {
                    let slot_offset = (j * 0x10) as usize;
                    if slot_offset + 0x10 > table_data.len() {
                        break;
                    }
                    let filename = &table_data[slot_offset..slot_offset + 8];
                    if filename == b".code\x00\x00\x00" {
                        let code_file_off = u32::from_le_bytes(
                            table_data[slot_offset + 8..slot_offset + 12]
                                .try_into()
                                .unwrap(),
                        );
                        let code_file_len = u32::from_le_bytes(
                            table_data[slot_offset + 12..slot_offset + 16]
                                .try_into()
                                .unwrap(),
                        );

                        if code_file_len > 0 {
                            let ctr_offset = (code_file_off as u128
                                + sector_size as u128)
                                / 0x10;
                            let code_iv = exefs_iv + ctr_offset;

                            let code_abs_offset =
                                ((part.offset_sectors as u64
                                    + ncch.exefs_offset as u64
                                    + 1)
                                    * sector_size as u64)
                                    + code_file_off as u64;

                            let label = format!("Partition {p} ExeFS: Decrypting: .code");
                            decrypt_parallel_double(
                                &mut reader,
                                &mut writer,
                                code_abs_offset,
                                code_file_len as usize,
                                &key_bytes,
                                &key_2c_bytes,
                                code_iv,
                                1024 * 1024,
                                &mut on_progress,
                                &label,
                            )?;
                            on_progress(&format!(
                                "Partition {p} ExeFS: Decrypting: .code... Done!"
                            ));
                        }
                        break;
                    }
                }
            }

            // Step 3: Decrypt remaining ExeFS data (everything after first sector)
            let exefs_data_sectors = ncch.exefs_length.saturating_sub(1);
            if exefs_data_sectors > 0 {
                let exefs_data_size =
                    exefs_data_sectors as u64 * sector_size as u64;
                let ctr_offset = sector_size as u128 / 0x10;
                let data_iv = exefs_iv + ctr_offset;

                let exefs_data_offset = (part.offset_sectors as u64
                    + ncch.exefs_offset as u64
                    + 1)
                    * sector_size as u64;

                let label = format!("Partition {p} ExeFS: Decrypting");
                decrypt_parallel(
                    &mut reader,
                    &mut writer,
                    exefs_data_offset,
                    exefs_data_size as usize,
                    &key_2c_bytes,
                    data_iv,
                    1024 * 1024,
                    &mut on_progress,
                    &label,
                )?;
                on_progress(&format!(
                    "Partition {p} ExeFS: Decrypting: Done"
                ));
            }
        } else {
            on_progress(&format!(
                "Partition {p} ExeFS: No Data... Skipping..."
            ));
        }

        // ======= DECRYPT ROMFS =======
        if ncch.romfs_offset != 0 {
            let romfs_chunk_size: usize = 4 * 1024 * 1024;
            let romfs_total_bytes =
                ncch.romfs_length as u64 * sector_size as u64;

            let romfs_iv = ncch.romfs_iv();
            let romfs_offset = (part.offset_sectors as u64
                + ncch.romfs_offset as u64)
                * sector_size as u64;

            let label = format!("Partition {p} RomFS: Decrypting");
            decrypt_parallel(
                &mut reader,
                &mut writer,
                romfs_offset,
                romfs_total_bytes as usize,
                &key_bytes,
                romfs_iv,
                romfs_chunk_size,
                &mut on_progress,
                &label,
            )?;
            on_progress(&format!(
                "Partition {p} RomFS: Decrypting: Done"
            ));
        } else {
            on_progress(&format!(
                "Partition {p} RomFS: No Data... Skipping..."
            ));
        }

        // ======= PATCH FLAGS =======
        writer.seek(SeekFrom::Start(part_offset + 0x18B))?;
        writer.write_all(&[0x00])?;

        let mut flag = ncch.partition_flags[7];
        flag &= !0x01; // clear FixedCryptoKey
        flag &= !0x20; // clear CryptoUsingNewKeyY
        flag |= 0x04; // set NoCrypto
        writer.seek(SeekFrom::Start(part_offset + 0x18F))?;
        writer.write_all(&[flag])?;
    }

    writer.flush()?;
    on_progress("Done...");
    Ok(())
}






