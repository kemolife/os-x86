//! Cooperative/preemptive kernel-thread scheduler.
//!
//! All tasks run in ring 0 and share the kernel address space (one page
//! directory), so a context switch is just a stack swap — no CR3 reload, no
//! TSS needed yet. Preemption is driven by the timer IRQ calling `schedule()`.

pub mod task;

pub use task::{enable, enabled, init, schedule, sleep, spawn};
