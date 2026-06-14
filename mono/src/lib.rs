//! Monolithic kernel: drivers, filesystem, and the shell all run in the kernel.
//! Built on the shared `oscore` HAL. Exports `kernel_main` for the bootloader.

#![no_std]
#![allow(dead_code)]

extern crate alloc;

pub mod proc;
pub mod fs;
pub mod syscall;
pub mod shell;
pub mod kernel;
