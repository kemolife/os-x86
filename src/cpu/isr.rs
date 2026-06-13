use crate::cpu::idt::{set_idt_gate, set_idt};
use crate::cpu::ports::port_byte_out;
use crate::drivers::screen::kprint;
use crate::libc::string::{int_to_ascii, hex_to_ascii};

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Registers {
    pub ds: u32,
    pub edi: u32, pub esi: u32, pub ebp: u32, pub useless: u32,
    pub ebx: u32, pub edx: u32, pub ecx: u32, pub eax: u32,
    pub int_no: u32, pub err_code: u32,
    pub eip: u32, pub cs: u32, pub eflags: u32, pub esp: u32, pub ss: u32,
}

pub type IsrT = fn(*const Registers);

static mut INTERRUPT_HANDLERS: [Option<IsrT>; 256] = [None; 256];

extern "C" {
    fn isr0();  fn isr1();  fn isr2();  fn isr3();  fn isr4();  fn isr5();  fn isr6();  fn isr7();
    fn isr8();  fn isr9();  fn isr10(); fn isr11(); fn isr12(); fn isr13(); fn isr14(); fn isr15();
    fn isr16(); fn isr17(); fn isr18(); fn isr19(); fn isr20(); fn isr21(); fn isr22(); fn isr23();
    fn isr24(); fn isr25(); fn isr26(); fn isr27(); fn isr28(); fn isr29(); fn isr30(); fn isr31();
    fn irq0();  fn irq1();  fn irq2();  fn irq3();  fn irq4();  fn irq5();  fn irq6();  fn irq7();
    fn irq8();  fn irq9();  fn irq10(); fn irq11(); fn irq12(); fn irq13(); fn irq14(); fn irq15();
}

pub unsafe fn isr_install() {
    set_idt_gate(0,  isr0 as *const ()  as usize as u32); set_idt_gate(1,  isr1 as *const ()  as usize as u32);
    set_idt_gate(2,  isr2 as *const ()  as usize as u32); set_idt_gate(3,  isr3 as *const ()  as usize as u32);
    set_idt_gate(4,  isr4 as *const ()  as usize as u32); set_idt_gate(5,  isr5 as *const ()  as usize as u32);
    set_idt_gate(6,  isr6 as *const ()  as usize as u32); set_idt_gate(7,  isr7 as *const ()  as usize as u32);
    set_idt_gate(8,  isr8 as *const ()  as usize as u32); set_idt_gate(9,  isr9 as *const ()  as usize as u32);
    set_idt_gate(10, isr10 as *const () as usize as u32); set_idt_gate(11, isr11 as *const () as usize as u32);
    set_idt_gate(12, isr12 as *const () as usize as u32); set_idt_gate(13, isr13 as *const () as usize as u32);
    set_idt_gate(14, isr14 as *const () as usize as u32); set_idt_gate(15, isr15 as *const () as usize as u32);
    set_idt_gate(16, isr16 as *const () as usize as u32); set_idt_gate(17, isr17 as *const () as usize as u32);
    set_idt_gate(18, isr18 as *const () as usize as u32); set_idt_gate(19, isr19 as *const () as usize as u32);
    set_idt_gate(20, isr20 as *const () as usize as u32); set_idt_gate(21, isr21 as *const () as usize as u32);
    set_idt_gate(22, isr22 as *const () as usize as u32); set_idt_gate(23, isr23 as *const () as usize as u32);
    set_idt_gate(24, isr24 as *const () as usize as u32); set_idt_gate(25, isr25 as *const () as usize as u32);
    set_idt_gate(26, isr26 as *const () as usize as u32); set_idt_gate(27, isr27 as *const () as usize as u32);
    set_idt_gate(28, isr28 as *const () as usize as u32); set_idt_gate(29, isr29 as *const () as usize as u32);
    set_idt_gate(30, isr30 as *const () as usize as u32); set_idt_gate(31, isr31 as *const () as usize as u32);
}

pub unsafe fn pic_remap() {
    port_byte_out(0x20, 0x11); port_byte_out(0xA0, 0x11);
    port_byte_out(0x21, 0x20); port_byte_out(0xA1, 0x28);
    port_byte_out(0x21, 0x04); port_byte_out(0xA1, 0x02);
    port_byte_out(0x21, 0x01); port_byte_out(0xA1, 0x01);
    port_byte_out(0x21, 0x00); port_byte_out(0xA1, 0x00);
}

