use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::InterruptStackFrame;

static mut INTERRUPT_TABLE: InterruptDescriptorTable = InterruptDescriptorTable::new();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

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

    /// The main interrupt entry point.
    fn interrupt_entry(frame: InterruptStackFrame, index: u8, error_code: Option<u64>) {
        panic!("Received interrupt: index:{index}, error_code:{error_code:?}, frame={frame:#?}");
    }
}
