#![no_std]
#![no_main]
#![feature(format_args_nl)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]
#![feature(int_roundings)]

extern crate alloc;

use crate::mem::MemoryInfo;
use crate::mem::PageAllocator;
use bootloader_api::config::Mapping;
use bootloader_api::BootloaderConfig;
use core::ops::Deref;
use core::panic::PanicInfo;

mod io;
mod mem;
mod threads;
mod utils;

fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    println!("Hello from test kernel!");
    println!("{boot_info:#?}");
    println!("{:#?}", boot_info.memory_regions.deref());

    // Initialize memory system
    MemoryInfo::init(boot_info);
    PageAllocator::init(u64::MAX);
    crate::mem::init_heap();

    shutdown_power_off();
}

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.frame_buffer.minimum_framebuffer_height = Some(720);
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};
bootloader_api::entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // ToDo: include thread name
    if let Some(s) = info.message() {
        if let Some(loc) = info.location() {
            println!("thread TODO panicked at '{s:?}', {}:{}", loc.file(), loc.line());
        } else {
            println!("thread TODO panicked at '{s:?}'");
        }
    } else {
        println!("thread TODO panicked");
    }

    shutdown_power_off();
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

pub fn shutdown_power_off() -> ! {
    use x86_64::instructions::{nop, port::Port};

    unsafe {
        let mut port = Port::new(0xB004);
        port.write(0x2000u16);

        // Special exit sequence for QEMU and Bochs
        let s = b"Shutdown";
        let mut port = Port::new(0x8900);
        for i in 0..s.len() {
            port.write(s[i]);
        }

        // Exit code for newer QEMU versions
        let mut port = Port::new(0x501);
        port.write(0x31u32);
    }

    loop {
        nop();
    }
}
