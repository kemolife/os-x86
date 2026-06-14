//! Hooks the shared HAL calls "up" into whichever kernel is linked.
//!
//! The low-level code (timer IRQ, int 0x80) must invoke kernel policy that
//! differs between the monolithic and microkernel builds — the scheduler and
//! the syscall/IPC dispatcher. Each kernel registers its functions at boot;
//! the HAL calls them through these indirections instead of hard-linking to a
//! specific kernel's modules.

pub type SyscallFn = fn(u32, u32, u32, u32) -> u32;
pub type TickFn = fn();

static mut SYSCALL: Option<SyscallFn> = None;
static mut TICK: Option<TickFn> = None;

pub unsafe fn set_syscall(f: SyscallFn) {
    SYSCALL = Some(f);
}

pub unsafe fn set_tick(f: TickFn) {
    TICK = Some(f);
}

/// Called from the int 0x80 handler. Returns the syscall result (or 0 if none).
pub unsafe fn syscall(n: u32, a: u32, b: u32, c: u32) -> u32 {
    match SYSCALL {
        Some(f) => f(n, a, b, c),
        None => 0,
    }
}

/// Called once per timer tick (after the HAL's own bookkeeping).
pub unsafe fn tick() {
    if let Some(f) = TICK {
        f();
    }
}
