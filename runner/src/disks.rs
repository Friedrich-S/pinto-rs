use std::collections::HashMap;
use std::fs::File;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

pub const LOADER_SIZE: usize = 314;
pub const SECTOR_SIZE: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Kernel,
    Filesys,
    Scratch,
    Swap,
}

impl Role {
    pub const ORDER: &[Self] = &[Self::Kernel, Self::Filesys, Self::Scratch, Self::Swap];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiskAlign {
    Bochs,
    Full,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiskFormat {
    Partitioned,
    Raw,
}

#[derive(Debug)]
pub struct DiskPart {
    pub path: String,
    pub offset: usize,
    pub bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiskGeometry {
    pub heads: usize,
    pub sectors_per_track: usize,
}

struct PartProperties {
    pub start: usize,
    pub num_sectors: usize,
}

/// The standard IDs for the used partition types.
/// The IDs are identical to those used in the original PintOS.
#[repr(u8)]
#[allow(dead_code)]
enum PartitionIds {
    Kernel = 0x20,
    Filesys = 0x21,
    Scratch = 0x22,
    Swap = 0x23,
}

pub fn assemble_disk(
    output: &mut impl Write,
    parts: &HashMap<Role, DiskPart>,
    loader: Option<&[u8; LOADER_SIZE]>,
    geometry: Option<&DiskGeometry>,
    align: DiskAlign,
    format: DiskFormat,
    args: &[&str],
) {
    let geometry = geometry.unwrap_or(&DiskGeometry {
        heads: 16,
        sectors_per_track: 63,
    });

    let (align, pad) = match align {
        DiskAlign::Bochs => (false, true),
        DiskAlign::Full => (true, false),
        DiskAlign::None => (false, false),
    };

    if format == DiskFormat::Raw && parts.len() != 1 {
        panic!("Must have exactly one partition for raw output");
    }

    let mut total_sectors = 0;
    if format == DiskFormat::Partitioned {
        total_sectors += match align {
            true => geometry.sectors_per_track,
            false => 1,
        };
    }

    let mut part_props = HashMap::new();
    for role in Role::ORDER {
        let Some(part) = parts.get(role) else {
            continue;
        };

        let start = total_sectors;
        let mut end = start + part.bytes.div_ceil(SECTOR_SIZE);
        if align {
            end = end.div_ceil(geometry.heads * geometry.sectors_per_track);
        }

        part_props.insert(
            *role,
            PartProperties {
                start,
                num_sectors: end - start,
            },
        );
        total_sectors = end;
    }

    // Write the disk
    if format == DiskFormat::Partitioned {
        let mut mbr = vec![0; LOADER_SIZE];
        if let Some(loader) = loader {
            mbr.copy_from_slice(loader);
        } else {
            mbr[0..2].copy_from_slice(&0xCD18u16.to_le_bytes());
        }

        mbr.extend_from_slice(&build_kernel_command_line(args));

        // Add partition table
        mbr.extend_from_slice(&build_partition_table(&part_props, geometry));

        // Add MBR signature
        mbr.extend_from_slice(&0xAA55u16.to_le_bytes());

        assert_eq!(mbr.len(), 512);

        // Write to the disk file
        output.write_all(&mbr).unwrap();
        if align {
            output.write_all(&vec![0; SECTOR_SIZE * geometry.sectors_per_track - 1]).unwrap();
        }
    }

    for role in Role::ORDER {
        let Some(part) = parts.get(role) else {
            continue;
        };
        let Some(props) = part_props.get(role) else {
            continue;
        };

        let mut source = File::open(&part.path).unwrap();
        source.seek(SeekFrom::Start(part.offset as u64)).unwrap();
        std::io::copy(&mut source, output).unwrap();
        output.write_all(&vec![0; props.num_sectors * SECTOR_SIZE - part.bytes]).unwrap();
    }

    if pad {
        let pad_sectors = total_sectors.next_multiple_of(geometry.heads * geometry.sectors_per_track);
        output.write_all(&vec![0; (pad_sectors - total_sectors) * SECTOR_SIZE]).unwrap();
    }
}

fn build_kernel_command_line(args: &[&str]) -> Vec<u8> {
    let mut s = String::new();
    for arg in args {
        s.push_str(arg);
        s.push('\0');
    }
    let mut raw = Vec::new();
    raw.extend_from_slice(&(args.len() as u32).to_le_bytes());
    raw.extend_from_slice(&s.as_bytes());

    const MAX_LEN: usize = 128 + 4;
    assert!(raw.len() <= MAX_LEN, "The command line exceeds 128 bytes");
    for _ in 0..(MAX_LEN - raw.len()) {
        raw.push(0);
    }

    raw
}

fn build_partition_table(parts: &HashMap<Role, PartProperties>, geometry: &DiskGeometry) -> Vec<u8> {
    let mut v = Vec::new();

    for role in Role::ORDER {
        let Some(part) = parts.get(role) else {
            continue;
        };

        v.push(match role {
            Role::Kernel => 0x80, // Bootable
            _ => 0x0,
        });

        v.extend_from_slice(&pack_chs(part.start, geometry));

        v.push(match role {
            Role::Kernel => PartitionIds::Kernel,
            Role::Filesys => PartitionIds::Filesys,
            Role::Scratch => PartitionIds::Scratch,
            Role::Swap => PartitionIds::Swap,
        } as u8);

        v.extend_from_slice(&pack_chs(part.start + part.num_sectors - 1, geometry));

        v.extend_from_slice(&(part.start as u32).to_le_bytes());
        v.extend_from_slice(&(part.num_sectors as u32).to_le_bytes());

        assert_eq!(v.len() % 16, 0);
    }

    // Ensure that the MBR is always 64 bytes in size
    assert!(v.len() <= 64);
    for _ in 0..(64 - v.len()) {
        v.push(0);
    }

    v
}

/// Packs a CHS block address into a 3-byte structure for the partition table.
///
/// This function is an exact port of the original in PintOS (`src/utils/Pintos.pm`)
fn pack_chs(lba: usize, geometry: &DiskGeometry) -> [u8; 3] {
    let (cyl, head, sect) = {
        let cyl = lba / (geometry.heads * geometry.sectors_per_track);
        let temp = lba % (geometry.heads * geometry.sectors_per_track);
        let head = temp / geometry.sectors_per_track;
        let sect = temp % geometry.sectors_per_track + 1;

        match cyl <= 1023 {
            true => (cyl, head, sect),
            false => (1023, 254, 63),
        }
    };

    [
        head as u8,
        (sect as u8) | (((cyl as u8) >> 2) & 0xc0),
        (cyl as u8) & 0xFF,
    ]
}
