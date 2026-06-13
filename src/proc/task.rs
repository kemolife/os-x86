//! Task table + round-robin scheduler for ring-0 kernel threads.

use crate::mm::heap::kmalloc;

const MAX_TASKS: usize = 8;
const STACK_SIZE: usize = 8 * 1024;

#[derive(Clone, Copy, PartialEq)]
enum State {
    Unused,
    Ready,
    Running,
    Finished,
}

#[derive(Clone, Copy)]
struct Task {
    esp: u32,   // saved stack pointer (the switch frame lives here)
    stack: u32, // base of the allocated stack (for cleanup later)
    id: u32,
    state: State,
}

const EMPTY: Task = Task {
    esp: 0,
    stack: 0,
    id: 0,
    state: State::Unused,
};

static mut TASKS: [Task; MAX_TASKS] = [EMPTY; MAX_TASKS];
static mut CURRENT: usize = 0;
static mut NUM_TASKS: usize = 0;
static mut NEXT_ID: u32 = 1;
static mut ENABLED: bool = false;

extern "C" {
    fn switch_context(save_old_esp: *mut u32, new_esp: u32);
}

/// Register the current (boot) execution as task 0. Its ESP is captured the
/// first time we switch away from it.
pub unsafe fn init() {
    TASKS[0] = Task {
        esp: 0,
        stack: 0,
        id: 0,
        state: State::Running,
    };
    NUM_TASKS = 1;
    CURRENT = 0;
}

/// When a thread function returns, it lands here: mark finished and yield away.
extern "C" fn task_exit() {
    unsafe {
        TASKS[CURRENT].state = State::Finished;
        loop {
            schedule();
        }
    }
}

/// Create a kernel thread that begins executing `entry`.
pub unsafe fn spawn(entry: extern "C" fn()) {
    if NUM_TASKS >= MAX_TASKS {
        return;
    }
    let stack = kmalloc(STACK_SIZE) as u32;
    let mut sp = (stack + STACK_SIZE as u32) as *mut u32;

    // Hand-craft the frame that `switch_context` will unwind into:
    //   ret      -> entry, and entry's own `ret` -> task_exit
    //   popf     -> eflags with IF set
    //   popa     -> 8 scratch registers (don't care)
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

    TASKS[NUM_TASKS] = Task {
        esp: sp as u32,
        stack,
        id: NEXT_ID,
        state: State::Ready,
    };
    NEXT_ID += 1;
    NUM_TASKS += 1;
}

/// Begin preemptive scheduling (timer IRQ will call `schedule`).
pub unsafe fn enable() {
    ENABLED = true;
}

pub unsafe fn enabled() -> bool {
    ENABLED
}

/// Round-robin: pick the next Ready/Running task and switch to it.
pub unsafe fn schedule() {
    if NUM_TASKS < 2 {
        return;
    }
    let prev = CURRENT;
    let mut next = prev;
    let mut found = false;
    for _ in 0..NUM_TASKS {
        next = (next + 1) % NUM_TASKS;
        let s = TASKS[next].state;
        if s == State::Ready || s == State::Running {
            found = true;
            break;
        }
    }
    if !found || next == prev {
        return;
    }

    if TASKS[prev].state == State::Running {
        TASKS[prev].state = State::Ready;
    }
    TASKS[next].state = State::Running;
    CURRENT = next;

    let save = &raw mut TASKS[prev].esp;
    let new_esp = TASKS[next].esp;
    switch_context(save, new_esp);
}
