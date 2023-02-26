//! Each thread structure is stored on the heap for safety reasons (see below).
//! At the very bottom of the kernel stack memory page is an 8-byte key that
//! can be used to retrieve a reference to the thread for this stack from the
//! global thread list.
//! This also fixes the problem with the original Pintos implementation where the
//! [`Thread`] struct could not grow too large. In this alternative approach it can
//! grow arbitrarily large.
//!
//! # Safety
//! The original C-like Pintos approach is to simply store the thread at the bottom
//! of the stack page as an inline value. All other code would simply hold a pointer
//! to that thread. The downside to this is that it is not portable at all into safe
//! Rust code. What is done instead is to use the Rust safety guarantees and store the
//! threads in a safe manner in a global map and just index into the map by replacing the
//! thread structure in the stack page by a key.

use crate::mem::VirtualAddress;
use crate::mem::PAGE_SIZE;
use crate::proc::Process;
use crate::utils::read_esp;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;
use lazy_static::lazy_static;
use slotmap::KeyData;
use slotmap::SlotMap;
use spinning_top::Spinlock;

lazy_static! {
    static ref ALL_THREADS: Spinlock<SlotMap<ThreadKey, Arc<Thread>>> = Spinlock::new(SlotMap::with_key());
}

#[derive(Debug)]
pub struct Thread {
    id: ThreadId,
    status: ThreadStatus,
    name: String,
    stack: usize,
    priority: ThreadPriority,
    /// A reference to the parent process if this is a user program.
    process: Option<Arc<Process>>,
    magic: u32,
}

impl Thread {
    const MAGIC: u32 = 0xcd6abf4b;

    fn new(name: impl ToString, priority: ThreadPriority) -> Self {
        assert!(ThreadPriority::MIN <= priority && priority <= ThreadPriority::MAX);

        let esp = read_esp();
        let page_bottom = VirtualAddress::new(esp as u64).page_round_down();

        Self {
            id: ThreadId::new(),
            status: ThreadStatus::Blocked,
            name: name.to_string(),
            stack: (page_bottom.raw() + PAGE_SIZE) as usize,
            priority,
            process: None,
            magic: Self::MAGIC,
        }
    }

    /// Transforms the code that is currently running into a thread.
    pub fn init() {
        //let mut thread = Self::new("main", ThreadPriority::DEFAULT);
        //thread.status = ThreadStatus::Running;

        //let key = ALL_THREADS.lock().insert(Arc::new(thread));
        //Self::set_current(key);
    }

    /// Returns the current running thread.
    pub fn current() -> Option<Arc<Thread>> {
        let esp = read_esp();
        let page_bottom = VirtualAddress::new(esp as u64).page_round_down();
        // SAFETY: it is assumed that the kernel stack pointer is always valid to
        // read from. If this was not the case, this code would not even run properly.
        let raw_key = unsafe { *(page_bottom.raw() as *const u64) };
        let key = ThreadKey::from_raw(raw_key);

        ALL_THREADS.lock().get(key).map(|v| Arc::clone(v))
    }

    fn set_current(key: ThreadKey) {
        let esp = read_esp();
        let page_bottom = VirtualAddress::new(esp as u64).page_round_down();
        // SAFETY: it is assumed that the kernel stack pointer is always valid to
        // read from. If this was not the case, this code would not even run properly.
        unsafe {
            *(page_bottom.raw() as *mut u64) = key.to_raw();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThreadStatus {
    Running,
    Ready,
    Blocked,
    Dying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThreadPriority(u32);

impl ThreadPriority {
    pub const MIN: ThreadPriority = ThreadPriority(0);
    pub const DEFAULT: ThreadPriority = ThreadPriority(31);
    pub const MAX: ThreadPriority = ThreadPriority(63);
}

impl core::ops::Add<ThreadPriority> for ThreadPriority {
    type Output = Self;

    fn add(self, rhs: ThreadPriority) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl core::ops::Add<u32> for ThreadPriority {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl core::ops::Sub<ThreadPriority> for ThreadPriority {
    type Output = Self;

    fn sub(self, rhs: ThreadPriority) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl core::ops::Sub<u32> for ThreadPriority {
    type Output = Self;

    fn sub(self, rhs: u32) -> Self::Output {
        Self(self.0 - rhs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThreadId(u32);

impl ThreadId {
    /// Allocates a new [`ThreadId`] by reading and incrementing
    /// the global ID counter.
    fn new() -> Self {
        static NEXT_ID: AtomicU32 = AtomicU32::new(1);

        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

slotmap::new_key_type! {
    pub struct ThreadKey;
}

impl ThreadKey {
    pub fn from_raw(val: u64) -> Self {
        Self(KeyData::from_ffi(val))
    }

    pub fn to_raw(&self) -> u64 {
        self.0.as_ffi()
    }
}
