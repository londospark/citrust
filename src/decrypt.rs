use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

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

                            let data_len_m =
                                code_file_len as usize / (1024 * 1024);
                            let data_len_b =
                                code_file_len as usize % (1024 * 1024);

                            let mut current_offset = code_abs_offset;
                            let mut blocks_processed: u128 = 0;

                            for i in 0..data_len_m {
                                let chunk_size = 1024 * 1024;
                                reader
                                    .seek(SeekFrom::Start(current_offset))?;
                                let mut chunk = vec![0u8; chunk_size];
                                reader.read_exact(&mut chunk)?;

                                let chunk_iv = code_iv + blocks_processed;
                                aes_ctr_decrypt(
                                    &key_bytes, chunk_iv, &mut chunk,
                                );
                                aes_ctr_decrypt(
                                    &key_2c_bytes, chunk_iv, &mut chunk,
                                );

                                writer
                                    .seek(SeekFrom::Start(current_offset))?;
                                writer.write_all(&chunk)?;

                                blocks_processed += chunk_size as u128 / 16;
                                current_offset += chunk_size as u64;

                                on_progress(&format!(
                                    "\rPartition {p} ExeFS: Decrypting: .code... {} / {} mb...",
                                    i,
                                    data_len_m + 1
                                ));
                            }
                            if data_len_b > 0 {
                                reader
                                    .seek(SeekFrom::Start(current_offset))?;
                                let mut chunk = vec![0u8; data_len_b];
                                reader.read_exact(&mut chunk)?;

                                let chunk_iv = code_iv + blocks_processed;
                                aes_ctr_decrypt(
                                    &key_bytes, chunk_iv, &mut chunk,
                                );
                                aes_ctr_decrypt(
                                    &key_2c_bytes, chunk_iv, &mut chunk,
                                );

                                writer
                                    .seek(SeekFrom::Start(current_offset))?;
                                writer.write_all(&chunk)?;
                            }
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
                let exefs_data_m =
                    exefs_data_size as usize / (1024 * 1024);
                let exefs_data_b =
                    exefs_data_size as usize % (1024 * 1024);
                let ctr_offset = sector_size as u128 / 0x10;
                let data_iv = exefs_iv + ctr_offset;

                let exefs_data_offset = (part.offset_sectors as u64
                    + ncch.exefs_offset as u64
                    + 1)
                    * sector_size as u64;

                let mut current_offset = exefs_data_offset;
                let mut blocks_processed: u128 = 0;

                for i in 0..exefs_data_m {
                    let chunk_size = 1024 * 1024;
                    reader.seek(SeekFrom::Start(current_offset))?;
                    let mut chunk = vec![0u8; chunk_size];
                    reader.read_exact(&mut chunk)?;

                    aes_ctr_decrypt(
                        &key_2c_bytes,
                        data_iv + blocks_processed,
                        &mut chunk,
                    );

                    writer.seek(SeekFrom::Start(current_offset))?;
                    writer.write_all(&chunk)?;

                    blocks_processed += chunk_size as u128 / 16;
                    current_offset += chunk_size as u64;
                    on_progress(&format!(
                        "\rPartition {p} ExeFS: Decrypting: {} / {} mb",
                        i,
                        exefs_data_m + 1
                    ));
                }
                if exefs_data_b > 0 {
                    reader.seek(SeekFrom::Start(current_offset))?;
                    let mut chunk = vec![0u8; exefs_data_b];
                    reader.read_exact(&mut chunk)?;
                    aes_ctr_decrypt(
                        &key_2c_bytes,
                        data_iv + blocks_processed,
                        &mut chunk,
                    );
                    writer.seek(SeekFrom::Start(current_offset))?;
                    writer.write_all(&chunk)?;
                }
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
            let romfs_block_size: usize = 16 * 1024 * 1024;
            let romfs_total_bytes =
                ncch.romfs_length as u64 * sector_size as u64;
            let romfs_size_m =
                romfs_total_bytes as usize / romfs_block_size;
            let romfs_size_b =
                romfs_total_bytes as usize % romfs_block_size;
            let romfs_total_mb = romfs_total_bytes / (1024 * 1024) + 1;

            let romfs_iv = ncch.romfs_iv();
            let romfs_offset = (part.offset_sectors as u64
                + ncch.romfs_offset as u64)
                * sector_size as u64;

            let mut current_offset = romfs_offset;
            let mut blocks_processed: u128 = 0;

            for i in 0..romfs_size_m {
                reader.seek(SeekFrom::Start(current_offset))?;
                let mut chunk = vec![0u8; romfs_block_size];
                reader.read_exact(&mut chunk)?;
                aes_ctr_decrypt(
                    &key_bytes,
                    romfs_iv + blocks_processed,
                    &mut chunk,
                );
                writer.seek(SeekFrom::Start(current_offset))?;
                writer.write_all(&chunk)?;

                blocks_processed += romfs_block_size as u128 / 16;
                current_offset += romfs_block_size as u64;
                on_progress(&format!(
                    "\rPartition {p} RomFS: Decrypting: {} / {} mb",
                    (i as u64) * 16,
                    romfs_total_mb
                ));
            }
            if romfs_size_b > 0 {
                reader.seek(SeekFrom::Start(current_offset))?;
                let mut chunk = vec![0u8; romfs_size_b];
                reader.read_exact(&mut chunk)?;
                aes_ctr_decrypt(
                    &key_bytes,
                    romfs_iv + blocks_processed,
                    &mut chunk,
                );
                writer.seek(SeekFrom::Start(current_offset))?;
                writer.write_all(&chunk)?;
            }
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
