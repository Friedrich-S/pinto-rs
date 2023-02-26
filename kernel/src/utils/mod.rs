use bitvec::prelude::BitOrder;
use bitvec::slice::BitSlice;
use bitvec::store::BitStore;

pub trait BitSliceScan {
    fn scan(&self, start: usize, num: usize, val: bool) -> Option<usize>;

    fn scan_and_flip(&mut self, start: usize, num: usize, val: bool) -> Option<usize>;
}

impl<T: BitStore, O: BitOrder> BitSliceScan for BitSlice<T, O> {
    fn scan(&self, start: usize, num: usize, val: bool) -> Option<usize> {
        if start > self.len() {
            return None;
        }

        if (start + num) <= self.len() {
            let last = self.len() - num;
            for i in start..=last {
                let sub_slice = self.get(start..(start + num))?;
                let is_valid = match val {
                    true => sub_slice.all(),
                    false => sub_slice.not_any(),
                };
                if is_valid {
                    return Some(i);
                }
            }
        }

        None
    }

    fn scan_and_flip(&mut self, start: usize, num: usize, val: bool) -> Option<usize> {
        let idx = self.scan(start, num, val)?;
        self.get_mut(start..(start + num))?.fill(val);

        Some(idx)
    }
}

/// Reads and returns the value of the stack pointer register.
pub fn read_esp() -> usize {
    let esp: usize;

    // SAFETY: it is safe to read from a register.
    unsafe {
        core::arch::asm!("mov {}, [esp]", out(reg) esp, options(nostack, nomem, preserves_flags));
    }

    esp
}
