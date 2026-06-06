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

## Stage 1 — Memory Management

Prerequisite for everything else. Current `kmalloc` is a bump allocator with no `free`.

| Feature | Details |
|---------|---------|
| Physical Memory Manager (PMM) | Bitmap over all RAM frames (4 KB each). `pmm_alloc_frame()` / `pmm_free_frame()`. Uses E820 map to mark reserved regions |
| Paging | Set up page directory + page tables. Enable via `CR0.PG`. Identity-map first 4 MB for kernel |
| Page fault handler | ISR14 currently just prints and hangs. Should show fault address (`CR2`), error code, stack trace |
| Kernel heap | `kmalloc` / `kfree` built on paging. Free-list or slab allocator. Replace bump allocator |
| Virtual memory areas | Track kernel memory regions (code, heap, stack) — needed before user space |

---

## Stage 2 — Multitasking

| Feature | Details |
|---------|---------|
| TSS (Task State Segment) | x86 requires TSS for ring 0/3 privilege switches. One TSS entry in GDT |
| Process control block (PCB) | Struct holding registers, stack pointer, page directory, state, PID |
| Context switch | Assembly routine: save caller registers, swap `ESP`/`EIP`, restore next task |
| Round-robin scheduler | Timer IRQ0 triggers scheduler. Simplest: fixed quantum, circular queue |
| Kernel threads | Multiple execution contexts inside kernel before tackling user space |
| `sleep(ms)` | Block current thread for N milliseconds using PIT tick count |

---

## Stage 3 — Storage

| Feature | Details |
|---------|---------|
| ATA/IDE PIO driver | Read/write 512-byte sectors via ports `0x1F0`–`0x1F7`. IRQ14 (primary) / IRQ15 (secondary) |
| Partition table parsing | Read MBR partition table to find FAT partition |
| FAT12 / FAT16 | Parse FAT filesystem on the boot floppy/disk image. `open`, `read`, `readdir` |
| VFS layer | Unified interface over storage drivers. `vfs_open()`, `vfs_read()`, `vfs_write()` |

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
