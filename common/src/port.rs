//! Port abstractions.
//!
//! The implementation for the standard port is inspired by the implementation
//! of the same thing in the x86_64 crate.

use core::arch::asm;
use core::marker::PhantomData;

mod macros {
    macro_rules! wait_for {
        ($cond:expr) => {
            while !$cond {
                core::hint::spin_loop()
            }
        };
    }

    pub(crate) use wait_for;
}
use macros::wait_for;

pub type PortRW<T> = Port<T, ReadWriteAccess>;
pub type PortR<T> = Port<T, ReadAccess>;
pub type PortW<T> = Port<T, WriteAccess>;

pub struct Port<T, A> {
    port: u16,
    ty: PhantomData<(T, A)>,
}

impl<T, A> Port<T, A> {
    pub const fn new(port: u16) -> Port<T, A> {
        Port { port, ty: PhantomData }
    }
}

impl<T: PortRead, A: PortReadAccess> Port<T, A> {
    pub unsafe fn read(&mut self) -> T {
        T::read_from_port(self.port)
    }
}

impl<T: PortWrite, A: PortWriteAccess> Port<T, A> {
    pub unsafe fn write(&mut self, val: T) {
        T::write_to_port(self.port, val);
    }
}

pub struct SerialPort {
    data: PortRW<u8>,
    int_en: PortW<u8>,
    fifo_ctrl: PortW<u8>,
    line_ctrl: PortW<u8>,
    modem_ctrl: PortW<u8>,
    line_sts: PortR<u8>,
}

impl SerialPort {
    pub const unsafe fn new(base: u16) -> Self {
        Self {
            data: Port::new(base),
            int_en: PortW::new(base + 1),
            fifo_ctrl: PortW::new(base + 2),
            line_ctrl: PortW::new(base + 3),
            modem_ctrl: PortW::new(base + 4),
            line_sts: PortR::new(base + 5),
        }
    }

    pub fn init(&mut self) {
        unsafe {
            // Disable interrupts
            self.int_en.write(0x00);

            // Enable DLAB
            self.line_ctrl.write(0x80);

            // Set maximum speed to 38400 bps by configuring DLL and DLM
            self.data.write(0x03);
            self.int_en.write(0x00);

            // Disable DLAB and set data word length to 8 bits
            self.line_ctrl.write(0x03);

            // Enable FIFO, clear TX/RX queues and
            // set interrupt watermark at 14 bytes
            self.fifo_ctrl.write(0xC7);

            // Mark data terminal ready, signal request to send
            // and enable auxilliary output #2 (used as interrupt line for CPU)
            self.modem_ctrl.write(0x0B);

            // Enable interrupts
            self.int_en.write(0x01);
        }
    }

    fn line_sts(&mut self) -> LineStatusFlags {
        unsafe { LineStatusFlags::from_bits_truncate(self.line_sts.read()) }
    }

    pub fn send(&mut self, data: u8) {
        unsafe {
            match data {
                8 | 0x7F => {
                    wait_for!(self.line_sts().contains(LineStatusFlags::OUTPUT_EMPTY));
                    self.data.write(8);
                    wait_for!(self.line_sts().contains(LineStatusFlags::OUTPUT_EMPTY));
                    self.data.write(b' ');
                    wait_for!(self.line_sts().contains(LineStatusFlags::OUTPUT_EMPTY));
                    self.data.write(8)
                }
                _ => {
                    wait_for!(self.line_sts().contains(LineStatusFlags::OUTPUT_EMPTY));
                    self.data.write(data);
                }
            }
        }
    }
}

impl core::fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}

mod sealed {
    pub trait Access {}
}

pub trait PortReadAccess: sealed::Access {}
pub trait PortWriteAccess: sealed::Access {}

#[derive(Debug)]
pub struct ReadWriteAccess(());
impl sealed::Access for ReadWriteAccess {}
impl PortReadAccess for ReadWriteAccess {}
impl PortWriteAccess for ReadWriteAccess {}

#[derive(Debug)]
pub struct ReadAccess(());
impl sealed::Access for ReadAccess {}
impl PortReadAccess for ReadAccess {}

#[derive(Debug)]
pub struct WriteAccess(());
impl sealed::Access for WriteAccess {}
impl PortWriteAccess for WriteAccess {}

pub trait PortRead {
    unsafe fn read_from_port(port: u16) -> Self;
}

pub trait PortWrite {
    unsafe fn write_to_port(port: u16, val: Self);
}

impl PortRead for u8 {
    unsafe fn read_from_port(port: u16) -> u8 {
        let value: u8;
        unsafe {
            asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
        }
        value
    }
}

impl PortRead for u16 {
    #[inline]
    unsafe fn read_from_port(port: u16) -> u16 {
        let value: u16;
        unsafe {
            asm!("inw ax, dx", out("ax") value, in("dx") port, options(nomem, nostack, preserves_flags));
        }
        value
    }
}

impl PortRead for u32 {
    #[inline]
    unsafe fn read_from_port(port: u16) -> u32 {
        let value: u32;
        unsafe {
            asm!("inl eax, dx", out("eax") value, in("dx") port, options(nomem, nostack, preserves_flags));
        }
        value
    }
}

impl PortWrite for u8 {
    #[inline]
    unsafe fn write_to_port(port: u16, value: u8) {
        unsafe {
            asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
        }
    }
}

impl PortWrite for u16 {
    #[inline]
    unsafe fn write_to_port(port: u16, value: u16) {
        unsafe {
            asm!("out dx, ax", in("dx") port, in("ax") value, options(nomem, nostack, preserves_flags));
        }
    }
}

impl PortWrite for u32 {
    #[inline]
    unsafe fn write_to_port(port: u16, value: u32) {
        unsafe {
            asm!("out dx, eax", in("dx") port, in("eax") value, options(nomem, nostack, preserves_flags));
        }
    }
}

bitflags::bitflags! {
    /// Line status flags
    struct LineStatusFlags: u8 {
        const INPUT_FULL = 1;
        // 1 to 4 unknown
        const OUTPUT_EMPTY = 1 << 5;
        // 6 and 7 unknown
    }
}
