use std::io::{self, Read, Seek, SeekFrom};

#[derive(Debug, Clone)]
pub struct NcsdHeader {
    pub sector_size: u32,
    pub partitions: [PartitionEntry; 8],
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PartitionEntry {
    pub offset_sectors: u32,
    pub length_sectors: u32,
}

impl PartitionEntry {
    pub fn offset_bytes(&self, sector_size: u32) -> u64 {
        self.offset_sectors as u64 * sector_size as u64
    }

    pub fn length_bytes(&self, sector_size: u32) -> u64 {
        self.length_sectors as u64 * sector_size as u64
    }

    pub fn is_empty(&self) -> bool {
        self.length_sectors == 0 || self.offset_sectors == 0
    }
}

impl NcsdHeader {
    pub fn parse<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        // Verify magic at 0x100
        reader.seek(SeekFrom::Start(0x100))?;
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"NCSD" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid NCSD magic",
            ));
        }

        // Read flags at 0x188 to determine sector size
        reader.seek(SeekFrom::Start(0x188))?;
        let mut flags = [0u8; 8];
        reader.read_exact(&mut flags)?;
        let sector_size = 0x200u32 * 2u32.pow(flags[6] as u32);

        // Read 8 partition entries at 0x120
        reader.seek(SeekFrom::Start(0x120))?;
        let mut partitions = [PartitionEntry::default(); 8];
        for partition in &mut partitions {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            partition.offset_sectors = u32::from_le_bytes(buf);
            reader.read_exact(&mut buf)?;
            partition.length_sectors = u32::from_le_bytes(buf);
        }

        Ok(NcsdHeader {
            sector_size,
            partitions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_valid_ncsd_header() {
        let mut data = vec![0u8; 512];

        // NCSD magic at 0x100
        data[0x100..0x104].copy_from_slice(b"NCSD");

        // Sector size flags at 0x188: flags[6] = 0 means sector_size = 0x200 * 2^0 = 0x200
        data[0x18E] = 0;

        // Partition table at 0x120: 8 entries, each 8 bytes (offset, length)
        // First partition: offset=0x1000 sectors, length=0x2000 sectors
        data[0x120..0x124].copy_from_slice(&0x1000u32.to_le_bytes());
        data[0x124..0x128].copy_from_slice(&0x2000u32.to_le_bytes());

        let mut cursor = Cursor::new(data);
        let header = NcsdHeader::parse(&mut cursor).unwrap();

        assert_eq!(header.sector_size, 0x200);
        assert_eq!(header.partitions[0].offset_sectors, 0x1000);
        assert_eq!(header.partitions[0].length_sectors, 0x2000);
    }

    #[test]
    fn test_reject_invalid_magic() {
        let mut data = vec![0u8; 512];
        data[0x100..0x104].copy_from_slice(b"XXXX");

        let mut cursor = Cursor::new(data);
        let result = NcsdHeader::parse(&mut cursor);

        assert!(result.is_err());
    }

    #[test]
    fn test_sector_size_calculation() {
        let mut data = vec![0u8; 512];
        data[0x100..0x104].copy_from_slice(b"NCSD");

        // flags[6] = 1 means sector_size = 0x200 * 2^1 = 0x400
        data[0x18E] = 1;

        let mut cursor = Cursor::new(data);
        let header = NcsdHeader::parse(&mut cursor).unwrap();

        assert_eq!(header.sector_size, 0x400);
    }

    #[test]
    fn test_partition_entry_helpers() {
        let entry = PartitionEntry {
            offset_sectors: 0x100,
            length_sectors: 0x200,
        };

        let sector_size = 0x200;
        assert_eq!(entry.offset_bytes(sector_size), 0x100 * 0x200);
        assert_eq!(entry.length_bytes(sector_size), 0x200 * 0x200);
        assert!(!entry.is_empty());

        let empty = PartitionEntry::default();
        assert!(empty.is_empty());
    }
}
