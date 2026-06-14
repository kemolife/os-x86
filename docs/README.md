# os-x86 — Developer Guide

Deep-dive documentation for the os-x86 kernel. Each file covers one subsystem:
**why** it exists, **how** it is implemented, **how to test** it, and **what
the output means**. Every abbreviation is spelled out the first time it appears.

Read them in order — each builds on the previous.

| # | File | Topic |
|---|------|-------|
| 00 | [x86, BIOS & Assembly basics](00-x86-bios-and-assembly.md) | How a PC starts, CPU registers, real vs protected mode — the foundation for everything else |
| 01 | [Bootloader](01-bootloader.md) | The 512-byte boot sector: load the kernel, set up the GDT, enter 32-bit mode |
| 02 | [Interrupts (IDT / ISR / IRQ / PIC)](02-interrupts.md) | Handling CPU exceptions and hardware interrupts |
| 03 | [Drivers (VGA / keyboard / serial / timer)](03-drivers.md) | Talking to hardware |
| 04 | [Memory management](04-memory.md) | E820 map, frame allocator, paging, kernel heap |
| 05 | [Multitasking](05-multitasking.md) | Context switch, scheduler, threads, `sleep` |
| 06 | [Storage (ATA)](06-storage.md) | Reading disk sectors |
| 07 | [Filesystem (FAT12)](07-filesystem.md) | Reading named files |
| 08 | [User space (ring 3 + syscalls)](08-userspace.md) | Running unprivileged code |
| 09 | [Kernel command shell](09-shell.md) | The interactive terminal |

## Build and run

Everything runs through Docker (the host toolchain isn't needed). On Apple
Silicon the `--platform=linux/amd64` flag is required.

```bash
# build the toolchain image once
docker build --platform=linux/amd64 -t os-x86 .

# build the OS image (os-image-mono.bin)
docker run --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 make mono

# run — serial output (boot log, memory, threads) goes to your terminal
docker run -it --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 \
  qemu-system-i386 -m 128 -drive file=os-image-mono.bin,format=raw,if=floppy -nographic
```

Quit QEMU: `Ctrl-A` then `x`.

## Two output channels — don't confuse them

The kernel prints to two different places:

| Channel | What appears there | How to see it |
|---------|--------------------|---------------|
| **Serial** (COM1) | boot log, memory-manager stats, thread `[A]/[B]` output, ATA probe | `-nographic` (serial → terminal) |
| **VGA** text screen | the interactive `>` shell prompt, `END` / `PAGE` commands, uptime counter | `-display curses` |

Most subsystem results in this guide are on **serial**, so the plain
`-nographic` run shows them.

## Abbreviation quick-reference

| Short | Full | One line |
|-------|------|----------|
| BIOS | Basic Input/Output System | firmware that runs first at power-on |
| MBR | Master Boot Record | the first 512-byte sector; the boot sector |
| CPU | Central Processing Unit | the processor |
| GDT | Global Descriptor Table | defines memory segments in protected mode |
| IDT | Interrupt Descriptor Table | maps interrupt numbers to handler code |
| ISR | Interrupt Service Routine | code that runs on a CPU exception |
| IRQ | Interrupt ReQuest | a hardware interrupt line |
| PIC | Programmable Interrupt Controller | chip that routes IRQs to the CPU |
| PIT | Programmable Interval Timer | the hardware timer chip |
| VGA | Video Graphics Array | the text/graphics display hardware |
| PS/2 | Personal System/2 | the keyboard/mouse port standard |
| PMM | Physical Memory Manager | tracks which 4KB RAM frames are free |
| E820 | (the BIOS call number `0xE820`) | reports the memory map |
| LBA | Linear Block Addressing | numbering disk sectors 0,1,2,… |
| CHS | Cylinder/Head/Sector | the old geometric disk addressing |
| ATA | AT Attachment (a.k.a. IDE) | the hard-disk interface |
| PIO | Programmed I/O | moving data through ports, no DMA |
| DMA | Direct Memory Access | hardware copying RAM without the CPU |
| TLB | Translation Lookaside Buffer | the CPU's page-translation cache |
