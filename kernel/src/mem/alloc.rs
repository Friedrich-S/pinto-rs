use crate::mem::PageAllocator;
use crate::mem::VirtualAddress;
use crate::mem::PAGE_SIZE;
use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::ptr::NonNull;
use enumflags2::BitFlags;
use spinning_top::const_spinlock;
use spinning_top::Spinlock;
use tap::Pipe;
use tap::Tap;
use tap::TapOptional;

const NUM_DESCS: usize = (PAGE_SIZE / 32).ilog2() as usize;

#[global_allocator]
static ALLOCATOR: SimpleAlloc = SimpleAlloc { descs: &ALLOC_DESCS };
const DEFAULT_DESC: Spinlock<Descriptor> = const_spinlock(Descriptor::new());
static ALLOC_DESCS: [Spinlock<Descriptor>; NUM_DESCS] = [DEFAULT_DESC; NUM_DESCS];

pub fn init_heap() {
    ALLOCATOR.init();
}

/// A simple malloc implementation similar to the one used in the original Pintos.
pub struct SimpleAlloc {
    descs: &'static [Spinlock<Descriptor>; NUM_DESCS],
}

impl SimpleAlloc {
    fn init(&self) {
        let mut block_size = 16;
        for desc in self.descs {
            let mut desc = desc.lock();
            desc.block_size = block_size;
            desc.blocks_per_arena = ((PAGE_SIZE as usize) - core::mem::size_of::<Arena>()) / block_size;

            block_size *= 2;
        }
    }
}

// ToDo: add safety notes to all unsafe function calls here
unsafe impl GlobalAlloc for SimpleAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // The safety requirements state that the caller must ensure that the layout
        // must have a non-zero size, so we do not need to check this.

        let desc = self.descs.iter().find(|d| d.lock().block_size >= layout.size());

        if let Some(desc_raw) = desc {
            let mut desc = desc_raw.lock();
            if desc.free_list.is_empty() {
                let Some(arena) = PageAllocator::get_pages(BitFlags::empty(), 1) else {
                    return core::ptr::null_mut();
                };
                let arena = arena
                    .tap(|a| unsafe {
                        *a.clone().cast::<Arena>().as_mut() = Arena {
                            magic: Arena::MAGIC,
                            desc: Some(desc_raw),
                            num_free: desc.blocks_per_arena,
                        }
                    })
                    .cast::<Arena>();

                for i in 0..desc.blocks_per_arena {
                    let block = arena.as_ref().to_block(i);
                    desc.free_list.push_back(block);
                }
            }

            let Some(block) = desc.free_list.pop_front() else {
                return core::ptr::null_mut();
            };
            let mut arena = block.as_ref().to_arena();
            arena.as_mut().num_free -= 1;
            block.cast().as_ptr()
        } else {
            // The requested size is too big for any descriptor.
            // ALlocate enough pages to hold the size plus an arena
            let num_pages = (layout.size() + core::mem::size_of::<Arena>()).div_ceil(PAGE_SIZE as usize);
            let Some(arena) = PageAllocator::get_pages(BitFlags::empty(), num_pages) else {
                return core::ptr::null_mut();
            };
            let arena = arena.tap(|a| unsafe {
                *a.clone().cast::<Arena>().as_mut() = Arena {
                    magic: Arena::MAGIC,
                    desc,
                    num_free: num_pages,
                }
            });
            arena.cast().as_ptr()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the safety requirements state that `ptr` must not be null.
        let block = NonNull::new_unchecked(ptr).cast::<Block>();
        let mut arena = block.as_ref().to_arena();

        if let Some(desc) = arena.as_ref().desc {
            // It's a normal block, handle it here.

            let mut desc = desc.lock();

            #[cfg(debug_assertions)]
            core::ptr::write_bytes(block.cast::<u8>().as_ptr(), 0xCC, desc.block_size);

            desc.free_list.push_front(block);

            // If the arena is now entirely unused, free it.
            arena.as_mut().num_free += 1;
            if arena.as_mut().num_free >= desc.blocks_per_arena {
                for i in 0..desc.blocks_per_arena {
                    let block = arena.as_ref().to_block(i);
                    desc.free_list.remove(block);
                }
            }
        } else {
            // It's a big block, free its pages.
            PageAllocator::free_pages(arena.cast(), arena.as_ref().num_free);
        }
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

    unsafe fn to_block(&self, idx: usize) -> NonNull<Block> {
        let (blocks_per_arena, block_size) =
            self.desc.map(|d| d.lock().pipe(|l| (l.blocks_per_arena, l.block_size))).unwrap_or((0, 0));

        assert_eq!(self.magic, Self::MAGIC);
        assert!(idx < blocks_per_arena);
        // SAFETY: this is save here, because it is relative to &self, which by definition
        // cannot be null.
        NonNull::new_unchecked((self as *const Arena).add(1).cast::<u8>().cast_mut().add(idx * block_size)).cast()
    }
}

#[derive(Debug)]
struct BlockList {
    head: Option<NonNull<Block>>,
    tail: Option<NonNull<Block>>,
}

impl BlockList {
    const fn new() -> Self {
        Self { head: None, tail: None }
    }

    fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    unsafe fn push_back(&mut self, block: NonNull<Block>) {
        if self.head.is_none() && self.tail.is_none() {
            self.head = Some(block);
            self.tail = Some(block);
        } else {
            self.tail.tap_some(|v| (&mut *v.as_ptr()).next = Some(block));
            self.tail = Some(block);
        }
    }

    unsafe fn push_front(&mut self, block: NonNull<Block>) {
        if self.head.is_none() && self.tail.is_none() {
            self.head = Some(block);
            self.tail = Some(block);
        } else {
            (&mut *block.as_ptr()).next = self.head;
            self.head = Some(block);
        }
    }

    unsafe fn pop_front(&mut self) -> Option<NonNull<Block>> {
        let head = self.head;

        if let Some(head) = head {
            self.head = (&mut *head.as_ptr()).next;

            if self.head.is_none() {
                self.tail = None;
            }
        }

        head
    }

    unsafe fn remove(&mut self, block: NonNull<Block>) {
        if let Some(head) = self.head {
            let mut prev_v = head;
            let mut v = head;

            while v != block {
                let Some(next) = v.as_ref().next else {
                    return;
                };
                prev_v = v;
                v = next;
            }

            // The head is removed
            if v == head {
                // The list is now empty
                if self.head == self.tail {
                    self.head = None;
                    self.tail = None;
                } else {
                    self.head = v.as_ref().next;
                }
            } else {
                // Close the hole in the list
                prev_v.as_mut().next = v.as_ref().next;
            }
        }
    }
}

#[derive(Debug)]
struct Block {
    next: Option<NonNull<Block>>,
}

impl Block {
    unsafe fn to_arena(&self) -> NonNull<Arena> {
        let addr = VirtualAddress::new(self as *const _ as _);
        let arena = addr.page_round_down().raw();
        // SAFETY: this is save here, because it is relative to &self, which by definition
        // cannot be null.
        let arena = NonNull::new_unchecked(arena as *mut Arena);

        let a = arena.as_ref();
        assert_eq!(a.magic, Arena::MAGIC);
        // SAFETY: the first OR condition validates that unwrap_unchecked in the second one is always valid.
        assert!(
            a.desc.is_none()
                || ((addr.page_offset() as usize) - core::mem::size_of::<Arena>())
                    % a.desc.unwrap_unchecked().lock().block_size
                    == 0
        );
        assert!(a.desc.is_some() || (addr.page_offset() as usize) == core::mem::size_of::<Arena>());

        arena
    }
}
