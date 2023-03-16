use core::arch::asm;

#[repr(C, packed)]
#[allow(dead_code)]
pub struct DiskAddressPacket {
    /// The size of the address packet (always 16).
    packet_size: u8,
    /// Always a zero byte.
    zero: u8,
    /// The number of sectors to transfer.
    num_sectors: u16,
    /// The offset within the segment for the transfer buffer
    offset: u16,
    /// The segment containing the transfer buffer
    segment: u16,
    /// The starting LBA
    start_lba: u64,
}

impl DiskAddressPacket {
    pub fn from_lba(start_lba: u64, num_sectors: u16, target_offset: u16, target_segment: u16) -> Self {
        Self {
            packet_size: 0x10,
            zero: 0,
            num_sectors,
            offset: target_offset,
            segment: target_segment,
            start_lba,
        }
    }

    pub unsafe fn perform_load(&self, disk_number: u16) {
        let self_addr = self as *const Self as u16;
        unsafe {
            asm!(
                "mov {1:x}, si", // backup the `si` register, whose contents are required by LLVM
                "mov si, {0:x}",
                "int 0x13",
                "jc read_failed",
                "mov si, {1:x}", // restore the `si` register to its prior state
                in(reg) self_addr,
                out(reg) _,
                in("ax") 0x4200u16, // Enable extended read
                in("dx") disk_number, // The number of the disk to read from
            );
        }
    }
}
