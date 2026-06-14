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
| ATA writes / IRQ-driven | Currently polled reads only | todo |
| FAT12 read | Parse BPB + root dir + FAT chains; `read_file(8.3 name)` (`src/fs/fat12.rs`) | ✓ |
| Partition table parsing | Read MBR partition table to find a FAT partition | todo |
| FAT12/16 writes + readdir | create/append files, list directories | todo |
| VFS layer | Unified interface over storage drivers. `vfs_open()`, `vfs_read()`, `vfs_write()` | todo |

---

## Stage 4 — User Space

| Feature | Details |
|---------|---------|
| Ring 3 privilege | Add user-mode code/data segments to GDT (DPL=3). `iret` into ring 3 |
| Syscall interface | `int 0x80` dispatch table. Start with `write`, `exit`, `read` |
| ELF loader | Parse ELF32 binary, load segments into user address space, jump to entry point |
| User stack | Allocate per-process user stack in low virtual memory |
| `fork` / `exec` | Clone address space (`fork`), replace image with ELF (`exec`) |

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
