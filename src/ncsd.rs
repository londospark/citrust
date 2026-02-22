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
