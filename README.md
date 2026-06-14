# os-x86

Bare-metal x86 OS built from scratch in **Rust**. Covers a NASM bootloader, protected mode, IDT/ISR, VGA text driver, PS/2 keyboard, serial I/O, and a `#![no_std]` Rust kernel. Learning project following the OS-development-from-scratch approach.

## Features

- 16-bit real mode bootloader (NASM)
- Robust LBA→CHS disk loader (handles kernels larger than 64KB / one track)
- GDT setup + switch to 32-bit protected mode
- Interrupt Descriptor Table (IDT) with all 32 CPU exception handlers
- PIC remapping and 16 IRQ handlers
- VGA text mode screen driver (scrolling, backspace)
- PS/2 keyboard driver with scancode → ASCII mapping
- COM1 serial driver (output + IRQ4 input)
- PIT timer (IRQ0)
- Memory management: E820 map, physical frame allocator (PMM), 32-bit paging
  with on-demand mapping, page-fault handler (reports CR2), kernel heap with a
  `#[global_allocator]` (so `alloc::{Box, Vec, ...}` work)
- Preemptive multitasking: ring-0 kernel threads, round-robin scheduler driven
  by the timer IRQ, assembly context switch, `sleep(ms)`
- Storage: ATA (IDE) PIO disk driver, read-only FAT12 filesystem (`read_file`)
- Minimal libc: `string`, `mem` (legacy bump allocator)
- Interactive kernel shell: `END` halts CPU, `PAGE` tests `kmalloc`

## Project Layout

```
boot/                   bootloader (NASM assembly)
  bootstrap.asm         MBR: loads kernel at 0x10000, switches to protected mode
  detect_memory.asm     BIOS E820 memory map -> 0x8000 (real mode)
  disk_load.asm         LBA→CHS sector-by-sector BIOS int 0x13 loader
  global_descriptor_table.asm  GDT definition
  switch_to_protected_mode.asm protected mode switch routine
  kernel_entry.asm      32-bit entry stub: calls kernel_main

cpu/
  interrupt.asm         Low-level ISR/IRQ stubs (pusha, iret) — only ASM kept

src/                    Rust kernel (#![no_std], staticlib)
  lib.rs                module tree + panic handler
  kernel.rs             kernel_main + user_input command handler
  cpu/
    ports.rs            inb/outb port I/O
    idt.rs              IDT gate setup, lidt
    isr.rs              ISR/IRQ install, handlers, PIC remap
    timer.rs            PIT timer init
  drivers/
    screen.rs           VGA text mode driver (kprint, scroll)
    keyboard.rs         PS/2 keyboard driver
    serial.rs           COM1 serial driver
  libc/
    mem.rs              memory_copy, memory_set, kmalloc (legacy bump allocator)
    string.rs           int_to_ascii, hex_to_ascii, strcmp, strlen, append
  mm/                   memory management
    e820.rs             parse the BIOS E820 map left at 0x8000
    pmm.rs              physical frame allocator (4KB-frame bitmap)
    paging.rs           page dir/tables, identity map, on-demand mapping
    heap.rs             first-fit free-list heap + #[global_allocator]
  proc/                 multitasking
    task.rs             task table, spawn(), round-robin schedule(), sleep()
  fs/                   filesystems
    fat12.rs            read-only FAT12 (read_file by 8.3 name)

kernel.ld               linker script (links kernel at 0x10000)
i686-kernel.json        custom bare-metal i686 target spec
Cargo.toml              staticlib crate, panic=abort

real_mode_routines/     Standalone 16-bit real mode examples (educational)
protected_mode_routines/ 32-bit print routine used by bootloader
```

## Toolchain

The kernel is compiled with the Rust **nightly** toolchain using `build-std`
(core + compiler_builtins) against a custom `i686-unknown-none` target, then
linked with `i686-linux-gnu-ld`. The provided Dockerfile bundles nightly Rust,
NASM, the i686 cross-linker, and QEMU — use it to avoid host setup.

## Quick Start (Docker — recommended)

