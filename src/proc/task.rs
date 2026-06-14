//! Task table + round-robin scheduler for ring-0 kernel threads.

use crate::mm::heap::kmalloc;

const MAX_TASKS: usize = 8;
const STACK_SIZE: usize = 8 * 1024;

#[derive(Clone, Copy, PartialEq)]
enum State {
    Unused,
    Ready,
    Running,
    Blocked,  // sleeping until `wake_tick`
    Finished,
}

#[derive(Clone, Copy)]
struct Task {
    esp: u32,        // saved stack pointer (the switch frame lives here)
    stack: u32,      // base of the allocated kernel stack
    kstack_top: u32, // top of the kernel stack — loaded into TSS esp0 when this task runs
    page_dir: u32,   // physical page directory; 0 = use the shared kernel space
    id: u32,
    state: State,
    wake_tick: u32,  // timer tick at which a Blocked task becomes Ready
}

const EMPTY: Task = Task {
    esp: 0,
    stack: 0,
    kstack_top: 0,
    page_dir: 0,
    id: 0,
    state: State::Unused,
    wake_tick: 0,
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
        kstack_top: 0x90000, // the boot stack
        page_dir: 0,
        id: 0,
        state: State::Running,
        wake_tick: 0,
    };
    NUM_TASKS = 1;
    CURRENT = 0;
    // Idle task: runs `hlt` when nothing else is runnable (e.g. all asleep),
    // so the scheduler always has a fallback.
    spawn(idle);
}

extern "C" fn idle() {
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nostack, nomem));
        }
    }
}

/// Block the current task for `ms` milliseconds, yielding the CPU meanwhile.
pub unsafe fn sleep(ms: u32) {
    let now = crate::cpu::timer::ticks();
    let delay = (ms * crate::cpu::timer::TIMER_HZ) / 1000;
    TASKS[CURRENT].wake_tick = now + delay.max(1);
    TASKS[CURRENT].state = State::Blocked;
    schedule();
}

/// Terminate the current task and switch away. If scheduling isn't running
/// (or this is the only task), it returns and the caller continues.
pub unsafe fn exit_current() {
    if !ENABLED || NUM_TASKS < 2 {
        return;
    }
    TASKS[CURRENT].state = State::Finished;
    schedule();
}

/// Number of tasks in the table (for `ps`).
pub unsafe fn count() -> usize {
    NUM_TASKS
}

/// Id of the currently running task (for the getpid syscall).
pub unsafe fn current_id() -> u32 {
    TASKS[CURRENT].id
}

/// (id, state-code) for task `i`: 0=Unused 1=Ready 2=Running 3=Blocked 4=Finished.
pub unsafe fn get(i: usize) -> (u32, u8) {
    let code = match TASKS[i].state {
        State::Unused => 0,
        State::Ready => 1,
        State::Running => 2,
        State::Blocked => 3,
        State::Finished => 4,
    };
    (TASKS[i].id, code)
}

/// Wake any Blocked task whose deadline has passed (called from the timer IRQ).
pub unsafe fn wake_sleepers(now: u32) {
    for i in 0..NUM_TASKS {
        if TASKS[i].state == State::Blocked && now >= TASKS[i].wake_tick {
            TASKS[i].state = State::Ready;
        }
    }
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
        kstack_top: stack + STACK_SIZE as u32,
        page_dir: 0,
        id: NEXT_ID,
        state: State::Ready,
        wake_tick: 0,
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

    // Run the next task on its own kernel stack (so concurrent in-kernel /
    // syscall execution doesn't share one stack), and in its address space.
    crate::cpu::gdt::set_kernel_stack(TASKS[next].kstack_top);
    if TASKS[next].page_dir != 0 {
        crate::mm::paging::switch_address_space(TASKS[next].page_dir);
    } else {
        crate::mm::paging::switch_to_kernel_space();
    }

    let save = &raw mut TASKS[prev].esp;
    let new_esp = TASKS[next].esp;
    switch_context(save, new_esp);
}
