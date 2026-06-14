//! Minimal round-robin scheduler for the microkernel demo. Reuses the shared
//! `switch_context` (cpu/switch.asm). Tasks block/unblock for IPC.

use oscore::mm::heap::kmalloc;

pub const MAX: usize = 4;
const STACK: usize = 8 * 1024;

#[derive(Clone, Copy, PartialEq)]
enum State {
    Unused,
    Ready,
    Running,
    Blocked,
}

#[derive(Clone, Copy)]
struct Task {
    esp: u32,
    state: State,
}

static mut TS: [Task; MAX] = [Task { esp: 0, state: State::Unused }; MAX];
static mut CUR: usize = 0;
static mut N: usize = 0;

extern "C" {
    fn switch_context(save_old_esp: *mut u32, new_esp: u32);
}

extern "C" fn task_exit() {
    unsafe {
        TS[CUR].state = State::Unused;
        loop {
            schedule();
        }
    }
}

pub unsafe fn init() {
    TS[0] = Task { esp: 0, state: State::Running };
    N = 1;
    CUR = 0;
}

pub unsafe fn spawn(entry: extern "C" fn()) -> usize {
    let stack = kmalloc(STACK) as u32;
    let mut sp = (stack + STACK as u32) as *mut u32;
    sp = sp.offset(-1);
    *sp = task_exit as *const () as u32;
    sp = sp.offset(-1);
    *sp = entry as *const () as u32;
    sp = sp.offset(-1);
    *sp = 0x202; // EFLAGS, IF=1
    for _ in 0..8 {
        sp = sp.offset(-1);
        *sp = 0;
    }
    let slot = N;
    TS[slot] = Task { esp: sp as u32, state: State::Ready };
    N += 1;
    slot
}

pub unsafe fn current() -> usize {
    CUR
}

pub unsafe fn block_current() {
    TS[CUR].state = State::Blocked;
    schedule();
}

pub unsafe fn unblock(i: usize) {
    if TS[i].state == State::Blocked {
        TS[i].state = State::Ready;
    }
}

pub unsafe fn schedule() {
    if N < 2 {
        return;
    }
    let prev = CUR;
    let mut next = prev;
    let mut found = false;
    for _ in 0..N {
        next = (next + 1) % N;
        let s = TS[next].state;
        if s == State::Ready || s == State::Running {
            found = true;
            break;
        }
    }
    if !found || next == prev {
        return;
    }
    if TS[prev].state == State::Running {
        TS[prev].state = State::Ready;
    }
    TS[next].state = State::Running;
    CUR = next;
    switch_context(&raw mut TS[prev].esp, TS[next].esp);
}
