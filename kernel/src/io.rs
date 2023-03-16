use common::SerialPort;
use core::fmt::Write;

/// An implementation of the standard `println` macro that works in the kernel.
/// Prints to the serial port by default.
#[macro_export]
macro_rules! println {
    () => {
        $crate::println!("\n");
    };
    ($($arg:tt)*) => {{
        $crate::io::_print(format_args_nl!($($arg)*));
    }};
}

fn print_to<T>(args: core::fmt::Arguments<'_>, global_s: fn() -> T, label: &str)
where
    T: Write,
{
    if let Err(e) = global_s().write_fmt(args) {
        panic!("failed printing to {label}: {e}");
    }
}

#[doc(hidden)]
#[cfg(not(test))]
pub fn _print(args: core::fmt::Arguments<'_>) {
    print_to(args, serial, "serial");
}

/// Open a serial port for writing text to the output.
fn serial() -> SerialPort {
    let mut port = unsafe { SerialPort::new(0x3F8) };
    port.init();
    port
}
