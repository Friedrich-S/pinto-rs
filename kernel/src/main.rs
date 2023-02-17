#![no_std]
#![no_main]
#![feature(format_args_nl)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]
#![feature(int_roundings)]

extern crate alloc;

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
    PageAllocator::init(u64::MAX);
    crate::mem::init_heap();

    loop {}
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
        println!("thread TODO panicked at '{s:?}'");
    } else {
        println!("thread TODO panicked");
    }

    loop {}
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
