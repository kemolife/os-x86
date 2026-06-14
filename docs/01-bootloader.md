# 01 — Bootloader

## Why we need it

The BIOS only loads **one** 512-byte sector (the MBR) and runs it in 16-bit real
mode. Our kernel is ~120KB of 32-bit Rust. The bootloader's job is to bridge
that gap:

1. Load the rest of the kernel from disk into memory.
2. Tell the CPU about memory (the E820 map).
3. Set up the **GDT** (Global Descriptor Table) and switch the CPU into 32-bit
   **protected mode**.
4. Jump into the Rust kernel.

All of this must fit in 512 bytes (minus the 2-byte boot signature).

## File structure

```
boot/bootstrap.asm                  the boot sector — top-level orchestration
boot/detect_memory.asm              E820 memory map (int 0x15)        -> see 04-memory
boot/disk_load.asm                  read kernel sectors (int 0x13)
boot/global_descriptor_table.asm    the GDT data
boot/switch_to_protected_mode.asm   CR0 flip + far jump to 32-bit
boot/kernel_entry.asm               32-bit stub: call kernel_main
kernel.ld                           linker script (where the kernel lands)
i686-kernel.json                    custom Rust target (bare-metal 32-bit x86)
Makefile                            build recipe
```

## How it works, step by step

`bootstrap.asm` runs this sequence (real mode):

```asm
mov [BOOT_DRIVE], dl    ; BIOS left the boot drive number in DL — save it
mov bp, 0x9000          ; set up a stack at 0x9000
mov sp, bp
mov bx, MSG_REAL_MODE
call print_string       ; "Started in 16 - bit Real Mode"
call detect_memory      ; ask BIOS for the RAM map -> stored at 0x8000
call load_kernel        ; read the kernel from disk to 0x10000
call switch_to_pm       ; enter 32-bit protected mode (never returns)
```

### Loading the kernel — and the 0x10000 rule

```asm
load_kernel:
    mov ax, KERNEL_OFFSET >> 4   ; ES = 0x1000  -> physical 0x10000
    mov es, ax
    xor bx, bx                   ; ES:BX = 0x1000:0 = physical 0x10000
    mov cx, 250                  ; number of sectors to read (16-bit count)
    mov dl, [BOOT_DRIVE]
    call disk_load
```

The kernel loads at **physical `0x10000` (64KB)**, *not* `0x1000`. This matters:
the boot sector lives at `0x7C00`. A 120KB kernel loaded at `0x1000` would span
`0x1000`–`0x1F200` — straight over `0x7C00` — and would overwrite the very code
that's running the load, mid-load. Loading at `0x10000` keeps the kernel above
the boot sector. (The original C kernel was only 20KB, so it fit below `0x7C00`
and never hit this — the Rust kernel is bigger and exposed the bug.)

Because a 16-bit offset can't express `0x10000`, we address it via the **segment**
register `ES` (`0x1000 × 16 = 0x10000`); see segment:offset in
[00-x86-bios-and-assembly](00-x86-bios-and-assembly.md) §3.

### Reading sectors — `disk_load.asm` (LBA → CHS)

The BIOS disk routine (`int 0x13`, function `0x02`) addresses the floppy
geometrically: **CHS** = Cylinder / Head / Sector. But it has hard limits — a
single call can't cross a track (18 sectors) or a 64KB memory boundary. Reading
120KB in one call fails.

Our loader sidesteps all of that by reading **one sector at a time**, computing
the CHS triple from a running **LBA** (Linear Block Addressing — sector index
0,1,2,…) counter:

```
cylinder = lba / (18 * 2)      ; 18 sectors/track, 2 heads
remainder = lba % (18 * 2)
head     = remainder / 18
sector   = remainder % 18 + 1  ; sectors are 1-based
```

After each sector the destination segment `ES` advances by `0x20` (512 bytes =
0x20 "paragraphs" of 16 bytes) while the offset stays 0 — so no read ever
straddles a 64KB boundary. The count is held in `CX` (16-bit), so the kernel can
grow past the old 255-sector (`DH`, 8-bit) ceiling.

### The GDT and the mode switch

`global_descriptor_table.asm` defines three 8-byte entries: a mandatory null
descriptor, a code segment, and a data segment — both covering the full 4GB flat.
`switch_to_protected_mode.asm` then:

```asm
cli                     ; no interrupt handlers exist yet
lgdt [gdt_descriptor]   ; tell the CPU where the GDT is
mov eax, cr0
or  eax, 0x1            ; set Protection Enable bit
mov cr0, eax
jmp CODE_SEG:init_pm    ; far jump -> reloads CS, flushes the pipeline
```

The far jump is essential: it discards instructions the CPU already decoded as
16-bit and resumes decoding as 32-bit. `init_pm` then points all data segment
registers at the data descriptor, sets up a 32-bit stack at `0x90000`, and calls
`BEGIN_PM`, which calls `KERNEL_OFFSET` (`0x10000`) — the kernel.

### Into Rust — `kernel_entry.asm` and the build

`kernel_entry.asm` is a five-line stub at `0x10000`:

```asm
[bits 32]
_start:
    [extern kernel_main]
    call kernel_main
    jmp $
```

The kernel itself is Rust compiled to a static library and linked so that
`kernel_entry.o` sits first (at `0x10000`). The build (`Makefile`):
- `cargo build` with a custom target `i686-kernel.json` (bare-metal 32-bit, no
  OS, soft-float) plus `build-std=core,compiler_builtins,alloc` — Rust rebuilds
  its core libraries for our target.
- `i686-linux-gnu-ld -T kernel.ld` links the assembly stubs + the Rust `.a` into
  a flat binary.
- `cat boot/bootstrap.bin kernel.bin > os-image-mono.bin`, padded to a 1.44MB floppy.

## How to test it

A successful boot prints (on serial):

```
Started in 16 - bit Real Mode
Loading kernel into memory.
... (E820 map, then memory + thread output)
os-x86 ready. serial I/O active.
```

Run it:

```bash
docker run -it --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 \
  qemu-system-i386 -m 128 -drive file=os-image-mono.bin,format=raw,if=floppy -nographic
```

- Seeing **"Started in 16 - bit Real Mode"** = the BIOS found our MBR and jumped
  to it.
- Seeing **"Loading kernel into memory."** then later kernel output = the
  disk load, GDT, and protected-mode switch all worked, and Rust is running.
- A reboot loop (the BIOS banner repeating) would mean a **triple fault** — an
  unhandled error during the switch, usually a bad GDT or a load that clobbered
  the boot sector.

Inspect the assembled boot sector (must be exactly 512 bytes):

```bash
docker run --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 \
  bash -c "nasm boot/bootstrap.asm -f bin -o /tmp/b.bin && wc -c /tmp/b.bin"
```
