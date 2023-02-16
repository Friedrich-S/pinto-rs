#![no_std]
#![no_main]

use core::panic::PanicInfo;

fn kernel_main(boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    loop {}
}

bootloader_api::entry_point!(kernel_main);

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        // ToDo
    }
}
