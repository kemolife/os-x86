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
kcore/src/cpu/gdt.rs       user code/data descriptors, the TSS, enter_user_mode()
cpu/interrupt.asm    isr128 stub for the int 0x80 syscall vector
kcore/src/cpu/idt.rs       set_idt_gate_flags() — install a DPL-3 (user-callable) gate
kcore/src/cpu/isr.rs       isr_handler routes int 0x80 to the syscall dispatcher
mono/src/syscall/mod.rs   dispatch + sys_write + sys_exit
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
saved `EAX`. Implemented calls: `write` (1), `exit` (2), `getpid` (3),
`yield` (4), `sleep` (5). `getpid` shows the round trip — the program reads its
pid from `EAX` and passes it to `exit`, so the kernel prints its own task id.

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
  or ring-3 access faults.

## Memory isolation (per-process address spaces)

Each program runs in its **own page directory**. The kernel's identity map is
**supervisor-only**, and every new address space *shares* those kernel page
tables (so syscalls/interrupts still work) but adds the program's own pages
(code, data, stack) at a high virtual address (`0x40000000`+), backed by fresh
physical frames mapped **USER**. The scheduler loads each task's `CR3` on
switch, and each task has its **own kernel stack** (TSS `esp0`), so:

- ring 3 can't read or write kernel memory (supervisor pages), and
- two programs can't see each other's memory (separate directories).

When a task exits it's **reaped**: its kernel stack, page tables, mapped frames,
and directory are freed (verified: running a program 5× leaves the free-frame
count unchanged).

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
  qemu-system-i386 -m 128 -drive file=os-image-mono.bin,format=raw,if=floppy -nographic
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

## Loading a real program from disk (ELF)

The built-in program above is compiled into the kernel. The **ELF loader**
(`mono/src/fs/elf.rs`) runs a *separately built* program off the FAT12 disk:

1. `fat12::read_file("INIT.ELF")` reads the file into a buffer.
2. Check the `\x7fELF` magic; read the entry point and the program-header table.
3. For each `PT_LOAD` segment, copy `filesz` bytes from the file to its virtual
   address and zero-fill the rest (`memsz - filesz`, the `.bss`).
4. `enter_user_mode(entry, stack)` — run it at ring 3.

The user program (`user/program.asm`) is a freestanding ELF32 linked at 4MB
(inside the user-accessible identity map). Build + run it:

```bash
docker run --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 bash -c '
  make mono >/dev/null 2>&1 && make user.elf
  dd if=/dev/zero of=/tmp/fat.img bs=512 count=2880 2>/dev/null
  mkfs.fat -F 12 /tmp/fat.img >/dev/null 2>&1
  mcopy -i /tmp/fat.img bin/user/init.elf ::INIT.ELF
  timeout 8 qemu-system-i386 -m 128 -boot a \
    -drive file=os-image-mono.bin,format=raw,if=floppy -drive file=/tmp/fat.img,format=raw,if=ide \
    -nographic -serial file:/tmp/r.log -monitor null 2>/dev/null || true
  tr -d "\000" < /tmp/r.log | grep -E "Hello from an ELF|exit code"'
```

Expected:

```
Hello from an ELF program on disk!
[exit code=42]
```

The exit code `42` (set inside the ELF, vs `0` in the built-in fallback) proves
the on-disk program actually ran.

## What's next

Real isolation: give each process its own page tables (so two programs can't
see each other's memory), then `fork`/`exec`. See [../ROADMAP.md](../ROADMAP.md).
