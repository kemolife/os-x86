# 03 — Drivers (VGA / Keyboard / Serial / Timer)

## Why we need them

Once in protected mode the BIOS is gone. To show text, read keys, print debug
output, and keep time, the kernel must drive the hardware directly through
memory-mapped regions and **I/O ports** (see
[00-x86-bios-and-assembly](00-x86-bios-and-assembly.md) §7).

## File structure

```
kcore/src/drivers/screen.rs     VGA text-mode output (kprint)
kcore/src/drivers/keyboard.rs   PS/2 keyboard input (IRQ1)
kcore/src/drivers/serial.rs     COM1 serial port (output + IRQ4 input)
kcore/src/cpu/timer.rs          PIT timer (IRQ0) + tick counter
kcore/src/cpu/ports.rs          in/out port instructions (port_byte_in/out, word_in/out)
```

---

## VGA text screen — `screen.rs`

**VGA** = Video Graphics Array. In text mode the screen is an 80×25 grid mapped
into memory at **`0xB8000`**. Each cell is 2 bytes: an ASCII character + an
attribute byte (foreground/background colour). Writing a character is just
writing two bytes into that array; the hardware displays it.

`kprint(msg)` walks a null-terminated byte string, writing each char and
advancing the cursor; it scrolls when the cursor passes row 25 and handles
backspace. The hardware cursor position is set through ports `0x3D4`/`0x3D5`.

This is where the interactive **`>` shell prompt** appears — visible with
`-display curses`.

---

## PS/2 keyboard — `keyboard.rs`

**PS/2** is the keyboard port standard. A key press raises **IRQ1**; the handler
reads a **scancode** (a number identifying the key) from port `0x60` and maps it
through a table to an ASCII character. The driver buffers characters until Enter,
then calls a registered callback (`user_input` in `kernel.rs`) with the line.

That callback is the shell: it recognises `END` (halt the CPU) and `PAGE` (test
the allocator). Only the keyboard path is interactive — and only on the VGA
console, because keystrokes arrive as PS/2 IRQs, not over serial.

---

## COM1 serial port — `serial.rs`

**Serial** (the COM1 port, I/O base `0x3F8`) is a simple byte-at-a-time link.
It's the kernel's main debug channel: under QEMU the serial output is redirected
to your terminal, so *all* the boot/memory/thread logs you see come through
here.

`init_serial()` programs the line settings (8 data bits, no parity, 1 stop bit —
"8N1"). `serial_write(byte)` polls the line-status register until the transmit
buffer is empty, then writes the byte. Incoming bytes raise **IRQ4**.

```rust
pub unsafe fn serial_write(c: u8) {
    while !tx_empty() {}          // wait until the UART can accept a byte
    port_byte_out(0x3F8, c);
}
```

---

## PIT timer — `timer.rs`

**PIT** = Programmable Interval Timer (the Intel 8253/8254). It fires **IRQ0** at
a programmed frequency. We set 50 Hz (50 ticks/second):

```rust
let divisor = 1193180 / freq;   // the PIT runs at ~1.193 MHz
port_byte_out(0x43, 0x36);      // command: channel 0, rate generator
port_byte_out(0x40, divisor low byte);
port_byte_out(0x40, divisor high byte);
```

Every tick increments a counter (`ticks()`), updates the uptime display in the
screen's top-right corner, and — once enabled — drives the **scheduler** and
wakes sleeping tasks (see [05-multitasking](05-multitasking.md)).

## How to test the drivers

Run with both channels:

```bash
docker run -it --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 \
  qemu-system-i386 -m 128 -drive file=os-image-mono.bin,format=raw,if=floppy \
  -display curses -serial stdio
```

| You should see | Proves |
|----------------|--------|
| `os-x86 ready. serial I/O active.` in the terminal | **serial** output works |
| `UP:Ns` ticking up in the screen's top-right (curses) | **PIT timer** IRQ0 fires |
| the `>` prompt and your typed text on the curses screen | **VGA** + **PS/2 keyboard** work |
| typing `END` halts; `PAGE` prints an address | the keyboard → shell callback works |
