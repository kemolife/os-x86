# os-x86

Bare-metal x86 OS built from scratch. Covers bootloader, protected mode, IDT/ISR, VGA text driver, PS/2 keyboard, and a minimal C runtime. Learning project following the OS development from scratch approach.

## Features

- 16-bit real mode bootloader (NASM)
- GDT setup + switch to 32-bit protected mode
- Interrupt Descriptor Table (IDT) with all 32 CPU exception handlers
- PIC remapping and 16 IRQ handlers
- VGA text mode screen driver (scrolling, backspace)
- PS/2 keyboard driver with scancode → ASCII mapping
- Minimal libc: `string`, `mem` (bump allocator)
- PIT timer (IRQ0)
- Interactive kernel shell: `END` halts CPU, `PAGE` tests `kmalloc`

## Project Layout

```
boot/                   bootloader (NASM assembly)
  bootstrap.asm         MBR: loads kernel, switches to protected mode
  disk_load.asm         BIOS int 0x13 disk read
  global_descriptor_table.asm  GDT definition
  switch_to_protected_mode.asm protected mode switch routine
  kernel_entry.asm      C kernel entry point stub

cpu/                    CPU subsystem (C + ASM)
  idt.c/h               IDT gate setup, lidt
  isr.c/h               ISR/IRQ install, handlers, PIC remap
  interrupt.asm         Low-level ISR/IRQ stubs (pusha, iret)
  ports.c/h             inb/outb port I/O
  timer.c/h             PIT timer init
  type.h                Common types and bit macros

drivers/
  screen.c/h            VGA text mode driver (kprint, scroll)
  keyboard.c/h          PS/2 keyboard driver

kernel/
  kernel.c              kernel_main + user_input command handler
  kernel.h              user_input declaration

libc/
  string.c/h            int_to_ascii, hex_to_ascii, strcmp, strlen, append
  mem.c/h               memory_copy, memory_set, kmalloc (bump allocator)
  function.h            UNUSED() macro

real_mode_routines/     Standalone 16-bit real mode examples (educational)
protected_mode_routines/ 32-bit print routine used by bootloader
kernel-examples/        Alternate kernel examples (screen, ports, interrupts)
```

## Quick Start (Docker — recommended)

Only requirement: [Docker](https://docs.docker.com/get-docker/) installed.

```bash
# 1. Build the Docker image (once)
docker build -t os-x86 .

# 2. Copy and use the pre-configured Makefile
cp makefile-example Makefile

# 3. Compile inside the container — outputs os-image.bin to current directory
docker run --rm -v "$(pwd)":/os os-x86 make

# 4. Run in QEMU (VGA text mode in terminal, serial on stdout)
docker run -it --rm -v "$(pwd)":/os os-x86 \
    qemu-system-i386 -display curses -serial stdio -fda os-image.bin
```

Press `Alt+2` to open QEMU monitor, `q` + Enter to quit.

Expected output:
```
Started in 16-bit Real Mode
Loading kernel into memory.
Successfully landed in 32-bit Protected Mode
received interrupt: 2
Non Maskable Interrupt
...
Type END to halt the CPU or PAGE to request a kmalloc()
>
```

### Debug inside Docker

```bash
# Terminal 1 — boot with GDB stub exposed
docker run -it --rm -v "$(pwd)":/os --network host os-x86 \
    qemu-system-i386 -s -S -display curses -fda os-image.bin

# Terminal 2 — attach GDB
docker run -it --rm -v "$(pwd)":/os --network host os-x86 \
    gdb -ex "target remote localhost:1234" -ex "symbol-file bin/kernel/kernel.elf"
```

### Clean

```bash
docker run --rm -v "$(pwd)":/os os-x86 make clean
```

---

## Native Setup (optional)

Skip this if using Docker.

### macOS (Homebrew)

```bash
brew install nasm qemu
brew tap nativeos/i386-elf-toolchain
brew install i386-elf-binutils i386-elf-gcc
```

Edit the three toolchain paths in `Makefile` after `cp makefile-example Makefile`:

```makefile
CC  = i386-elf-gcc
GDB = i386-elf-gdb
LD  = i386-elf-ld
```

Then `make && make run`.

### Linux (Debian/Ubuntu)

```bash
sudo apt install nasm qemu-system-x86 gcc gcc-multilib binutils-i686-linux-gnu
cp makefile-example Makefile   # default paths already work
make && make run
```

## How It Works

```
BIOS → MBR (bootstrap.asm, 0x7c00)
  → loads 15 sectors from disk → kernel.bin at 0x1000
  → sets up GDT
  → switches to 32-bit protected mode
  → jumps to kernel_entry.asm
    → calls kernel_main() [C]
      → screen_init()         — configure VGA (80×25, 0xb8000)
      → mem_init(0x10000)     — set heap base
      → isr_install()         — register CPU exception handlers (0–31)
      → irq_install()         — remap PIC, register IRQ handlers (32–47), enable interrupts
      → init_timer(50)        — start PIT at 50 Hz
      → init_keyboard()       — register IRQ1 handler
      → keyboard_set_handler(user_input) — connect keyboard → kernel
      → waits for keyboard input via IRQ1 callback
        → user_input()        — handles "END" / "PAGE" commands
```

## Memory Map

| Address | Content |
|---------|---------|
| `0x0000` – `0x7BFF` | Free (stack grows down from `0x9000`) |
| `0x7C00` | Boot sector (MBR) |
| `0x1000` | Kernel loaded here |
| `0x10000` | Heap start (`kmalloc` bump pointer) |
| `0xB8000` | VGA text framebuffer |

## Roadmap

See [ROADMAP.md](ROADMAP.md) for planned stages: memory management, multitasking, storage, user space, and advanced features.

## Known Limitations

- `kmalloc` is a bump allocator — no free, no paging
- No user space / privilege separation yet
- Keyboard only handles uppercase + basic punctuation (no shift state)
