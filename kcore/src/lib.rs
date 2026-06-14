//! kcore — the shared hardware-abstraction layer used by both the monolithic
//! and microkernel builds: CPU (GDT/IDT/ISR/timer/ports), drivers (VGA, serial,
//! keyboard, ATA), memory management (E820/PMM/paging/heap), and a tiny libc.
//!
//! Kernel-specific policy (scheduler, syscall/IPC dispatch) is reached through
//! `hooks`, which each kernel registers at boot.

#![no_std]
#![allow(dead_code)]

extern crate alloc;

pub mod hooks;
pub mod cpu;
pub mod drivers;
pub mod libc;
pub mod mm;

use core::alloc::{GlobalAlloc, Layout};

struct KernelAlloc;

unsafe impl GlobalAlloc for KernelAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.align() > 16 {
            return core::ptr::null_mut();
        }
        mm::heap::kmalloc(layout.size())
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        mm::heap::kfree(ptr);
    }
}

#[global_allocator]
static ALLOCATOR: KernelAlloc = KernelAlloc;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nostack, nomem));
        }
    }
}