Only requirement: [Docker](https://docs.docker.com/get-docker/) installed.

```bash
# 1. Build the Docker image (once). --platform is required on Apple Silicon.
docker build --platform=linux/amd64 -t os-x86 .

# 2. Compile inside the container — outputs os-image.bin to the current directory
docker run --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 make

# 3. Run in QEMU, serial on stdout
docker run --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 \
    qemu-system-i386 -drive file=os-image.bin,format=raw,if=floppy -nographic -serial stdio
```

Expected serial output:
```
Started in 16 - bit Real Mode
Loading kernel into memory.
os-x86 ready. serial I/O active.
```

The interactive prompt (`Type something...` / `>`) renders to the VGA text
console. To see it, run with a display instead of `-nographic`:

```bash
docker run -it --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 \
    qemu-system-i386 -drive file=os-image.bin,format=raw,if=floppy -display curses -serial stdio
```

### Debug inside Docker

```bash
# build the ELF (with symbols) alongside the image
docker run --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 make kernel.elf

# Terminal 1 — boot with GDB stub exposed
docker run -it --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os --network host os-x86 qemu-system-i386 -s -S -drive file=os-image.bin,format=raw,if=floppy -nographic

# Terminal 2 — attach GDB
docker run -it --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os --network host os-x86 \
    gdb -ex "target remote localhost:1234" -ex "symbol-file bin/kernel/kernel.elf"
```

### Clean

```bash
docker run --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 make clean
```

---

## Native Setup (optional)

Skip this if using Docker.

```bash
# Rust nightly + the bare-metal source components
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

# NASM + QEMU + i686 cross-linker
#   macOS:  brew install nasm qemu; brew install x86_64-elf-binutils (or i686 variant)
#   Linux:  sudo apt install nasm qemu-system-x86 binutils-i686-linux-gnu
```

If the linker binary differs on your host, edit `LD` at the top of the `Makefile`.
Then:

```bash
make        # build os-image.bin
make run    # build + launch QEMU
```

## How It Works

```
BIOS → MBR (bootstrap.asm, 0x7c00)
  → load_kernel: reads 250 sectors → kernel.bin at 0x10000
      (LBA→CHS sector-by-sector loader; ES advances per sector so a read
       never crosses a 64KB DMA boundary or a floppy track edge)
  → sets up GDT
  → switches to 32-bit protected mode
  → jumps to kernel_entry.asm (_start at 0x10000)
    → calls kernel_main() [Rust]
      → init_serial()        — COM1 8N1
      → mm::e820 / pmm / paging / heap — parse RAM, frame allocator, enable
                               paging, bring up the global-allocator heap
      → screen_init()        — configure VGA (80×25, 0xb8000)
      → mem_init(0x50000)    — legacy bump heap base (for the PAGE demo)
      → isr_install()        — register CPU exception handlers (0–31)
      → irq_install()        — remap PIC, register IRQ handlers (32–47), enable interrupts
      → init_timer(50)       — start PIT at 50 Hz
      → init_keyboard()      — register IRQ1 handler
      → keyboard_set_handler(user_input) — connect keyboard → kernel
      → waits for keyboard input via IRQ1 callback
        → user_input()       — handles "END" / "PAGE" commands
```

## Memory Map

| Address | Content |
|---------|---------|
| `0x00000` – `0x07BFF` | Free (real-mode stack grows down from `0x9000`) |
| `0x07C00` | Boot sector (MBR) |
| `0x08000` | E820 memory map (count + entries) |
| `0x10000` | Kernel loaded here (~120KB, ends ~`0x2F200`) |
| `kernel_end` | PMM frame bitmap |
| `0x50000` | Legacy bump heap (`mem.rs`, used by `PAGE`) |
| `0x90000` | Protected-mode stack top |
| `0xB8000` | VGA text framebuffer |
| `0x100000`+ | Extended RAM — PMM frames: page tables, then the 1MB kernel heap |

Paging identity-maps the low 16MB (physical == virtual), so every address
above is reachable unchanged. The kernel loads at `0x10000` (not `0x1000`) so a
kernel larger than ~26KB does not overwrite the boot sector at `0x7C00` while
`disk_load` is still running.

## Documentation

In-depth, beginner-friendly guides live in [`docs/`](docs/README.md) — one file
per subsystem (why it exists, how it's implemented, how to test it, what the
output means, with every abbreviation explained). Start with
[docs/00 — x86, BIOS & Assembly basics](docs/00-x86-bios-and-assembly.md).

## Roadmap

See [ROADMAP.md](ROADMAP.md) for planned stages: memory management, multitasking, storage, user space, and advanced features. Future subsystems land as new module folders under `src/` (`mm/`, `proc/`, `fs/`, `syscall/`).

## Known Limitations

- `kmalloc` is a bump allocator — no free, no paging
- No user space / privilege separation yet
- Keyboard only handles uppercase + basic punctuation (no shift state)
- Sector count in `bootstrap.asm` (`mov cx, 250`) must stay ≥ `ceil(kernel.bin / 512)`; 16-bit, room for ~1024 sectors before the stack at `0x90000`
