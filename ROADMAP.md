# Roadmap

Each stage builds on the previous. Complete in order.

---

## Stage 0 — Boot (current gaps)

| Feature | Why |
|---------|-----|
| BIOS E820 memory map | Detect actual usable RAM before kernel starts; required for proper PMM |
| Multiboot header | Let GRUB load the kernel instead of custom bootloader; standard, debuggable |
| Higher-half kernel | Map kernel to `0xC0000000` virtual; separates kernel/user address space cleanly |

---

## Stage 1 — Memory Management ✓ (mostly done)

Prerequisite for everything else.

| Feature | Details | Status |
|---------|---------|--------|
| E820 map | `boot/detect_memory.asm` + `src/mm/e820.rs` | ✓ |
| Physical Memory Manager (PMM) | 4KB-frame bitmap, `alloc_frame`/`free_frame`/`alloc_contiguous`, reserves <1MB + non-usable E820 regions (`src/mm/pmm.rs`) | ✓ |
| Paging | Page dir + tables, identity-map low 16MB, enable `CR0.PG` (`src/mm/paging.rs`) | ✓ |
| Page fault handler | ISR14 reports the fault address (`CR2`) to VGA + serial | ✓ |
| Kernel heap | First-fit free list with coalescing + `#[global_allocator]` so `alloc::{Box,Vec}` work (`src/mm/heap.rs`) | ✓ |
| Virtual memory areas | Track kernel memory regions (code, heap, stack) — needed before user space | todo |
| On-demand mapping | Map a frame to a virtual address on page fault instead of looping; grow the heap past 16MB | todo |

---

## Stage 2 — Multitasking (in progress)

| Feature | Details | Status |
|---------|---------|--------|
| Process control block (PCB) | `Task` { esp, stack, id, state } in `src/proc/task.rs` | ✓ |
| Context switch | `cpu/switch.asm` `switch_context` — pushf/pusha, swap ESP, popa/popf/ret | ✓ |
| Round-robin scheduler | `schedule()` picks next Ready task; `spawn(entry)` builds a bootstrap stack frame; threads end via a `task_exit` trampoline | ✓ |
| Timer preemption | IRQ0 (`timer_tick`) calls `schedule()` once enabled | ✓ |
| Kernel threads | Two demo threads interleave under preemption, then exit cleanly | ✓ |
| TSS (Task State Segment) | Needed for ring 0/3 switches (Stage 4). Not required for ring-0 threads | todo |
| `sleep(ms)` | Block a thread for N ms using PIT tick count (needs a blocked state) | todo |

---

## Stage 3 — Storage (in progress)

| Feature | Details | Status |
|---------|---------|--------|
| ATA/IDE PIO driver | LBA28 sector reads via ports `0x1F0`–`0x1F7`; floating-bus/timeout guard so a diskless boot doesn't hang (`src/drivers/ata.rs`) | ✓ (read) |
| ATA writes | LBA28 polled sector writes + cache flush (`write_sectors`) | ✓ |
| FAT12 write | Allocate clusters, write data + FAT chain + dir entry (`write_file`); shell `save` | ✓ |
| IRQ-driven transfers | Currently polled | todo |
| FAT12 read | Parse BPB + root dir + FAT chains; `read_file(8.3 name)` (`src/fs/fat12.rs`) | ✓ |
| Partition table parsing | Read MBR partition table to find a FAT partition | todo |
| FAT12/16 writes + readdir | create/append files, list directories | todo |
| VFS layer | Unified interface over storage drivers. `vfs_open()`, `vfs_read()`, `vfs_write()` | todo |

---

## Stage 4 — User Space (in progress)

| Feature | Details | Status |
|---------|---------|--------|
| Ring 3 privilege | User code/data GDT descriptors (DPL 3) + TSS; `enter_user_mode` iret into ring 3 (`src/cpu/gdt.rs`) | ✓ |
| Syscall interface | `int 0x80` gate (DPL 3) + dispatch: `sys_write`, `sys_exit` (`src/syscall`) | ✓ |
| ELF loader | Parse ELF32, load PT_LOAD segments, enter ring 3 at the entry; loads INIT.ELF off the FAT12 disk (`src/fs/elf.rs`, `user/program.asm`) | ✓ |
| Per-process page tables | Each program has its own address space (own CR3 + kernel stack); kernel is supervisor-only. Real isolation. | ✓ |
| Task reaping | Free a finished task's kernel stack, page tables, frames, and directory | ✓ |
| `fork` / `exec` | Clone address space; replace image with an ELF | todo |

