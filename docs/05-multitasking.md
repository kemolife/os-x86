# 05 — Multitasking

## Why we need it

A single thread of execution can only do one thing. Multitasking lets several
**tasks** (here, kernel threads) make progress "at the same time" by rapidly
switching the CPU between them. It's the basis for a responsive system and,
later, for running user programs.

Our tasks are **ring-0 kernel threads**: they all run at full privilege and
share one address space (one page directory), so switching between them is just
swapping the CPU's stack — no `CR3` reload, no **TSS** (Task State Segment, only
needed for privilege-level changes) yet.

## File structure

```
cpu/switch.asm        switch_context — the low-level stack swap (assembly)
mono/src/proc/task.rs      task table, spawn(), scheduler, sleep()
mono/src/proc/mod.rs        re-exports
oscore/src/cpu/timer.rs      timer IRQ calls schedule() + wakes sleepers
```

## How it works

### 1. The context switch (`cpu/switch.asm`)

A task's entire CPU state, when it isn't running, lives **on its own stack**.
`switch_context(save_old_esp, new_esp)` saves the current state and loads
another's:

```asm
switch_context:
    pushf                 ; save EFLAGS
    pusha                 ; save all 8 general registers
    mov  eax, [esp+40]    ; arg: where to store the old ESP
    mov  edx, [esp+44]    ; arg: the new ESP to switch to
    mov  [eax], esp       ; remember where we left off
    mov  esp, edx         ; switch stacks  <-- the actual context switch
    popa                  ; restore the other task's registers
    popf                  ; restore its EFLAGS
    ret                   ; return into the other task
```

The trick: after `mov esp, edx` every `pop` reads the *other* task's saved
values. The function "returns" into a completely different task.

### 2. Tasks and the table (`task.rs`)

```rust
struct Task { esp: u32, stack: u32, id: u32, state: State, wake_tick: u32 }
enum State { Unused, Ready, Running, Blocked, Finished }
```

`spawn(entry)` allocates a stack and hand-crafts a fake saved-context on it so
the first switch "returns" into `entry`:

```
[ entry address ]   <- the final `ret` jumps here
[ EFLAGS = 0x202 ]  <- popf loads this (0x202 has the Interrupt Flag set)
[ 8 zero dwords  ]  <- popa loads these (scratch)
```

When `entry` eventually returns, it lands on a `task_exit` trampoline that marks
the task `Finished` and yields forever — the scheduler then ignores it.

### 3. The scheduler (`schedule()`)

Round-robin: starting after the current task, find the next `Ready`/`Running`
task and `switch_context` to it. `Blocked` (sleeping) and `Finished` tasks are
skipped. An always-runnable **idle task** (a `hlt` loop) is spawned at init so
the scheduler never runs out of choices when everything else is asleep.

### 4. Preemption (`timer.rs`)

The 50 Hz timer IRQ calls `schedule()` on every tick (after waking any due
sleepers). That's what makes it *preemptive* — a task doesn't have to
cooperate; the timer forcibly switches it out.

### 5. `sleep(ms)`

```rust
pub unsafe fn sleep(ms: u32) {
    let wake = ticks() + ms * TIMER_HZ / 1000;
    TASKS[CURRENT].wake_tick = wake;
    TASKS[CURRENT].state = State::Blocked;   // stop scheduling me
    schedule();                              // give the CPU away now
}
```

The task is marked `Blocked` and yields immediately. Each timer tick,
`wake_sleepers` flips any task whose deadline passed back to `Ready`. So sleeping
costs **zero CPU** — the difference from a busy-wait loop.

## How to test it — and what the output means

The boot demo spawns two threads then continues. On serial you see:

```
[A][B][A][B][B][A][B][A][A][B][A done][B done]
```

**This is the core multitasking proof.** Two kernel threads (A and B) each print
their letter:

- They run **concurrently** — the 50 Hz timer **preempts** whichever is running
  and switches to the other. That interleaving is why you see them mixed.
- The order is **not** a strict `ABAB`. Note `[B][B]` and `[A][A]`. It depends on
  exactly when each timer tick lands relative to each thread's work — just like
  a real scheduler, the timing isn't perfectly regular.
- `[A done][B done]` = both threads finished their loops and exited cleanly via
  `task_exit`. Afterward the main task carries on alone.

### The sleep variant

In the current demo `thread_a` sleeps 1 second between prints while `thread_b`
stays busy:

```
[A] [B] [B] [B] [A] [B] [B] [B] [A] [B] [B] [A done] [B] [B] [B] [B] [B done]
```

`[A]` appears, then several `[B]`s, then `[A]` again ~1s later. **The gaps full
of `[B]` are the proof:** while `thread_a` is asleep it is `Blocked` and uses no
CPU, so the scheduler hands every slice to `thread_b` (and the idle task). If
`sleep` were a busy-loop, A would still be hogging time and B couldn't run that
much.

Run it:

```bash
docker run -it --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 \
  qemu-system-i386 -m 128 -drive file=os-image-mono.bin,format=raw,if=floppy -nographic
```
