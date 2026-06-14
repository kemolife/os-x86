use crate::cpu::isr::{Registers, register_interrupt_handler};
use crate::cpu::ports::port_byte_in;
use crate::drivers::screen::{kprint, kprint_backspace};
use crate::drivers::serial::serial_write;
use crate::libc::string::{append, backspace, strlen};

const IRQ1: u8 = 33;
const BACKSPACE: u8 = 0x0E;
const ENTER: u8 = 0x1C;
const SC_MAX: u8 = 57;

// Scancode (set 1) make codes for the modifier keys we track.
const LSHIFT: u8 = 0x2A;
const RSHIFT: u8 = 0x36;
const LSHIFT_REL: u8 = 0xAA;
const RSHIFT_REL: u8 = 0xB6;

// Extended (0xE0-prefixed) scancodes for the arrow keys.
const EXT_PREFIX: u8 = 0xE0;
const UP: u8 = 0x48;
const DOWN: u8 = 0x50;

const HIST_MAX: usize = 16;

// Unshifted (lowercase) layout.
static SC_ASCII: [u8; 58] = [
    b'?', b'?', b'1', b'2', b'3', b'4', b'5', b'6',
    b'7', b'8', b'9', b'0', b'-', b'=', b'?', b'?',
    b'q', b'w', b'e', b'r', b't', b'y', b'u', b'i',
    b'o', b'p', b'[', b']', b'?', b'?', b'a', b's',
    b'd', b'f', b'g', b'h', b'j', b'k', b'l', b';',
    b'\'', b'`', b'?', b'\\', b'z', b'x', b'c', b'v',
    b'b', b'n', b'm', b',', b'.', b'/', b'?', b'?',
    b'?', b' ',
];

// Shifted layout (Shift held).
static SC_ASCII_SHIFT: [u8; 58] = [
    b'?', b'?', b'!', b'@', b'#', b'$', b'%', b'^',
    b'&', b'*', b'(', b')', b'_', b'+', b'?', b'?',
    b'Q', b'W', b'E', b'R', b'T', b'Y', b'U', b'I',
    b'O', b'P', b'{', b'}', b'?', b'?', b'A', b'S',
    b'D', b'F', b'G', b'H', b'J', b'K', b'L', b':',
    b'"', b'~', b'?', b'|', b'Z', b'X', b'C', b'V',
    b'B', b'N', b'M', b'<', b'>', b'?', b'?', b'?',
    b'?', b' ',
];

static mut KEY_BUFFER: [u8; 256] = [0u8; 256];
static mut INPUT_HANDLER: Option<fn(*mut u8)> = None;
static mut SHIFT: bool = false;
static mut EXTENDED: bool = false;

// Command history (ring of recent lines).
static mut HISTORY: [[u8; 256]; HIST_MAX] = [[0u8; 256]; HIST_MAX];
static mut HIST_LEN: usize = 0; // number of stored lines
static mut HIST_BROWSE: usize = 0; // 0..=HIST_LEN; == HIST_LEN means "new line"

unsafe fn hist_push(line: *const u8) {
    if *line == 0 {
        return; // don't store empty lines
    }
    let dst = if HIST_LEN < HIST_MAX {
        let i = HIST_LEN;
        HIST_LEN += 1;
        i
    } else {
        for i in 1..HIST_MAX {
            HISTORY[i - 1] = HISTORY[i]; // drop the oldest
        }
        HIST_MAX - 1
    };
    let mut i = 0;
    while *line.add(i) != 0 && i < 255 {
        HISTORY[dst][i] = *line.add(i);
        i += 1;
    }
    HISTORY[dst][i] = 0;
}

// Erase the current line on screen+serial and replace it with `src`.
unsafe fn replace_line(src: *const u8) {
    let cur = strlen((&raw const KEY_BUFFER) as *const u8);
    for _ in 0..cur {
        kprint_backspace();
        serial_write(0x08);
        serial_write(b' ');
        serial_write(0x08);
    }
    let mut i = 0;
    while *src.add(i) != 0 && i < 255 {
        KEY_BUFFER[i] = *src.add(i);
        serial_write(*src.add(i));
        i += 1;
    }
    KEY_BUFFER[i] = 0;
    kprint((&raw const KEY_BUFFER) as *const u8);
}

unsafe fn history_up() {
    if HIST_BROWSE > 0 {
        HIST_BROWSE -= 1;
        replace_line((&raw const HISTORY[HIST_BROWSE]) as *const u8);
    }
}

unsafe fn history_down() {
    if HIST_BROWSE < HIST_LEN {
        HIST_BROWSE += 1;
        if HIST_BROWSE == HIST_LEN {
            replace_line(b"\0".as_ptr()); // past the newest -> empty line
        } else {
            replace_line((&raw const HISTORY[HIST_BROWSE]) as *const u8);
        }
    }
}

fn keyboard_callback(regs: *const Registers) {
    unsafe {
        let _ = regs;
        let sc = port_byte_in(0x60);

        // Extended-scancode prefix (arrow keys etc.) — the real code follows.
        if sc == EXT_PREFIX {
            EXTENDED = true;
            return;
        }
        if EXTENDED {
            EXTENDED = false;
            match sc {
                UP => history_up(),
                DOWN => history_down(),
                _ => {}
            }
            return;
        }

        // Shift make/release.
        if sc == LSHIFT || sc == RSHIFT {
            SHIFT = true;
            return;
        }
        if sc == LSHIFT_REL || sc == RSHIFT_REL {
            SHIFT = false;
            return;
        }
        // Ignore all other key releases (high bit set).
        if sc & 0x80 != 0 {
            return;
        }

        if sc == BACKSPACE {
            backspace((&raw mut KEY_BUFFER) as *mut u8);
            kprint_backspace();
            serial_write(0x08); // echo backspace to serial too
            serial_write(b' ');
            serial_write(0x08);
        } else if sc == ENTER {
            kprint(b"\n\0".as_ptr());
            serial_write(b'\r');
            serial_write(b'\n');
            hist_push((&raw const KEY_BUFFER) as *const u8);
            HIST_BROWSE = HIST_LEN; // reset browsing to the new (empty) line
            if let Some(handler) = INPUT_HANDLER {
                handler((&raw mut KEY_BUFFER) as *mut u8);
            }
            KEY_BUFFER[0] = 0;
        } else if sc <= SC_MAX {
            let letter = if SHIFT {
                SC_ASCII_SHIFT[sc as usize]
            } else {
                SC_ASCII[sc as usize]
            };
            let str = [letter, 0u8];
            append((&raw mut KEY_BUFFER) as *mut u8, letter);
            kprint(str.as_ptr());
            serial_write(letter); // echo typed char to serial too
        }
    }
}

pub unsafe fn keyboard_set_handler(handler: fn(*mut u8)) {
    INPUT_HANDLER = Some(handler);
}

pub unsafe fn init_keyboard() {
    register_interrupt_handler(IRQ1, keyboard_callback);
}
