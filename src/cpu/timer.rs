use crate::cpu::isr::{Registers, register_interrupt_handler};
use crate::cpu::ports::port_byte_out;
use crate::drivers::screen::screen_write_at;
use crate::libc::string::int_to_ascii;

const IRQ0: u8 = 32;

static mut TICK: u32 = 0;

fn timer_callback(regs: *const Registers) {
    unsafe {
        let _ = regs;
        TICK += 1;
        if TICK % 50 != 0 {
            return;
        }
        let uptime = TICK / 50;
        let mut num = [0u8; 10];
        int_to_ascii(uptime as i32, num.as_mut_ptr());

        let mut buf = [0u8; 12];
        buf[0] = b'U'; buf[1] = b'P'; buf[2] = b':';
        let mut i = 3;
        let mut j = 0;
        while num[j] != 0 {
            buf[i] = num[j];
            i += 1; j += 1;
        }
        buf[i] = b's';
        buf[i + 1] = 0;
        screen_write_at(buf.as_ptr(), 72, 0);
    }
}

pub unsafe fn init_timer(freq: u32) {
    register_interrupt_handler(IRQ0, timer_callback);
    let divisor = 1193180 / freq;
    port_byte_out(0x43, 0x36);
    port_byte_out(0x40, (divisor & 0xFF) as u8);
    port_byte_out(0x40, ((divisor >> 8) & 0xFF) as u8);
}
