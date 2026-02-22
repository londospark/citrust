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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use crate::keys::CryptoMethod;

    fn create_minimal_ncch_header() -> Vec<u8> {
        let mut data = vec![0u8; 0x200];
        
        // KeyY: first 16 bytes
        let key_y = 0x12345678_9ABCDEF0_FEDCBA98_76543210u128;
        data[0..16].copy_from_slice(&key_y.to_be_bytes());
        
        // TitleID at 0x108
        let title_id = 0x0004000000055D00u64;
        data[0x108..0x110].copy_from_slice(&title_id.to_le_bytes());
        
        // ExHeader length at 0x180
        data[0x180..0x184].copy_from_slice(&0x800u32.to_le_bytes());
        
        // Partition flags at 0x188
        data[0x188] = 0x00;  // flags[0]
        data[0x189] = 0x00;  // flags[1]
        data[0x18A] = 0x00;  // flags[2]
        data[0x18B] = 0x00;  // flags[3] - crypto method
        data[0x18C] = 0x00;  // flags[4]
        data[0x18D] = 0x00;  // flags[5]
        data[0x18E] = 0x00;  // flags[6]
        data[0x18F] = 0x00;  // flags[7] - NoCrypto/FixedKey
        
        // Plain region at 0x190
        data[0x190..0x194].copy_from_slice(&0x0u32.to_le_bytes());
        data[0x194..0x198].copy_from_slice(&0x0u32.to_le_bytes());
        
        // Logo region at 0x198
        data[0x198..0x19C].copy_from_slice(&0x0u32.to_le_bytes());
        data[0x19C..0x1A0].copy_from_slice(&0x0u32.to_le_bytes());
        
        // ExeFS at 0x1A0
        data[0x1A0..0x1A4].copy_from_slice(&0x1000u32.to_le_bytes());
        data[0x1A4..0x1A8].copy_from_slice(&0x800u32.to_le_bytes());
        
        // RomFS at 0x1B0
        data[0x1B0..0x1B4].copy_from_slice(&0x2000u32.to_le_bytes());
        data[0x1B4..0x1B8].copy_from_slice(&0x4000u32.to_le_bytes());
        
        data
    }

    #[test]
    fn test_parse_ncch_header() {
        let data = create_minimal_ncch_header();
        let mut cursor = Cursor::new(data);
        
        let header = NcchHeader::parse(&mut cursor, 0).unwrap();
        
        assert_eq!(header.key_y, 0x12345678_9ABCDEF0_FEDCBA98_76543210u128);
        assert_eq!(header.title_id, 0x0004000000055D00u64);
        assert_eq!(header.exheader_length, 0x800);
        assert_eq!(header.exefs_offset, 0x1000);
        assert_eq!(header.exefs_length, 0x800);
        assert_eq!(header.romfs_offset, 0x2000);
        assert_eq!(header.romfs_length, 0x4000);
    }

    #[test]
    fn test_crypto_method_detection() {
        let mut data = create_minimal_ncch_header();
        
        // Test each crypto method flag
        data[0x18B] = 0x00;
        let mut cursor = Cursor::new(data.clone());
        let header = NcchHeader::parse(&mut cursor, 0).unwrap();
        assert_eq!(header.crypto_method(), Some(CryptoMethod::Original));
        
        data[0x18B] = 0x01;
        let mut cursor = Cursor::new(data.clone());
        let header = NcchHeader::parse(&mut cursor, 0).unwrap();
        assert_eq!(header.crypto_method(), Some(CryptoMethod::Key7x));
        
        data[0x18B] = 0x0A;
        let mut cursor = Cursor::new(data.clone());
        let header = NcchHeader::parse(&mut cursor, 0).unwrap();
        assert_eq!(header.crypto_method(), Some(CryptoMethod::Key93));
        
        data[0x18B] = 0x0B;
        let mut cursor = Cursor::new(data.clone());
        let header = NcchHeader::parse(&mut cursor, 0).unwrap();
        assert_eq!(header.crypto_method(), Some(CryptoMethod::Key96));
    }

    #[test]
    fn test_no_crypto_flag() {
        let mut data = create_minimal_ncch_header();
        
        data[0x18F] = 0x04;  // Set NoCrypto bit
        let mut cursor = Cursor::new(data);
        let header = NcchHeader::parse(&mut cursor, 0).unwrap();
        
        assert!(header.is_no_crypto());
        assert!(!header.is_fixed_key());
    }

    #[test]
    fn test_fixed_key_flag() {
        let mut data = create_minimal_ncch_header();
        
        data[0x18F] = 0x01;  // Set FixedCryptoKey bit
        let mut cursor = Cursor::new(data);
        let header = NcchHeader::parse(&mut cursor, 0).unwrap();
        
        assert!(header.is_fixed_key());
        assert!(!header.is_no_crypto());
    }

    #[test]
    fn test_iv_construction() {
        let data = create_minimal_ncch_header();
        let mut cursor = Cursor::new(data);
        let header = NcchHeader::parse(&mut cursor, 0).unwrap();
        
        let title_id = header.title_id as u128;
        assert_eq!(header.plain_iv(), (title_id << 64) | 0x0100_0000_0000_0000u128);
        assert_eq!(header.exefs_iv(), (title_id << 64) | 0x0200_0000_0000_0000u128);
        assert_eq!(header.romfs_iv(), (title_id << 64) | 0x0300_0000_0000_0000u128);
    }
}
