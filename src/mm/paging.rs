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
const PAGE_USER: u32 = 1 << 2; // accessible from ring 3

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
            // Supervisor-only: the kernel identity map is not reachable from
            // ring 3. User programs get their own pages in a separate space.
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

/// Create a new address space: a fresh page directory that shares the kernel's
/// identity-mapped (supervisor) low memory, with no user mappings yet. Returns
/// the directory's physical address.
pub unsafe fn new_address_space() -> u32 {
    let dir = pmm::alloc_frame().expect("no frame for page directory") as *mut u32;
    for i in 0..1024 {
        *dir.add(i) = 0;
    }
    // Share the kernel's identity-map page tables so the kernel stays mapped
    // (supervisor) in every address space — needed for syscalls/interrupts.
    for i in 0..IDENTITY_TABLES {
        *dir.add(i) = *PAGE_DIR.add(i);
    }
    dir as u32
}

/// Map a user-accessible page `virt` -> `phys` in the address space `dir_phys`,
/// creating the page table if needed. `dir_phys` need not be the active space.
pub unsafe fn map_user_page(dir_phys: u32, virt: u32, phys: u32) {
    let dir = dir_phys as *mut u32;
    let di = (virt >> 22) as usize;
    let ti = ((virt >> 12) & 0x3FF) as usize;
    if *dir.add(di) & PAGE_PRESENT == 0 {
        let table = pmm::alloc_frame().expect("no frame for page table") as u32;
        let t = table as *mut u32;
        for i in 0..1024 {
            *t.add(i) = 0;
        }
        *dir.add(di) = table | PAGE_PRESENT | PAGE_WRITE | PAGE_USER;
    }
    let table = (*dir.add(di) & !0xFFF) as *mut u32;
    *table.add(ti) = (phys & !0xFFF) | PAGE_PRESENT | PAGE_WRITE | PAGE_USER;
}

/// Switch the active address space (load CR3 with a page-directory physical addr).
pub unsafe fn switch_address_space(dir_phys: u32) {
    core::arch::asm!("mov cr3, {}", in(reg) dir_phys, options(nostack));
}

/// Switch back to the shared kernel address space.
pub unsafe fn switch_to_kernel_space() {
    core::arch::asm!("mov cr3, {}", in(reg) PAGE_DIR as u32, options(nostack));
}

/// Physical address of the kernel page directory.
pub unsafe fn kernel_dir() -> u32 {
    PAGE_DIR as u32
}

/// Read the faulting linear address from CR2 (used by the page-fault handler).
pub unsafe fn fault_address() -> u32 {
    let cr2: u32;
    core::arch::asm!("mov {}, cr2", out(reg) cr2, options(nostack, nomem));
    cr2
}

/// Demand-paging window: faults here are satisfied by mapping a fresh frame.
/// It sits just above the identity-mapped region.
const GROW_BASE: u32 = 0x0100_0000; // 16MB
const GROW_TOP: u32 = 0x0200_0000; // 32MB

/// Map one 4KB page `virt` -> `phys`, creating the page table if needed.
pub unsafe fn map_page(virt: u32, phys: u32, flags: u32) {
    let di = (virt >> 22) as usize;
    let ti = ((virt >> 12) & 0x3FF) as usize;

    if *PAGE_DIR.add(di) & PAGE_PRESENT == 0 {
        let table = pmm::alloc_frame().expect("no frame for page table") as u32;
        let t = table as *mut u32;
        for i in 0..1024 {
            *t.add(i) = 0;
        }
        *PAGE_DIR.add(di) = table | PAGE_PRESENT | PAGE_WRITE;
    }

    let table = (*PAGE_DIR.add(di) & !0xFFF) as *mut u32;
    *table.add(ti) = (phys & !0xFFF) | flags | PAGE_PRESENT;
    core::arch::asm!("invlpg [{}]", in(reg) virt, options(nostack));
}

/// Page-fault recovery: if the fault is in the demand-paging window, back it
/// with a fresh frame and report success so the faulting instruction retries.
pub unsafe fn handle_fault(addr: u32) -> bool {
    if addr >= GROW_BASE && addr < GROW_TOP {
        if let Some(frame) = pmm::alloc_frame() {
            map_page(addr & !0xFFF, frame as u32, PAGE_WRITE);
            return true;
        }
    }
    false
}
