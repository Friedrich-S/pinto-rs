use crate::mem::MemoryInfo;
use crate::mem::PhysicalAddress;
use crate::mem::VirtualAddress;
use crate::mem::PAGE_SIZE;
use crate::println;
use crate::utils::BitSliceScan;
use bitvec::slice::BitSlice;
use core::ops::DerefMut;
use core::ptr::NonNull;
use enumflags2::bitflags;
use enumflags2::BitFlags;
use spinning_top::const_spinlock;
use spinning_top::Spinlock;

static PAGE_ALLOC: PageAllocator = PageAllocator {
    kernel_pool: Pool::new(),
    user_pool: Pool::new(),
};

type UsedMapType = usize;

#[bitflags]
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageAllocFlags {
    Zero,
    User,
}

#[derive(Debug)]
pub struct PageAllocator {
    kernel_pool: Pool,
    user_pool: Pool,
}

impl PageAllocator {
    pub fn init(user_page_limit: u64) {
        let free_start = PhysicalAddress::new(1024 * 1024).to_kernel_virtual();
        let free_end = PhysicalAddress::new(MemoryInfo::get().size).to_kernel_virtual();
        let free_pages = (free_end.raw() - free_start.raw()) / PAGE_SIZE;
        let user_pages = (free_pages / 2).min(user_page_limit);
        let kernel_pages = free_pages - user_pages;

        let base = free_start.raw();
        PAGE_ALLOC.kernel_pool.init(base, kernel_pages, "kernel pool");
        let base = free_start.raw() + kernel_pages * PAGE_SIZE;
        PAGE_ALLOC.user_pool.init(base, user_pages, "user pool");
    }

    pub fn get_pages(flags: BitFlags<PageAllocFlags>, num: usize) -> Option<NonNull<()>> {
        if num == 0 {
            return None;
        }

        let pool = match flags.contains(PageAllocFlags::User) {
            true => &PAGE_ALLOC.user_pool,
            false => &PAGE_ALLOC.kernel_pool,
        };

        let page_idx = {
            let mut used_map = pool.used_map.lock();
            let used_map = used_map.deref_mut().as_mut()?.0;
            // SAFETY: the used_map points to a static memory location (valid during the entire OS runtime).
            unsafe { (&mut *used_map).scan_and_flip(0, num, false)? }
        };

        let pages = pool.base.lock().raw() + PAGE_SIZE * (page_idx as u64);

        if flags.contains(PageAllocFlags::Zero) {
            // ToDo: write safety statement
            unsafe {
                core::ptr::write_bytes(pages as *mut u8, 0, (PAGE_SIZE as usize) * num);
            }
        }

        Some(NonNull::new(pages as *mut ())?)
    }

    pub fn free_pages(pages: NonNull<()>, num: usize) {
        let page_addr = VirtualAddress::new(pages.as_ptr() as u64);
        assert_eq!(page_addr.page_offset(), 0);

        if num == 0 {
            return;
        }

        let pool = if PAGE_ALLOC.kernel_pool.contains_page(page_addr) {
            &PAGE_ALLOC.kernel_pool
        } else if PAGE_ALLOC.user_pool.contains_page(page_addr) {
            &PAGE_ALLOC.user_pool
        } else {
            unreachable!();
        };

        let page_idx = page_addr.page_num() - pool.base.lock().page_num();

        // ToDo: write safety statement
        #[cfg(debug_assertions)]
        unsafe {
            core::ptr::write_bytes(pages.as_ptr().cast::<u8>(), 0xCC, (PAGE_SIZE as usize) * num);
        }

        let mut used_map = pool.used_map.lock();
        if let Some(used_map) = used_map.deref_mut().as_mut() {
            // SAFETY: the used_map points to a static memory location (valid during the entire OS runtime).
            let slice_range = (page_idx as usize)..((page_idx as usize) + num);
            unsafe { &mut *used_map.0 }.get_mut(slice_range).unwrap().fill(false);
        }
    }
}

#[derive(Debug)]
struct Pool {
    used_map: Spinlock<Option<UsedMap>>,
    base: Spinlock<VirtualAddress>,
}

impl Pool {
    const fn new() -> Self {
        Self {
            used_map: const_spinlock(None),
            base: Spinlock::new(VirtualAddress::new(0)),
        }
    }

    fn init(&self, base: u64, num_pages: u64, name: &'static str) {
        let bitmap_pages = bitvec::mem::elts::<UsedMapType>(num_pages as usize).div_ceil(PAGE_SIZE as usize);
        if (bitmap_pages as u64) > num_pages {
            panic!("Not enough memory in {name} for bitmap.");
        }
        let num_pages = num_pages - (bitmap_pages as u64);

        println!("{num_pages} pages available in {name}");

        let bitmat_slice = unsafe { core::slice::from_raw_parts_mut(base as *mut UsedMapType, 1) };
        *self.used_map.lock() = Some(UsedMap(BitSlice::from_slice_mut(bitmat_slice) as *mut _));
        *self.base.lock() = VirtualAddress::new(base + (bitmap_pages as u64) * PAGE_SIZE);
    }

    fn contains_page(&self, page: VirtualAddress) -> bool {
        let page_no = page.page_num();
        let start_page = self.base.lock().page_num();
        let num_pages = self.used_map.lock().map(|v| unsafe { &*v.0 }.len()).unwrap() as u64;

        page_no >= start_page && page_no < (start_page + num_pages)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct UsedMap(*mut BitSlice<UsedMapType>);

/// This is fine, because [`UsedMap`] only contains a pointer to static memory addresses.
unsafe impl Send for UsedMap {}
