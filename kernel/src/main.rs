//! # General Notes
//! - it is safe to have heap allocations in the kernel because `SimpleKernelAlloc` is
//!   set as the global allocator for the entire crate.

#![no_std]
#![no_main]
#![feature(format_args_nl)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]
#![feature(int_roundings)]
#![feature(abi_x86_interrupt)]
#![feature(asm_const)]

extern crate alloc;

use crate::devices::Timer;
use crate::mem::MemoryInfo;
use crate::mem::PageAllocator;
use crate::threads::Interrupts;
use crate::threads::Thread;
use bootloader_api::config::Mapping;
use bootloader_api::BootloaderConfig;
use core::arch::asm;
use core::panic::PanicInfo;

mod devices;
mod io;
mod mem;
mod proc;
mod threads;
mod utils;

fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    println!("Init Thread");
    Thread::init();

    // Initialize memory system
    println!("Init MemoryInfo");
    MemoryInfo::init(boot_info);
    println!("Init PageAllocator");
    PageAllocator::init(u64::MAX);
    println!("Init heap");
    crate::mem::init_heap();
    // ToDo: paging_init();

    // Segmentation
    // ToDo: tss_init();
    // ToDo: gdt_init();

    // Initialize interrupt handlers
    println!("Init Interrupts");
    Interrupts::init();
    println!("Init Timer");
    Timer::init();
    // ToDo: kbd_init();
    // ToDo: input_init();
    // ToDo: exception_init();
    // ToDo: syscall_init();

    // Start thread scheduler and enable interrupts
    // ToDo: thread_start();
    // ToDo: serial_init_queue();
    // ToDo: timer_calibrate();

    // Give main thread a minimal PCB so it can launch the first process
    // ToDo: userprog_init();

    // Initialize file system
    // ToDo: ide_init();
    // ToDo: locate_block_devices();
    // ToDo: filesys_init(format_filesys);

    println!("Boot complete.");

    // Run actions specified on kernel command line.
    // ToDo: run_actions(argv);

    unsafe {
        x86_64::software_interrupt!(0x30);
        x86_64::software_interrupt!(0x30);
    }

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
