# 04 — Memory Management

This is the foundation for multitasking and user programs. Four layers, each
built on the one before:

```
E820 map  ->  PMM (frame allocator)  ->  Paging  ->  Heap (Box/Vec)
```

## File structure

```
boot/detect_memory.asm   E820 query (real mode), writes the map to 0x8000
src/mm/e820.rs           read + print the map
src/mm/pmm.rs            physical frame allocator (4KB-frame bitmap)
src/mm/paging.rs         page directory/tables, identity map, on-demand mapping
src/mm/heap.rs           first-fit free-list heap + #[global_allocator]
kernel.ld                exports `kernel_end` (where the bitmap is placed)
```

---

## Layer 1 — E820 memory map

### Why

Before allocating RAM we must know which physical addresses are actually usable
(some are reserved for the BIOS, hardware, ACPI tables). Only the BIOS knows
this, and only in real mode — so we ask *before* switching to protected mode.

### How

`boot/detect_memory.asm` calls **`int 0x15`, function `0xE820`** in a loop. Each
call returns one 24-byte entry: `base` (8 bytes), `length` (8), `type` (4),
`attributes` (4). Type 1 = usable RAM; anything else is reserved. The routine
stores the entry count at physical `0x8000` and the entries at `0x8004`. After
the switch, `src/mm/e820.rs` reads them straight from those addresses.

### Result — what it means

```
E820 memory map:
  base=0x0 len=0x9FC00 type=1 (usable)      <- 639KB conventional RAM
  base=0x9FC00 len=0x400 type=2             <- reserved (EBDA)
  base=0xF0000 len=0x10000 type=2           <- reserved (BIOS ROM)
  base=0x100000 len=0x7EE0000 type=1 (usable)  <- ~127MB extended RAM
  base=0x7FE0000 len=0x20000 type=2         <- reserved (ACPI)
  base=0xFFFC0000 len=0x40000 type=2        <- reserved (firmware flash)
```

`base`/`len` are hex byte addresses/sizes. `type=1 (usable)` is RAM we may hand
out; `type=2` is off-limits. Total usable ≈ 639KB + 127MB on a 128MB QEMU
machine.

---

## Layer 2 — Physical Memory Manager (PMM)

### Why

We need to track, at all times, which chunks of physical RAM are free. The unit
is a **frame**: a 4KB-aligned block (4096 bytes), the natural size for paging.

### How

A **bitmap**: one bit per frame, 1 = used, 0 = free. The bitmap is placed right
after the kernel (the `kernel_end` symbol from `kernel.ld`). It is sized from the
highest usable E820 address. Initialisation:

1. Mark everything used.
2. Free the frames inside each usable E820 region.
3. Re-reserve everything below 1MB (BIOS, kernel, bitmap, legacy bump heap) — so
   frames are only ever handed out from extended RAM (≥ 1MB).

API: `alloc_frame()` (first free frame), `free_frame(addr)`,
`alloc_contiguous(n)` (n adjacent frames, used by the heap).

### Result — what it means

```
PMM: 32480 free / 32736 frames (4KB each)
```

- **32736 frames total** × 4KB ≈ **128MB** — matches the machine's RAM.
- **32480 free**, so 256 frames are reserved. 256 × 4KB = **1MB** — exactly the
  low memory we protect (BIOS + kernel + bitmap). Everything above 1MB is
  available.

---

## Layer 3 — Paging

### Why

**Paging** makes the CPU translate every **virtual** address through tables into
a **physical** one. This is the basis of memory protection and, later, giving
each process its own address space. It also lets us map memory on demand.

### How

A two-level structure: one **page directory** (1024 entries) → each entry points
to a **page table** (1024 entries) → each entry maps one 4KB page. We build
tables that **identity-map** the low 16MB (virtual address == physical address),
load the directory address into `CR3`, and set `CR0` bit 31 (the paging-enable
bit). The page tables themselves are allocated from the PMM — a nice closing of
the loop.

"Identity-mapped" means nothing's address changes — but the translation
machinery is now live, so we can start remapping selectively.

`map_page(virt, phys, flags)` maps a single page, allocating the page table on
demand and issuing `invlpg` to flush the stale entry from the **TLB**
(Translation Lookaside Buffer, the CPU's translation cache).

### On-demand paging (page-fault recovery)

If code touches an unmapped address the CPU raises a **page fault** (exception
14) and puts the bad address in register **`CR2`**. Our handler
(`paging::handle_fault`) checks whether the address is inside a designated
"grow" window (16–32MB); if so it grabs a fresh frame, maps it, and returns —
the faulting instruction simply retries and succeeds. Outside that window it
reports the fault instead.

### Result — what it means

```
paging: enabled (identity-mapped low 16MB)
```

The kernel keeps running normally after this line — which is the proof: paging
is on (`CR0.PG` set) and the identity map is correct, so every address the
kernel already used still resolves. A genuine fault prints, e.g.:

```
PAGE FAULT cr2=0xdeadbeef
```

— the value of `CR2`, i.e. exactly which address was touched.

---

## Layer 4 — Kernel heap

### Why

`alloc_frame` only gives whole 4KB frames. For arbitrary-sized allocations — and
to make Rust's `Box`, `Vec`, `String` work — we need a real heap with
`malloc`/`free` semantics.

### How

A **first-fit free list** over one contiguous, PMM-backed, identity-mapped 1MB
region. The region is a chain of blocks (each with a `{size, free, next}`
header). `kmalloc` walks the list for the first free block big enough, splitting
off the remainder. `kfree` marks a block free and **coalesces** it with adjacent
free neighbours so the heap doesn't fragment. Every block is 16-byte aligned.

It is wired as Rust's `#[global_allocator]`, so `alloc::{Box, Vec, ...}` work
throughout the kernel.

### Result — what it means

```
heap: 1048560 free / 0 used bytes, 1 blocks
```

- **1048560 bytes** ≈ 1MB (1MB minus a 16-byte block header) — the heap's size.
- **0 used / 1 block** — nothing allocated yet: one big free block.

After a `Box` + a growing `Vec`, stats become e.g. `... 80 used bytes, 4 blocks`;
after they drop, it returns to `1 blocks` — proving allocation, splitting, free,
and coalescing all work.

## How to test memory management

Plain serial boot shows all four layers:

```bash
docker run -it --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 \
  qemu-system-i386 -m 128 -drive file=os-image.bin,format=raw,if=floppy -nographic
```

Expected:

```
E820 memory map: ...
PMM: 32480 free / 32736 frames (4KB each)
paging: enabled (identity-mapped low 16MB)
heap: 1048560 free / 0 used bytes, 1 blocks
```

Try a different RAM size — change `-m 128` to `-m 64` and the PMM frame count
halves (`16352` total), confirming it reads the real E820 map rather than
hard-coding 128MB.
