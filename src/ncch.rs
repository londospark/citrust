use std::io::{self, Read, Seek, SeekFrom};

use crate::keys::CryptoMethod;

#[derive(Debug, Clone)]
pub struct NcchHeader {
    pub key_y: u128,
    pub title_id: u64,
    pub partition_flags: [u8; 8],
    pub exheader_length: u32,
    pub plain_offset: u32,
    pub plain_length: u32,
    pub logo_offset: u32,
    pub logo_length: u32,
    pub exefs_offset: u32,
    pub exefs_length: u32,
    pub romfs_offset: u32,
    pub romfs_length: u32,
}

impl NcchHeader {
    /// Parse NCCH header at the given partition offset
    pub fn parse<R: Read + Seek>(reader: &mut R, partition_offset: u64) -> io::Result<Self> {
        // KeyY: first 16 bytes of partition (from RSA signature)
        reader.seek(SeekFrom::Start(partition_offset))?;
        let mut key_y_bytes = [0u8; 16];
        reader.read_exact(&mut key_y_bytes)?;
        let key_y = u128::from_be_bytes(key_y_bytes);

        // TitleID at partition+0x108, little-endian u64
        reader.seek(SeekFrom::Start(partition_offset + 0x108))?;
        let mut tid_bytes = [0u8; 8];
        reader.read_exact(&mut tid_bytes)?;
        let title_id = u64::from_le_bytes(tid_bytes);

        // ExHeader length at partition+0x180, LE u32
        reader.seek(SeekFrom::Start(partition_offset + 0x180))?;
        let mut buf4 = [0u8; 4];
        reader.read_exact(&mut buf4)?;
        let exheader_length = u32::from_le_bytes(buf4);

        // Partition flags at partition+0x188
        reader.seek(SeekFrom::Start(partition_offset + 0x188))?;
        let mut partition_flags = [0u8; 8];
        reader.read_exact(&mut partition_flags)?;

        // Plain region at partition+0x190
        reader.seek(SeekFrom::Start(partition_offset + 0x190))?;
        reader.read_exact(&mut buf4)?;
        let plain_offset = u32::from_le_bytes(buf4);
        reader.read_exact(&mut buf4)?;
        let plain_length = u32::from_le_bytes(buf4);

        // Logo region at partition+0x198
        reader.read_exact(&mut buf4)?;
        let logo_offset = u32::from_le_bytes(buf4);
        reader.read_exact(&mut buf4)?;
        let logo_length = u32::from_le_bytes(buf4);

        // ExeFS at partition+0x1A0
        reader.read_exact(&mut buf4)?;
        let exefs_offset = u32::from_le_bytes(buf4);
        reader.read_exact(&mut buf4)?;
        let exefs_length = u32::from_le_bytes(buf4);

        // Skip 8 bytes (ExeFS hash region size + reserved)
        reader.seek(SeekFrom::Start(partition_offset + 0x1B0))?;

        // RomFS at partition+0x1B0
        reader.read_exact(&mut buf4)?;
        let romfs_offset = u32::from_le_bytes(buf4);
        reader.read_exact(&mut buf4)?;
        let romfs_length = u32::from_le_bytes(buf4);

        Ok(NcchHeader {
            key_y,
            title_id,
            partition_flags,
            exheader_length,
            plain_offset,
            plain_length,
            logo_offset,
            logo_length,
            exefs_offset,
            exefs_length,
            romfs_offset,
            romfs_length,
        })
    }

    /// Get crypto method from flags[3]
    pub fn crypto_method(&self) -> Option<CryptoMethod> {
        CryptoMethod::from_flag(self.partition_flags[3])
    }

    /// Check if NoCrypto bit is set (flags[7] & 0x04)
    pub fn is_no_crypto(&self) -> bool {
        self.partition_flags[7] & 0x04 != 0
    }

    /// Check if FixedCryptoKey (zero-key) bit is set (flags[7] & 0x01)
    pub fn is_fixed_key(&self) -> bool {
        self.partition_flags[7] & 0x01 != 0
    }

    /// Plain region IV
    pub fn plain_iv(&self) -> u128 {
        ((self.title_id as u128) << 64) | 0x0100_0000_0000_0000u128
    }

    /// ExeFS IV
    pub fn exefs_iv(&self) -> u128 {
        ((self.title_id as u128) << 64) | 0x0200_0000_0000_0000u128
    }

    /// RomFS IV
    pub fn romfs_iv(&self) -> u128 {
        ((self.title_id as u128) << 64) | 0x0300_0000_0000_0000u128
    }
}
