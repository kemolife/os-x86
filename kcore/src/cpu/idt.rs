const IDT_ENTRIES: usize = 256;
const KERNEL_CS: u16 = 0x08;

#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct IdtGate {
    low_offset: u16,
    sel: u16,
    always0: u8,
    flags: u8,
    high_offset: u16,
}

#[repr(C, packed)]
struct IdtRegister {
    limit: u16,
    base: u32,
}

static mut IDT: [IdtGate; IDT_ENTRIES] = [IdtGate {
    low_offset: 0,
    sel: 0,
    always0: 0,
    flags: 0,
    high_offset: 0,
}; IDT_ENTRIES];

static mut IDT_REG: IdtRegister = IdtRegister { limit: 0, base: 0 };

pub unsafe fn set_idt_gate(n: usize, handler: u32) {
    set_idt_gate_flags(n, handler, 0x8E); // present, ring 0, 32-bit interrupt gate
}

/// Like `set_idt_gate` but with explicit flags — e.g. 0xEE for a ring-3-callable
/// gate (DPL=3), needed for the `int 0x80` syscall vector.
pub unsafe fn set_idt_gate_flags(n: usize, handler: u32, flags: u8) {
    let idt = &raw mut IDT; // raw pointer to the static — no reference created
    (*idt)[n].low_offset = (handler & 0xFFFF) as u16;
    (*idt)[n].sel = KERNEL_CS;
    (*idt)[n].always0 = 0;
    (*idt)[n].flags = flags;
    (*idt)[n].high_offset = ((handler >> 16) & 0xFFFF) as u16;
}

pub unsafe fn set_idt() {
    let idt_reg = &raw mut IDT_REG;
    (*idt_reg).base = (&raw const IDT) as u32;
    (*idt_reg).limit = (IDT_ENTRIES * core::mem::size_of::<IdtGate>() - 1) as u16;
    core::arch::asm!("lidt [{0}]", in(reg) idt_reg as *const IdtRegister, options(nostack));
}
