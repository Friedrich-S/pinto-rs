#![no_std]
#![no_main]

use crate::address_packet::DiskAddressPacket;
use byteorder::ByteOrder;
use byteorder::LittleEndian;
use core::arch::asm;
use core::arch::global_asm;
use core::panic::PanicInfo;

mod address_packet;

global_asm!(include_str!("boot.s"), options(att_syntax));

extern "C" {
    /// Allocated by the linker script such that it is located at the start of the partition table.
    static _partition_table: u8;
}

static mut CONSOLE: Console = Console::new();
const MBR_ENTRY_SIZE: usize = 16;

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

unsafe fn partition_table_raw() -> *const u8 {
    unsafe { &_partition_table }
}

#[no_mangle]
pub extern "C" fn main(disk_num: u16) -> ! {
    // Initialize the console first
    unsafe {
        CONSOLE.init();
    }

    let partition_table_start = unsafe { partition_table_raw() as *const [u8; MBR_ENTRY_SIZE] };

    unsafe {
        CONSOLE.print("PiLo\n");
    }

    const MAX_PARTITIONS: usize = 4;
    let raw = unsafe { core::slice::from_raw_parts(partition_table_start, MAX_PARTITIONS) };
    // Loop over every partition and check whether we can book the kernel from it.
    for i in 0..MAX_PARTITIONS {
        check_entry(disk_num, &raw[i]);
    }

    fail(b'L');
}

fn check_entry(disk_num: u16, entry: &[u8; MBR_ENTRY_SIZE]) {
    // Whether this partition is bootable
    let bootable = entry[0] == 0x80;
    // The ID assigned to this partition by the disk creation code
    let id = entry[3];

    // If the partition is bootable and contains the kernel, we have found the target.
    if bootable && id == (PartitionIds::Kernel as u8) {
        // --- Load and launch the kernel ---

        // The offset of the first sector
        let offset = LittleEndian::read_u32(&entry[8..]);
        // The number of sectors in the partition
        let mut num_sectors = LittleEndian::read_u32(&entry[12..]);
        // Limit the number of sectors to 1024 (like in the original PintOS)
        if num_sectors > 1024 {
            num_sectors = 1024;
        }

        // The start of the buffer where the sectors will be stored
        const BUF_START: u32 = 0x20000;
        let mut buf_addr = (BUF_START >> 4) as u16;
        let mut start_lba = offset as u64;
        while num_sectors != 0 {
            // Read up to 32 sectors at once
            let sectors = u32::min(num_sectors, 32) as u16;
            let dap = DiskAddressPacket::from_lba(start_lba, 1, 0, buf_addr);
            unsafe {
                dap.perform_load(disk_num);
            }

            start_lba += sectors as u64;
            num_sectors -= sectors as u32;
            buf_addr += 0x20;
        }

        // Load the ELF entry from the loaded sectors
        let buf = BUF_START as *const u8;
        let entry_ptr = unsafe { *(buf.offset(0x18) as *const u32) };
        let entry: unsafe extern "C" fn() = unsafe { core::mem::transmute(entry_ptr as *const ()) };
        // Call the entry and start the kernel
        unsafe { entry() };

        // Fail if the entry returns.
        fail(b'R');
    }
}

struct Console {
    /// Whether it is possible to write to the serial output. Set to `false` on a serial error.
    can_write: bool,
}

impl Console {
    pub const fn new() -> Self {
        Self { can_write: false }
    }

    pub unsafe fn init(&mut self) {
        self.can_write = true;

        // Initialize a serial port for the console
        unsafe {
            // Select serial port 0 at 9600 bps
            asm!(
                "sub %dx, %dx
                mov $0xe3, %al
                int $0x14
            ",
                out("dx") _, out("al") _,
                options(att_syntax, nomem, nostack)
            );
        }
    }

    /// Prints the given string to the BIOS serial port.
    pub unsafe fn print(&mut self, msg: &str) {
        if !self.can_write {
            return;
        }

        for &c in msg.as_bytes() {
            self.print_char(c);
        }
    }

    /// Prints the given ASCII character to the BIOS serial port.
    pub unsafe fn print_char(&mut self, c: u8) {
        let res: u8;

        unsafe {
            // Select AH=1 serial output mode, select serial port 0
            // Load character into "al" register and send it using the interrupt.
            asm!(
                "mov $0x01, %ah
                sub %dx, %dx
                int $0x14
            ", 
                out("ah") res, out("dx") _, in("al") c,
                options(att_syntax, nomem)
            );
        }

        // res now contains the result of the write operation (value in %ah).
        if res == 0x80 {
            // Serial error occured
            self.can_write = false;
            return;
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    fail(b'P')
}

#[cold]
#[inline(never)]
#[no_mangle]
pub extern "C" fn fail(code: u8) -> ! {
    unsafe {
        CONSOLE.print_char(b'!');
        CONSOLE.print_char(code);
    }

    unsafe {
        // Notify the BIOS that the boot has failed
        asm!("int $0x18", options(att_syntax, nomem, nostack));
    }

    loop {
        hlt()
    }
}

#[cold]
#[inline(never)]
#[no_mangle]
pub extern "C" fn read_failed() -> ! {
    //unsafe {
    //    CONSOLE.print("Bad read\n");
    //}

    fail(b'z');
}

fn hlt() {
    unsafe {
        asm!("hlt");
    }
}
