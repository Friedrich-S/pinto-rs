use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use spinning_top::const_spinlock;
use spinning_top::Spinlock;
use x86_64::instructions::interrupts;
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;

pub type InterruptHandler = fn(InterruptStackFrame);

static mut INTERRUPT_TABLE: InterruptDescriptorTable = InterruptDescriptorTable::new();
static INITIALIZED: AtomicBool = AtomicBool::new(false);
static HANDLERS: Spinlock<[Option<InterruptHandler>; 256]> = const_spinlock([None; 256]);

pub struct Interrupts;

impl Interrupts {
    /// Initializes the interrupt state.
    ///
    /// # Safety
    /// To make the internal operations safe, this function may only be called ONCE.
    ///
    /// # Panics
    /// Detects if the function was called multiple times and panics if that is the casae.
    pub fn init() {
        // Already initialized. This violates the safety assumptions and so we panic!
        if INITIALIZED.swap(true, Ordering::Relaxed) {
            panic!("Interrupt state already initialized!");
        }

        // Register a general interrupt handler
        fn handler(frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
            Interrupts::interrupt_entry(frame, index, error_code)
        }
        // SAFETY: the first check in this function ensures that this is only run
        // once and only one a single thread. Therefore, no data races can occur.
        unsafe {
            x86_64::set_general_handler!(&mut INTERRUPT_TABLE, handler);
        }

        // SAFETY: the first check in this function ensures that this is only run
        // once and only one a single thread. Therefore, no data races can occur.
        unsafe {
            INTERRUPT_TABLE.load();
        }
    }

    /// Disable interrupts and return the previous level.
    pub fn disable() -> bool {
        let prev = interrupts::are_enabled();
        interrupts::disable();
        prev
    }

    /// Enable interrupts nd return the previous level.
    pub fn enable() -> bool {
        let prev = interrupts::are_enabled();
        interrupts::enable();
        prev
    }

    pub fn set_level(enabled: bool) {
        match enabled {
            true => Self::enable(),
            false => Self::disable(),
        };
    }

    pub fn register_handler(index: u8, func: InterruptHandler, name: &'static str) {
        let mut handlers = HANDLERS.lock();
        handlers[index as usize] = Some(func);
    }

    /// The main interrupt entry point.
    ///
    /// Note: interrupts are disabled by default by the CPU upon entering an
    /// interrupt handler, so it does not need to be done manually.
    fn interrupt_entry(frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
        crate::println!("Received interrupt: index:{index}, error_code:{error_code:?}, frame={frame:#?}");

        // Invoke a registered interrupt handler if present
        if let Some(handler) = HANDLERS.lock()[index as usize] {
            handler(frame);
        } else {
            // ToDo: fully implement
        }
    }
}