---

## Stage 5 — Shell & Drivers

| Feature | Details |
|---------|---------|
| Lowercase keyboard | Track shift/caps lock state in keyboard driver. Currently only uppercase |
| PS/2 mouse | IRQ12. Read 3-byte packets from port `0x60`. Report `(dx, dy, buttons)` |
| Full shell | Command history (up/down), tab completion, pipes (`|`), redirection (`>`) |
| PC speaker | IRQ, PIT channel 2. `beep(freq, duration)` |
| RTC (real-time clock) | IRQ8. Read date/time from CMOS. `time` shell command |
| Serial as second console | Accept commands over COM1, not just echo |

---

## Stage 6 — Advanced (long-term)

| Feature | Notes |
|---------|-------|
| SMP (multi-core) | APIC, startup IPI, per-CPU scheduler queues |
| Signals | Unix-style `kill`, `SIGINT`, `SIGKILL` for user processes |
| Network stack | NE2000 / RTL8139 driver, ARP, IP, UDP, TCP |
| VESA / framebuffer | Graphical mode via BIOS VESA, pixel drawing primitives |
| UEFI bootloader | Replace BIOS MBR with UEFI PE binary, load via firmware |
| `libc` port | Port musl or newlib against the syscall interface |

---

## Dependency Graph

```
E820 map
  └─ PMM
       └─ Paging
            └─ Kernel heap (kfree)
                 └─ Multitasking
                      ├─ Ring 3 / Syscalls
                      │    └─ ELF loader → user programs
                      └─ VFS
                           └─ ATA/IDE → FAT → Shell
```

---

# Alternative track — Microkernel direction (optional)

The stages above grow this kernel the **monolithic, Unix-flavoured** way (like
Linux): drivers, filesystem, and the shell all live *inside* the kernel. There
is a different philosophy we can branch into instead — a **microkernel**, where
the kernel does almost nothing and everything else is a user-space program that
communicates by message passing. This is the QNX / seL4 / Fuchsia lineage.

This is a parallel direction, not a continuation — it reuses what we have
(tasks, ring 3, address-space isolation, the Blocked/Ready states) but changes
the *structure*.

## Why it matters / what it teaches

- **IPC (Inter-Process Communication)** is the heart of the system — the kernel
  becomes mostly a message router.
- **Drivers and the filesystem run in ring 3** as ordinary programs; clients
  request service by sending a message, not by calling into the kernel.
- **Fault isolation**: a crashed driver is one restartable server, not a dead
  kernel.
- **Capabilities**: explicit, unforgeable tokens decide who may talk to whom —
  a stronger security model than user/root.
- The **cost**: a file read becomes several context switches, so you feel why
  IPC speed is everything.

## Where this style is used in the real world

Safety-, real-time-, and security-critical systems (not desktops): **QNX** in
~200M+ cars, **L4/OKL4** in phone modems (billions shipped), **MINIX 3** inside
Intel's Management Engine, **seL4** in defense/aerospace, **INTEGRITY** in
aircraft avionics, **Fuchsia/Zircon** in Google Nest devices.

## Steps

| Step | Detail | Status |
|------|--------|--------|
| IPC syscalls | `send(dst_pid, msg)` / `recv() -> (src, msg)`; `recv` blocks (reuse the Blocked state), `send` wakes the receiver. Kernel copies the message and flips task states | todo |
| Echo server + client | A ring-3 server task that loops `recv` → reply; a client that sends a request and prints the reply — the minimal microkernel demo | todo |
| Move a driver out | Run the FAT12 / disk logic as a user-space **filesystem server**; the kernel only routes requests to it (the real lesson) | todo |
| Ports / capabilities | Address servers by unforgeable handles instead of raw pids; restrict who can message whom | todo |
| Server restart | Detect a crashed server and restart it without taking down the kernel (fault isolation) | todo |

## Relationship to the main track

```
Multitasking + Ring 3 + isolation  (already built)
        ├─ Monolithic track  → VFS, drivers-in-kernel, fork/exec, user shell
        └─ Microkernel track → IPC → user-space servers → capabilities
```

Both build on the same foundation; they diverge in *where the work lives*.
Pick one to explore, or do the microkernel track as a contained side-quest
(the IPC + echo-server milestone is ~150 lines).
