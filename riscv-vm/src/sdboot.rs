//! SD Card Boot Support
//!
//! Parses MBR partition table and FAT32 filesystem to load kernel from SD card.
//! Used by all VM platforms (native, Node.js, browser).

/// MBR partition entry (16 bytes)
#[derive(Debug, Clone, Copy, Default)]
pub struct PartitionEntry {
    pub bootable: bool,
    pub partition_type: u8,
    pub start_lba: u32,
    pub sector_count: u32,
}

/// Parse MBR partition table from first 512 bytes
pub fn parse_mbr(sector0: &[u8]) -> Result<[PartitionEntry; 4], &'static str> {
    if sector0.len() < 512 {
        return Err("Sector 0 too small");
    }
    
    // Check MBR signature
    if sector0[510] != 0x55 || sector0[511] != 0xAA {
        return Err("Invalid MBR signature");
    }
    
    let mut partitions = [PartitionEntry::default(); 4];
    
    for i in 0..4 {
        let offset = 446 + i * 16;
        let entry = &sector0[offset..offset + 16];
        
        partitions[i] = PartitionEntry {
            bootable: entry[0] == 0x80,
            partition_type: entry[4],
            start_lba: u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]),
            sector_count: u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]),
        };
    }
    
    Ok(partitions)
}

/// Find boot partition (FAT32 or FAT16)
pub fn find_boot_partition(partitions: &[PartitionEntry; 4]) -> Option<&PartitionEntry> {
    for part in partitions {
        // FAT32 types: 0x0B (CHS), 0x0C (LBA)
        // FAT16 types: 0x06 (CHS), 0x0E (LBA)
        if matches!(part.partition_type, 0x0B | 0x0C | 0x06 | 0x0E) && part.sector_count > 0 {
            return Some(part);
        }
    }
    None
}

/// Minimal FAT32 boot sector parsing
#[derive(Debug, Clone)]
pub struct Fat32BootSector {
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    pub sectors_per_fat: u32,
    pub root_cluster: u32,
}

impl Fat32BootSector {
    pub fn parse(boot_sector: &[u8]) -> Result<Self, &'static str> {
        if boot_sector.len() < 512 {
            return Err("Boot sector too small");
        }
        
        // Check for FAT signature
        if boot_sector[510] != 0x55 || boot_sector[511] != 0xAA {
            return Err("Invalid FAT boot sector signature");
        }
        
        let bytes_per_sector = u16::from_le_bytes([boot_sector[11], boot_sector[12]]);
        let sectors_per_cluster = boot_sector[13];
        let reserved_sectors = u16::from_le_bytes([boot_sector[14], boot_sector[15]]);
        let num_fats = boot_sector[16];
        
        // FAT32: sectors per FAT is at offset 36 (4 bytes)
        let sectors_per_fat = u32::from_le_bytes([
            boot_sector[36], boot_sector[37], boot_sector[38], boot_sector[39]
        ]);
        
        // Root cluster at offset 44
        let root_cluster = u32::from_le_bytes([
            boot_sector[44], boot_sector[45], boot_sector[46], boot_sector[47]
        ]);
        
        Ok(Self {
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            num_fats,
            sectors_per_fat,
            root_cluster,
        })
    }
    
    /// Get the first sector of the data region
    pub fn data_start_sector(&self) -> u32 {
        self.reserved_sectors as u32 + (self.num_fats as u32 * self.sectors_per_fat)
    }
    
    /// Convert cluster number to sector number
    pub fn cluster_to_sector(&self, cluster: u32) -> u32 {
        self.data_start_sector() + (cluster - 2) * self.sectors_per_cluster as u32
    }
}

/// FAT32 directory entry (32 bytes)
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: [u8; 11],
    pub attr: u8,
    pub cluster_high: u16,
    pub cluster_low: u16,
    pub file_size: u32,
}

impl DirEntry {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 32 {
            return None;
        }
        
        // Skip deleted entries and end marker
        if data[0] == 0x00 || data[0] == 0xE5 {
            return None;
        }
        
        // Skip long name entries
        if data[11] == 0x0F {
            return None;
        }
        
        let mut name = [0u8; 11];
        name.copy_from_slice(&data[0..11]);
        
