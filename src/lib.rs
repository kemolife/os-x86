#![no_std]
#![allow(dead_code)]

extern crate alloc;

pub mod cpu;
pub mod drivers;
pub mod libc;
pub mod mm;
pub mod kernel;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!("hlt", options(nostack, nomem)); }
    }
}
