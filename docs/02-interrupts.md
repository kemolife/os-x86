# 02 — Interrupts (IDT / ISR / IRQ / PIC)

## Why we need it

An **interrupt** lets the CPU stop, run a handler, and resume. Without them the
kernel couldn't react to a key press or a timer tick, and couldn't catch errors
like a divide-by-zero or a page fault. Three sources:

- **Exceptions** — the CPU raises them on faults (numbers 0–31): divide error,
  invalid opcode, page fault (14), etc.
- **Hardware interrupts (IRQs — Interrupt ReQuests)** — devices raise them: the
  timer (IRQ0), keyboard (IRQ1), serial (IRQ4).
- **Software interrupts** — the `int N` instruction fires one deliberately.

## Key terms

- **IDT** (Interrupt Descriptor Table): an array of 256 entries. Entry `N` says
  "when interrupt `N` happens, jump here, at this privilege level." The CPU finds
  it via the `lidt` instruction.
- **ISR** (Interrupt Service Routine): the handler code that runs.
- **PIC** (Programmable Interrupt Controller, the Intel 8259): the chip that
  collects device IRQ lines and signals the CPU. There are two chained PICs
  (master + slave) giving 16 IRQ lines.

## File structure

```
cpu/interrupt.asm     low-level entry stubs (assembly) for all 48 vectors
src/cpu/idt.rs        IdtGate struct, set_idt_gate(), lidt
src/cpu/isr.rs        install handlers, the common Rust isr_handler/irq_handler,
                      PIC remap, the handler registry
```

## How it works

### 1. The entry stubs (`cpu/interrupt.asm`)

When the CPU takes interrupt `N` it jumps to that entry's address — but it does
*not* tell the handler which number `N` was, and some exceptions push an extra
"error code" while others don't. The stubs normalise this. Each stub pushes the
vector number (and a dummy error code if the CPU didn't push one), then jumps to
a shared routine that saves all registers (`pusha`), calls the Rust handler,
restores registers, cleans the stack, and returns with `iret`:

```asm
isr14:                ; page fault — CPU already pushed an error code
    push byte 14
    jmp isr_common_stub
isr3:                 ; breakpoint — no error code, push a dummy 0
    push byte 0
    push byte 3
    jmp isr_common_stub
```

This uniform layout means the Rust side always sees the same `Registers` struct.

### 2. The IDT (`src/cpu/idt.rs`)

`set_idt_gate(n, handler_addr)` fills entry `n`: the handler address (split into
low/high 16-bit halves), the kernel code selector `0x08`, and flags `0x8E`
(present, ring 0, 32-bit interrupt gate). `set_idt()` loads the table with
`lidt`.

### 3. Installing handlers (`src/cpu/isr.rs`)

- `isr_install()` points IDT entries 0–31 at the exception stubs.
- `irq_install()` first **remaps the PIC**. By default the PIC delivers IRQ0–15
  as interrupt vectors 8–15 — which collide with CPU exceptions. We reprogram it
  so IRQ0–15 arrive as vectors **32–47** instead. Then it fills entries 32–47 and
  runs `sti` to enable interrupts.

### 4. The Rust handlers

```rust
pub unsafe extern "C" fn isr_handler(r: *const Registers) {
    let int_no = (*r).int_no;
    if int_no == 14 { /* page fault: try to recover, else report CR2 */ }
    else { /* print "received interrupt: N" + the exception name */ }
}

pub unsafe extern "C" fn irq_handler(r: *const Registers) {
    // send End-Of-Interrupt to the PIC(s), then call the registered handler
    if int_no >= 40 { port_byte_out(0xA0, 0x20); } // slave EOI
    port_byte_out(0x20, 0x20);                      // master EOI
    if let Some(h) = INTERRUPT_HANDLERS[int_no] { h(r); }
}
```

Drivers register themselves with `register_interrupt_handler(vector, fn)` — e.g.
the timer registers on vector 32, the keyboard on 33. The **EOI** (End Of
Interrupt) tells the PIC we're done so it can deliver the next IRQ.

## How to test it

At boot `kernel_main` deliberately fires three software interrupts:

```rust
asm!("int 2");  // Non-Maskable Interrupt
asm!("int 3");  // Breakpoint
asm!("int 1");  // Debug
```

On the **VGA screen** (run with `-display curses`) you'll see:

```
received interrupt: 2
Non Maskable Interrupt
received interrupt: 3
Breakpoint
received interrupt: 1
Debug
```

**What it means:** each `int N` made the CPU look up IDT entry `N`, jump to the
stub, save state, call `isr_handler`, which printed the number and the
exception name from a table. Seeing all three proves the IDT is correctly wired
and that the stubs save/restore state so execution continues normally afterward.

The **timer** (IRQ0) and **keyboard** (IRQ1) firing continuously — the uptime
counter ticking and typed characters appearing — proves the IRQ path and PIC
remap work too. The **page-fault** handler (vector 14) is covered in
[04-memory](04-memory.md).