        Some(Self {
            name,
            attr: data[11],
            cluster_high: u16::from_le_bytes([data[20], data[21]]),
            cluster_low: u16::from_le_bytes([data[26], data[27]]),
            file_size: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
        })
    }
    
    pub fn cluster(&self) -> u32 {
        ((self.cluster_high as u32) << 16) | (self.cluster_low as u32)
    }
    
    /// Check if this entry matches a short filename (8.3 format)
    pub fn matches_name(&self, name: &str) -> bool {
        let name_upper = name.to_uppercase();
        let parts: Vec<&str> = name_upper.split('.').collect();
        
        let (basename, ext) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            (parts[0], "")
        };
        
        // Build 8.3 format name
        let mut fat_name = [b' '; 11];
        for (i, &b) in basename.as_bytes().iter().take(8).enumerate() {
            fat_name[i] = b;
        }
        for (i, &b) in ext.as_bytes().iter().take(3).enumerate() {
            fat_name[8 + i] = b;
        }
        
        self.name == fat_name
    }
    
    pub fn is_directory(&self) -> bool {
        (self.attr & 0x10) != 0
    }
}

/// Load a file from FAT32 filesystem
///
/// Returns the file contents if found.
pub fn load_file_from_fat32(
    disk: &[u8],
    partition_start_sector: u32,
    filename: &str,
) -> Result<Vec<u8>, &'static str> {
    // Read FAT32 boot sector
    let boot_offset = (partition_start_sector as usize) * 512;
    if boot_offset + 512 > disk.len() {
        return Err("Partition beyond disk");
    }
    
    let fat32 = Fat32BootSector::parse(&disk[boot_offset..boot_offset + 512])?;
    
    // Read root directory
    let root_sector = fat32.cluster_to_sector(fat32.root_cluster);
    let root_offset = boot_offset + (root_sector as usize) * 512;
    
    // Search directory entries (read one cluster)
    let cluster_size = fat32.sectors_per_cluster as usize * 512;
    if root_offset + cluster_size > disk.len() {
        return Err("Root directory beyond disk");
    }
    
    let dir_data = &disk[root_offset..root_offset + cluster_size];
    
    for i in (0..cluster_size).step_by(32) {
        if let Some(entry) = DirEntry::parse(&dir_data[i..]) {
            if entry.matches_name(filename) && !entry.is_directory() {
                // Found the file! Read its contents
                let file_cluster = entry.cluster();
                let file_sector = fat32.cluster_to_sector(file_cluster);
                let file_offset = boot_offset + (file_sector as usize) * 512;
                let file_size = entry.file_size as usize;
                
                if file_offset + file_size > disk.len() {
                    return Err("File data beyond disk");
                }
                
                return Ok(disk[file_offset..file_offset + file_size].to_vec());
            }
        }
    }
    
    Err("File not found")
}

/// Boot information extracted from SD card
#[derive(Debug)]
pub struct SdBootInfo {
    pub kernel_data: Vec<u8>,
    pub kernel_load_addr: u64,
    pub fs_partition_start: u32,
    pub fs_partition_sectors: u32,
}

/// Parse SD card image and extract boot information
pub fn parse_sdcard(disk: &[u8]) -> Result<SdBootInfo, &'static str> {
    if disk.len() < 512 {
        return Err("Disk image too small");
    }
    
    // Parse MBR
    let partitions = parse_mbr(&disk[0..512])?;
    
    // Find boot partition
    let boot_part = find_boot_partition(&partitions)
        .ok_or("No FAT32 boot partition found")?;
    
    // Find filesystem partition (type 0x83 Linux or raw)
    let fs_part = partitions.iter()
        .find(|p| p.partition_type != 0 && p.start_lba != boot_part.start_lba)
        .ok_or("No filesystem partition found")?;
    
    // Load kernel from boot partition
    let kernel_data = load_file_from_fat32(disk, boot_part.start_lba, "kernel.bin")
        .or_else(|_| load_file_from_fat32(disk, boot_part.start_lba, "KERNEL.BIN"))?;
    
    Ok(SdBootInfo {
        kernel_data,
        kernel_load_addr: 0x8020_0000,  // After OpenSBI reservation
        fs_partition_start: fs_part.start_lba,
        fs_partition_sectors: fs_part.sector_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dir_entry_name_match() {
        let mut entry = DirEntry {
            name: *b"KERNEL  BIN",
            attr: 0,
            cluster_high: 0,
            cluster_low: 2,
            file_size: 1024,
        };
        
        assert!(entry.matches_name("kernel.bin"));
        assert!(entry.matches_name("KERNEL.BIN"));
        assert!(!entry.matches_name("other.bin"));
    }
}
