# 00 — x86, BIOS & Assembly Basics

This is the foundation. If the boot code looks like magic, read this first.

---

## 1. What happens when an x86 PC powers on

**x86** is the family of processors descended from the Intel 8086 (1978). A PC
compatible with it boots through a fixed ritual:

1. The **CPU** (Central Processing Unit — the processor) powers on in a
   primitive 16-bit mode called **real mode** (see §3). It begins executing at a
   fixed address wired into the chip (`0xFFFF0`), which lives in the firmware.
2. That firmware is the **BIOS** (Basic Input/Output System) — a program baked
   into a ROM chip on the motherboard. It tests the hardware, then looks for
   something to boot.
3. The BIOS reads the very first 512-byte **sector** of the boot disk into
   memory at address `0x7C00`. That sector is the **MBR** (Master Boot Record),
   also called the **boot sector**. If its last two bytes are `0x55 0xAA`
   (the "boot signature"), the BIOS jumps to `0x7C00` and starts running it.
4. From that instant, *our* code is in control. Everything after `0x7C00` is the
   bootloader we wrote (see [01-bootloader](01-bootloader.md)).

The BIOS also provides callable routines (via **interrupts**, §6) for reading
disks, printing characters, and querying memory — but only while in real mode.
Once we leave real mode, the BIOS is gone and we must drive hardware ourselves.

---

## 2. CPU registers — the CPU's variables

A **register** is a tiny, ultra-fast storage slot inside the CPU. All work
happens in registers. The 32-bit x86 ones we use:

**General-purpose (32-bit, prefixed `E` for "Extended"):**

| Register | Traditional role |
|----------|------------------|
| `EAX` | accumulator — math results, return values |
| `EBX` | base — a pointer/base address |
| `ECX` | counter — loop counts |
| `EDX` | data — I/O ports, extra math |
| `ESI` / `EDI` | source / destination index for copies |
| `EBP` | base pointer — bottom of the current stack frame |
| `ESP` | stack pointer — top of the stack |

Each 32-bit register has 16-bit and 8-bit sub-names: `EAX` → `AX` (low 16) →
`AL`/`AH` (low/high 8). Real-mode code uses the 16-bit names; protected-mode
code uses the `E` names.

**Segment registers** (16-bit): `CS` (code), `DS` (data), `SS` (stack), plus
`ES`, `FS`, `GS` (extra). Their meaning differs between real and protected mode
(§3, §4).

**Special:**
- `EIP` — Instruction Pointer: the address of the next instruction. You never
  write it directly; jumps and calls change it.
- `EFLAGS` — status bits: zero flag, carry flag, **IF** (Interrupt Flag — when
  0, hardware interrupts are ignored), etc.
- `CR0`, `CR2`, `CR3` — Control Registers. `CR0` bit 0 turns on protected mode,
  bit 31 turns on paging. `CR3` holds the page-directory address. `CR2` holds
  the address that caused a page fault.

---

## 3. Real mode — 16-bit, segment:offset addressing

At boot the CPU is in **real mode**: 16-bit registers, and a peculiar way of
forming a memory address. A 16-bit register can only count to 65,535, but early
PCs had up to 1MB of RAM (needs 20 bits). The fix: **segment:offset**.

```
physical address = segment × 16 + offset
```

So `ES:BX` with `ES = 0x1000`, `BX = 0` points at physical `0x1000 × 16 = 0x10000`.
This is why our bootloader sets a segment register to address memory above
64KB — a single 16-bit offset can't reach it (see the kernel-load code in
[01-bootloader](01-bootloader.md)).

Real mode has no memory protection: any code can touch any address. That's why
we leave it.

---

## 4. Protected mode — 32-bit, segments via the GDT

**Protected mode** gives flat 32-bit addresses (up to 4GB) and privilege
levels. But segment registers still exist — now they hold *selectors* that index
into the **GDT** (Global Descriptor Table). Each GDT entry ("descriptor")
describes a region of memory: its base, its limit, and permissions (code/data,
readable/writable, privilege ring 0–3).

