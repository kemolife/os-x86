use crate::cpu::isr::{Registers, register_interrupt_handler};
use crate::cpu::ports::{port_byte_in, port_byte_out};

const COM1: u16 = 0x3F8;
const IRQ4: u8 = 36;

fn serial_rx_handler(regs: *const Registers) {
    unsafe {
        let _ = regs;
        let c = port_byte_in(COM1);
        serial_write(c);
    }
}

pub unsafe fn init_serial() {
    port_byte_out(COM1 + 1, 0x00);
    port_byte_out(COM1 + 3, 0x80);
    port_byte_out(COM1 + 0, 0x03);
    port_byte_out(COM1 + 1, 0x00);
    port_byte_out(COM1 + 3, 0x03);
    port_byte_out(COM1 + 2, 0xC7);
    port_byte_out(COM1 + 4, 0x0B);
    port_byte_out(COM1 + 1, 0x01);
    register_interrupt_handler(IRQ4, serial_rx_handler);
}

unsafe fn tx_empty() -> bool {
    port_byte_in(COM1 + 5) & 0x20 != 0
}

pub unsafe fn serial_write(c: u8) {
    while !tx_empty() {}
    port_byte_out(COM1, c);
}

pub unsafe fn serial_write_str(str: *const u8) {
    let mut i = 0;
    while *str.add(i) != 0 {
        if *str.add(i) == b'\n' {
            serial_write(b'\r');
        }
        serial_write(*str.add(i));
        i += 1;
    }
}
