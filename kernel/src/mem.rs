use core::alloc::GlobalAlloc;
use core::alloc::Layout;

#[global_allocator]
static ALLOCATOR: SimpleAlloc = SimpleAlloc;

/// A simple malloc implementation similar to the one used in the original Pintos.
pub struct SimpleAlloc;

unsafe impl GlobalAlloc for SimpleAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        todo!()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        todo!()
    }
}
