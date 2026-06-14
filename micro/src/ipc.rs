//! The microkernel's core service: synchronous message passing between tasks.
//! `recv` blocks the caller until a message arrives; `send` delivers one and
//! wakes the receiver. This is what a microkernel is built around — drivers and
//! servers would talk to clients only through messages like these.

use crate::sched;

const MAX: usize = sched::MAX;

static mut MSG: [u32; MAX] = [0; MAX];
static mut SRC: [usize; MAX] = [0; MAX];
static mut FULL: [bool; MAX] = [false; MAX];

/// Send `val` to task `dst` (from the current task).
pub unsafe fn send(dst: usize, val: u32) {
    let me = sched::current();
    MSG[dst] = val;
    SRC[dst] = me;
    FULL[dst] = true;
    sched::unblock(dst);
}

/// Block until a message arrives; returns (sender task, value).
pub unsafe fn recv() -> (usize, u32) {
    let me = sched::current();
    while !FULL[me] {
        sched::block_current();
    }
    FULL[me] = false;
    (SRC[me], MSG[me])
}
