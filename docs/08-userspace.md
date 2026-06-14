# 08 — User Space (ring 3 + syscalls)

## Why we need it

So far everything runs at **ring 0** — full privilege, able to touch any
hardware. Real programs must run at **ring 3** (unprivileged) so a buggy or
malicious program can't crash the machine or read other memory. A program at
ring 3 can only ask the kernel to do privileged things through a narrow,
checked doorway: the **system call**.

x86 has four privilege levels (rings) 0–3; we use 0 (kernel) and 3 (user).

## File structure

```
src/cpu/gdt.rs       user code/data descriptors, the TSS, enter_user_mode()
cpu/interrupt.asm    isr128 stub for the int 0x80 syscall vector
src/cpu/idt.rs       set_idt_gate_flags() — install a DPL-3 (user-callable) gate
src/cpu/isr.rs       isr_handler routes int 0x80 to the syscall dispatcher
src/syscall/mod.rs   dispatch + sys_write + sys_exit
```

## The three pieces

### 1. Segments + TSS (`gdt.rs`)

Ring 3 needs its own code/data descriptors with **DPL** (Descriptor Privilege
Level) = 3, and a **TSS** (Task State Segment). The TSS matters because when a
ring-3 program triggers an interrupt, the CPU must switch to a *kernel* stack to
run the handler — it reads that stack pointer (`esp0`) from the TSS.

GDT selectors: `0x08/0x10` kernel code/data, `0x18/0x20` user code/data (DPL 3),
`0x28` TSS.

### 2. The syscall gate (`int 0x80`)

`int 0x80` is the doorway. Its IDT gate is installed with flags `0xEE` — the `E`
encodes DPL 3, so *user* code is allowed to invoke it (a normal DPL-0 gate would
#GP if called from ring 3). The handler reads the call number from `EAX` and
arguments from `EBX/ECX/EDX`, dispatches, and writes the result back into the
saved `EAX`.

### 3. Dropping to ring 3 (`enter_user_mode`)

You can't just `jmp` to ring 3 — you return to it as if from an interrupt, with
`iret`, which pops a 5-word frame and switches privilege:

```
push  SS      (user data | RPL 3)
push  ESP     (user stack)
push  EFLAGS  (0x202, interrupts on)
push  CS      (user code | RPL 3)
push  EIP     (user entry point)
iretd          ; 32-bit iret  <-- NOT `iret`, which assembles to 16-bit iretw
```

Two gotchas we hit:
- **`iretd`, not `iret`.** In this assembler `iret` emits the 16-bit `iretw`,
  which pops a garbage frame and #GP-loops.
- The user code/stack pages must have the **User bit** set in the page tables,
  or ring-3 access faults. We set it across the identity map for now (so there
  is no memory isolation yet — that needs per-process page tables).

## How to test it

The boot demo spawns a launcher thread that drops into a ring-3 program which
only talks to the kernel via `int 0x80`:

```rust
// ring 3:
int 0x80  eax=1 (SYS_WRITE) ebx=fd ecx=msg edx=len   // print
int 0x80  eax=2 (SYS_EXIT)  ebx=0                     // terminate
```

Run (serial):

```bash
docker run -it --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 \
  qemu-system-i386 -m 128 -drive file=os-image.bin,format=raw,if=floppy -nographic
```

Expected:

```
Hello from ring 3 via syscall!
[exit code=0]
```

**What it means:** the kernel built an `iret` frame and dropped to ring 3; the
unprivileged program could not print directly — it issued `int 0x80`, the CPU
switched to the kernel stack from the TSS, the handler ran `sys_write`, and
control returned to the user program. `SYS_EXIT` then terminated the task and
the scheduler moved on. That is the complete user/kernel round trip — the
foundation for running real programs.

## What's next

The program here is a function compiled into the kernel. The final step is an
**ELF loader**: read a separately-compiled program file off the FAT12 disk,
load its segments into memory, and `enter_user_mode` at its entry point — plus
per-process page tables for real isolation. See [../ROADMAP.md](../ROADMAP.md).
