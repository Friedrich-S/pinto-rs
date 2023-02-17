use crate::mem::PAGE_SIZE;
use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::ptr::NonNull;
use spinning_top::const_spinlock;
use spinning_top::Spinlock;
use tap::TapOptional;

const NUM_DESCS: usize = (PAGE_SIZE / 32).ilog2() as usize;
const DEFAULT_DESC: Spinlock<Descriptor> = const_spinlock(Descriptor::new());

#[global_allocator]
static ALLOCATOR: SimpleAlloc = SimpleAlloc {
    descs: [DEFAULT_DESC; NUM_DESCS],
};

pub fn init_heap() {
    ALLOCATOR.init();
}

/// A simple malloc implementation similar to the one used in the original Pintos.
pub struct SimpleAlloc {
    descs: [Spinlock<Descriptor>; NUM_DESCS],
}

impl SimpleAlloc {
    fn init(&self) {
        let mut block_size = 16;
        for desc in &self.descs {
            let mut desc = desc.lock();
            desc.block_size = block_size;
            desc.blocks_per_arena = ((PAGE_SIZE as usize) - core::mem::size_of::<Arena>()) / block_size;

            block_size *= 2;
        }
    }
}

unsafe impl GlobalAlloc for SimpleAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        todo!()
    }
}

#[derive(Debug)]
struct Descriptor {
    block_size: usize,
    blocks_per_arena: usize,
    free_list: BlockList,
}

impl Descriptor {
    const fn new() -> Self {
        Self {
            block_size: 0,
            blocks_per_arena: 0,
            free_list: BlockList::new(),
        }
    }
}

// Note: unbased assumption for now
// ToDo: add good reason for this
unsafe impl Send for Descriptor {}

#[derive(Debug)]
struct Arena {
    magic: u32,
    desc: Option<&'static Spinlock<Descriptor>>,
    num_free: usize,
}

impl Arena {
    pub const MAGIC: u32 = 0x9a548eed;
}

#[derive(Debug)]
struct BlockList {
    head: Option<NonNull<Block>>,
    tail: Option<NonNull<Block>>,
}

impl BlockList {
    pub const fn new() -> Self {
        Self { head: None, tail: None }
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub unsafe fn push_back(&mut self, block: NonNull<Block>) {
        if self.head.is_none() && self.tail.is_none() {
            self.head = Some(block);
            self.tail = Some(block);
        } else {
            self.tail.tap_some(|v| (&mut *v.as_ptr()).next = Some(block));
            self.tail = Some(block);
        }
    }

    pub unsafe fn push_front(&mut self, block: NonNull<Block>) {
        if self.head.is_none() && self.tail.is_none() {
            self.head = Some(block);
            self.tail = Some(block);
        } else {
            (&mut *block.as_ptr()).next = self.head;
            self.head = Some(block);
        }
    }

    pub unsafe fn pop_front(&mut self) -> Option<NonNull<Block>> {
        let head = self.head;

        if let Some(head) = head {
            self.head = (&mut *head.as_ptr()).next;

            if self.head.is_none() {
                self.tail = None;
            }
        }

        head
    }
}

#[derive(Debug)]
struct Block {
    next: Option<NonNull<Block>>,
}