Our kernel uses the simplest setup: two overlapping segments (code + data) that
both span the full 4GB starting at 0. So after the switch, an address is just a
plain number — segmentation is effectively "off", and we rely on **paging**
(see [04-memory](04-memory.md)) for protection instead.

Switching real → protected mode (done in `boot/switch_to_protected_mode.asm`):
1. Disable interrupts (`cli`) — we have no handlers yet.
2. Load the GDT address into the CPU (`lgdt`).
3. Set bit 0 of `CR0`.
4. Do a **far jump** to a 32-bit code segment. The jump reloads `CS` and flushes
   the CPU's instruction pipeline, which is mandatory — the CPU has already
   decoded the next instructions as 16-bit, and the far jump throws those away.

---

## 5. The stack

The **stack** is a region of memory that grows *downward* (toward lower
addresses). `ESP` points at the top. Two instructions use it:
- `push X` — write X at `[ESP]`, then `ESP -= 4`.
- `pop`    — read `[ESP]`, then `ESP += 4`.
- `call f` — push the return address, jump to `f`.
- `ret`    — pop the return address into `EIP`.

`pusha` / `popa` push / pop all eight general registers at once. The kernel uses
these heavily in interrupt handlers and the context switch
([05-multitasking](05-multitasking.md)) to save and restore CPU state.

---

## 6. Interrupts — the CPU's "stop what you're doing"

An **interrupt** makes the CPU pause, jump to a handler, then resume. Two kinds:
- **Exceptions** — the CPU itself raises them on errors (divide-by-zero, page
  fault, invalid opcode). Numbers 0–31.
- **Hardware interrupts (IRQs)** — devices raise them (timer tick, key press).
- **Software interrupts** — the `int N` instruction triggers one on purpose
  (the BIOS routines and our `int 0x80`-style calls work this way).

Where the CPU jumps for interrupt `N` is defined by a table: the **IVT**
(Interrupt Vector Table) in real mode, or the **IDT** (Interrupt Descriptor
Table) in protected mode. See [02-interrupts](02-interrupts.md).

---

## 7. Talking to hardware — I/O ports

Besides memory, x86 has a separate **I/O port** address space. Devices (timer,
keyboard, disk, serial) expose registers as port numbers. Two instructions:
- `out dx, al` — write the byte in `AL` to the port number in `DX`.
- `in al, dx`  — read a byte from the port in `DX` into `AL`.

Our Rust wrappers are `port_byte_out` / `port_byte_in` in `oscore/src/cpu/ports.rs`.
Example: the timer chip is programmed by writing to ports `0x40`/`0x43`; a disk
sector is read word-by-word from port `0x1F0`.

---

## 8. How our assembly files fit together

| File | Runs in | Job |
|------|---------|-----|
| `boot/bootstrap.asm` | real → protected | the boot sector: orchestrates everything |
| `boot/detect_memory.asm` | real mode | ask BIOS for the memory map (`int 0x15`) |
| `boot/disk_load.asm` | real mode | read kernel sectors (`int 0x13`) |
| `boot/global_descriptor_table.asm` | — | the GDT data |
| `boot/switch_to_protected_mode.asm` | real → protected | the `CR0` flip + far jump |
| `boot/kernel_entry.asm` | protected | tiny stub that calls Rust `kernel_main` |
| `cpu/interrupt.asm` | protected | low-level interrupt entry stubs |
| `cpu/switch.asm` | protected | the task context switch |

The instruction-by-instruction reasoning for each is in the per-feature docs.

---

## 9. Why Rust + a little assembly

Assembly is required for the parts that touch the CPU directly: the boot
sequence, the mode switch, interrupt entry, and the context switch. Everything
above that — drivers, memory management, the scheduler logic — is written in
**Rust** compiled as a freestanding `#![no_std]` static library (no operating
system underneath, no standard library). Rust gives memory safety for the 95% of
the kernel that doesn't need raw `asm`, while `unsafe` blocks mark the few places
that poke hardware. See the build details in [01-bootloader](01-bootloader.md).
