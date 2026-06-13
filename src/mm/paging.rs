//! 32-bit paging (no PAE). One page directory of 1024 entries, each pointing
//! at a 1024-entry page table that maps 4MB. We identity-map the low RAM so
//! that physical == virtual for the kernel, then turn on CR0.PG.
//!
//! The page directory and page tables are themselves allocated from the PMM —
//! they live in identity-mapped low RAM, so the CPU can walk them once paging
//! is enabled.

use crate::mm::pmm;

const PAGE_PRESENT: u32 = 1 << 0;
const PAGE_WRITE: u32 = 1 << 1;

/// Number of 4MB page tables to identity-map (16MB covers the kernel, the
/// bitmap, the bump heap, VGA, and the first slice of extended RAM).
const IDENTITY_TABLES: usize = 4;

static mut PAGE_DIR: *mut u32 = core::ptr::null_mut();

pub unsafe fn init() {
    let dir = pmm::alloc_frame().expect("no frame for page directory") as *mut u32;
    for i in 0..1024 {
        *dir.add(i) = 0;
    }

    for t in 0..IDENTITY_TABLES {
        let table = pmm::alloc_frame().expect("no frame for page table") as *mut u32;
        for i in 0..1024 {
            let phys = ((t * 1024 + i) * 0x1000) as u32;
            *table.add(i) = phys | PAGE_PRESENT | PAGE_WRITE;
        }
        *dir.add(t) = (table as u32) | PAGE_PRESENT | PAGE_WRITE;
    }

    PAGE_DIR = dir;

    core::arch::asm!("mov cr3, {}", in(reg) dir as u32, options(nostack));
    let mut cr0: u32;
    core::arch::asm!("mov {}, cr0", out(reg) cr0, options(nostack));
    cr0 |= 1 << 31; // PG
    core::arch::asm!("mov cr0, {}", in(reg) cr0, options(nostack));
}

/// Read the faulting linear address from CR2 (used by the page-fault handler).
pub unsafe fn fault_address() -> u32 {
    let cr2: u32;
    core::arch::asm!("mov {}, cr2", out(reg) cr2, options(nostack, nomem));
    cr2
}
