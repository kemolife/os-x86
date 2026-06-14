//! Microkernel build: a tiny kernel whose only real service is IPC; "work"
//! (here, an echo server) runs as separate tasks talking by messages.
//! Built on the shared `oscore` HAL. Exports `kernel_main` for the bootloader.

#![no_std]
#![allow(dead_code)]

pub mod sched;
pub mod ipc;
pub mod kernel;
