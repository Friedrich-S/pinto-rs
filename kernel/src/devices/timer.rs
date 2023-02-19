use crate::threads::Interrupts;
use spinning_top::const_spinlock;
use spinning_top::Spinlock;
use x86_64::instructions::port::Port;
use x86_64::structures::idt::InterruptStackFrame;

/// The number of ticks since the OS has booted. We use a
/// `Spinlock<u64>` instead of an `AtomicU64` here for
/// compatibility reasons.
static TICKS: Spinlock<u64> = const_spinlock(0);

pub struct Timer;

impl Timer {
    pub const FREQ: u32 = 100;

    /// Sets up the timer to interrupt Self::FREQ times per second and registers
    /// the corresponding interrupt handler.
    pub fn init() {
        PIT::configure_channel(TimerChannel::Channel0, TimerMode::Mode2, Self::FREQ);
        Interrupts::register_handler(0x20, Self::on_interrupt, "8254 Timer");
    }

    /// Returns the number of ticks since the OS has booted.
    pub fn ticks() -> u64 {
        *TICKS.lock()
    }

    fn on_interrupt(frame: InterruptStackFrame) {
        *TICKS.lock() += 1;
        // ToDo: thread_tick
    }
}

static CONTROL_PORT: Spinlock<Port<u8>> = const_spinlock(Port::new(0x43));
static COUNTER_PORT_0: Spinlock<Port<u8>> = const_spinlock(Port::new(0x40 + 0));
static COUNTER_PORT_2: Spinlock<Port<u8>> = const_spinlock(Port::new(0x40 + 2));

/// An abstraction for the 8254 Programmable Interval Timer.
struct PIT;

impl PIT {
    /// PIT cycles per second.
    const HZ: u32 = 1193180;

    const fn counter_port(channel: TimerChannel) -> u16 {
        0x40 + match channel {
            TimerChannel::Channel0 => 0,
            TimerChannel::Channel2 => 2,
        }
    }

    fn configure_channel(channel: TimerChannel, mode: TimerMode, frequency: u32) {
        let count: u16 = if frequency < 19 {
            0
        } else if frequency > Self::HZ {
            2
        } else {
            ((Self::HZ + frequency / 2) / frequency) as u16
        };

        let old_level = Interrupts::disable();
        unsafe {
            CONTROL_PORT.lock().write((channel.value() << 6) | 0x30 | (mode.value() << 1));
            let mut counter_port = channel.port().lock();
            counter_port.write(count as u8);
            counter_port.write((count >> 8) as u8)
        }
        Interrupts::set_level(old_level);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TimerChannel {
    Channel0,
    Channel2,
}

impl TimerChannel {
    fn port(&self) -> &'static Spinlock<Port<u8>> {
        match self {
            TimerChannel::Channel0 => &COUNTER_PORT_0,
            TimerChannel::Channel2 => &COUNTER_PORT_2,
        }
    }

    fn value(&self) -> u8 {
        match self {
            TimerChannel::Channel0 => 0,
            TimerChannel::Channel2 => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TimerMode {
    Mode2,
    Mode3,
}

impl TimerMode {
    fn value(&self) -> u8 {
        match self {
            TimerMode::Mode2 => 2,
            TimerMode::Mode3 => 3,
        }
    }
}
