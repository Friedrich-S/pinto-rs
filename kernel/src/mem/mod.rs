pub use self::alloc::*;
pub use pages::*;

use bootloader_api::info::MemoryRegionKind;
use core::ops::Deref;
use spinning_top::const_spinlock;
use spinning_top::Spinlock;

mod alloc;
mod pages;

/// The index of the first offset bit.
pub const PAGE_OFFSET_SHIFT: u32 = 0;
/// The number of offset bits.
pub const PAGE_OFFSET_BITS: u32 = 12;
/// The size of the memory pages (4 kB).
pub const PAGE_SIZE: u64 = 1 << PAGE_OFFSET_BITS;
pub const PAGE_OFFSET_MASK: u64 = ((1u64 << PAGE_OFFSET_BITS) - 1) << PAGE_OFFSET_SHIFT;

pub const PHYS_BASE: u64 = 0xc0000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualAddress {
    val: u64,
}

impl VirtualAddress {
    /// Creates a new [`VirtualAddress`].
    pub const fn new(val: u64) -> Self {
        Self { val }
    }

    /// Returns the raw address.
    pub fn raw(&self) -> u64 {
        self.val
    }

    /// Returns whether the address is a user address.
    pub fn is_user(&self) -> bool {
        self.val < (MEMORY_INFO.lock().base_virtual_address + PHYS_BASE)
    }

    /// Returns whether the address is a kernel address.
    pub fn is_kernel(&self) -> bool {
        self.val >= (MEMORY_INFO.lock().base_virtual_address + PHYS_BASE)
    }

    pub fn to_kernel_physical(&self) -> PhysicalAddress {
        assert!(self.is_kernel());

        PhysicalAddress::new(self.val - MEMORY_INFO.lock().base_virtual_address - PHYS_BASE)
    }

    pub fn page_offset(&self) -> u64 {
        self.val & PAGE_OFFSET_MASK
    }

    pub fn page_num(&self) -> u64 {
        self.val >> PAGE_OFFSET_BITS
    }

    /// Round down to the nearest page boundary.
    pub fn page_round_down(&self) -> Self {
        Self::new(self.val & !PAGE_OFFSET_MASK)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysicalAddress {
    val: u64,
}

impl PhysicalAddress {
    /// Creates a new [`PhysicalAddress`] relative to the start of the usable memory region.
    pub fn new(val: u64) -> Self {
        Self {
            val: val + MEMORY_INFO.lock().base_address,
        }
    }

    /// Creates a new [`PhysicalAddress`] using the absolute address.
    pub fn new_abs(val: u64) -> Self {
        Self { val }
    }

    /// Returns the raw address.
    pub fn raw(&self) -> u64 {
        self.val
    }

    /// Returns the physical address as a pointer.
    pub fn get<T>(&self) -> *mut T {
        self.val as *mut T
    }

    pub fn to_kernel_virtual(self) -> VirtualAddress {
        let info = MemoryInfo::get();

        VirtualAddress::new(self.val + info.base_virtual_address + PHYS_BASE)
    }
}

static MEMORY_INFO: Spinlock<MemoryInfo> = const_spinlock(MemoryInfo {
    base_address: 0,
    size: 0,
    base_virtual_address: 0,
});

/// Contains information about the available memory regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryInfo {
    pub base_address: u64,
    pub size: u64,
    pub base_virtual_address: u64,
}

impl MemoryInfo {
    /// Initializes the global memory info state.
    pub fn init(boot_info: &'static mut bootloader_api::BootInfo) {
        let mut info = Self::get();

        for region in boot_info.memory_regions.deref() {
            if let MemoryRegionKind::Usable = region.kind {
                info.base_address = region.start;
                info.size = region.end - region.start;
                break;
            }
        }

        info.base_virtual_address = boot_info.physical_memory_offset.into_option().unwrap_or(0);

        *MEMORY_INFO.lock() = info;
    }

    /// Returns a copy of the global memory info.
    pub fn get() -> MemoryInfo {
        *MEMORY_INFO.lock()
    }
}

impl MemoryInfo {
    /// Returns the amount of physical memory in 4 kB pages
    pub fn num_pages(&self) -> u32 {
        (self.size / PAGE_SIZE) as u32
    }
}