pub unsafe fn irq_install() {
    pic_remap();
    set_idt_gate(32, irq0 as *const ()  as usize as u32); set_idt_gate(33, irq1 as *const ()  as usize as u32);
    set_idt_gate(34, irq2 as *const ()  as usize as u32); set_idt_gate(35, irq3 as *const ()  as usize as u32);
    set_idt_gate(36, irq4 as *const ()  as usize as u32); set_idt_gate(37, irq5 as *const ()  as usize as u32);
    set_idt_gate(38, irq6 as *const ()  as usize as u32); set_idt_gate(39, irq7 as *const ()  as usize as u32);
    set_idt_gate(40, irq8 as *const ()  as usize as u32); set_idt_gate(41, irq9 as *const ()  as usize as u32);
    set_idt_gate(42, irq10 as *const () as usize as u32); set_idt_gate(43, irq11 as *const () as usize as u32);
    set_idt_gate(44, irq12 as *const () as usize as u32); set_idt_gate(45, irq13 as *const () as usize as u32);
    set_idt_gate(46, irq14 as *const () as usize as u32); set_idt_gate(47, irq15 as *const () as usize as u32);
    set_idt();
    core::arch::asm!("sti", options(nostack, nomem));
}

pub unsafe fn register_interrupt_handler(n: u8, handler: IsrT) {
    INTERRUPT_HANDLERS[n as usize] = Some(handler);
}

static EXCEPTION_MESSAGES: [&[u8]; 32] = [
    b"Division By Zero\0",
    b"Debug\0",
    b"Non Maskable Interrupt\0",
    b"Breakpoint\0",
    b"Into Detected Overflow\0",
    b"Out of Bounds\0",
    b"Invalid Opcode\0",
    b"No Coprocessor\0",
    b"Double Fault\0",
    b"Coprocessor Segment Overrun\0",
    b"Bad TSS\0",
    b"Segment Not Present\0",
    b"Stack Fault\0",
    b"General Protection Fault\0",
    b"Page Fault\0",
    b"Unknown Interrupt\0",
    b"Coprocessor Fault\0",
    b"Alignment Check\0",
    b"Machine Check\0",
    b"Reserved\0",
    b"Reserved\0",
    b"Reserved\0",
    b"Reserved\0",
    b"Reserved\0",
    b"Reserved\0",
    b"Reserved\0",
    b"Reserved\0",
    b"Reserved\0",
    b"Reserved\0",
    b"Reserved\0",
    b"Reserved\0",
    b"Reserved\0",
];

#[no_mangle]
pub unsafe extern "C" fn isr_handler(r: *const Registers) {
    let int_no = (*r).int_no as usize;

    // Page fault: try to recover (demand paging) before reporting anything.
    if int_no == 14 {
        let cr2 = crate::mm::paging::fault_address();
        if crate::mm::paging::handle_fault(cr2) {
            return; // mapped on demand; retry the faulting instruction
        }
        let mut addr = [0u8; 16];
        hex_to_ascii(cr2 as i32, addr.as_mut_ptr());
        kprint(b"PAGE FAULT (CR2): \0".as_ptr());
        kprint(addr.as_ptr());
        kprint(b"\n\0".as_ptr());
        crate::drivers::serial::serial_write_str(b"PAGE FAULT cr2=\0".as_ptr());
        crate::drivers::serial::serial_write_str(addr.as_ptr());
        crate::drivers::serial::serial_write_str(b"\n\0".as_ptr());
        return;
    }

    kprint(b"received interrupt: \0".as_ptr());
    let mut s = [0u8; 4];
    int_to_ascii(int_no as i32, s.as_mut_ptr());
    kprint(s.as_ptr());
    kprint(b"\n\0".as_ptr());
    if int_no < 32 {
        kprint(EXCEPTION_MESSAGES[int_no].as_ptr());
        kprint(b"\n\0".as_ptr());
    }
}

#[no_mangle]
pub unsafe extern "C" fn irq_handler(r: *const Registers) {
    let int_no = (*r).int_no as usize;
    if int_no >= 40 {
        port_byte_out(0xA0, 0x20);
    }
    port_byte_out(0x20, 0x20);

    if let Some(handler) = INTERRUPT_HANDLERS[int_no] {
        handler(r);
    }
}
