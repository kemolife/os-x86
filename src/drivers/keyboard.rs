use crate::cpu::isr::{Registers, register_interrupt_handler};
use crate::cpu::ports::port_byte_in;
use crate::drivers::screen::{kprint, kprint_backspace};
use crate::libc::string::{append, backspace};

const IRQ1: u8 = 33;
const BACKSPACE: u8 = 0x0E;
const ENTER: u8 = 0x1C;
const SC_MAX: u8 = 57;

static SC_ASCII: [u8; 58] = [
    b'?', b'?', b'1', b'2', b'3', b'4', b'5', b'6',
    b'7', b'8', b'9', b'0', b'-', b'=', b'?', b'?',
    b'Q', b'W', b'E', b'R', b'T', b'Y', b'U', b'I',
    b'O', b'P', b'[', b']', b'?', b'?', b'A', b'S',
    b'D', b'F', b'G', b'H', b'J', b'K', b'L', b';',
    b'\'', b'`', b'?', b'\\', b'Z', b'X', b'C', b'V',
    b'B', b'N', b'M', b',', b'.', b'/', b'?', b'?',
    b'?', b' ',
];

static mut KEY_BUFFER: [u8; 256] = [0u8; 256];
static mut INPUT_HANDLER: Option<fn(*mut u8)> = None;

fn keyboard_callback(regs: *const Registers) {
    unsafe {
        let _ = regs;
        let scancode = port_byte_in(0x60);
        if scancode > SC_MAX {
            return;
        }
        if scancode == BACKSPACE {
            backspace((&raw mut KEY_BUFFER) as *mut u8);
            kprint_backspace();
        } else if scancode == ENTER {
            kprint(b"\n\0".as_ptr());
            if let Some(handler) = INPUT_HANDLER {
                handler((&raw mut KEY_BUFFER) as *mut u8);
            }
            KEY_BUFFER[0] = 0;
        } else {
            let letter = SC_ASCII[scancode as usize];
            let str = [letter, 0u8];
            append((&raw mut KEY_BUFFER) as *mut u8, letter);
            kprint(str.as_ptr());
        }
    }
}

pub unsafe fn keyboard_set_handler(handler: fn(*mut u8)) {
    INPUT_HANDLER = Some(handler);
}

pub unsafe fn init_keyboard() {
    register_interrupt_handler(IRQ1, keyboard_callback);
}
