use crate::libc::mem::memory_copy;
use crate::cpu::ports::{port_byte_in, port_byte_out};

const RED_ON_WHITE: u8 = 0xf4;

pub struct ScreenConfig {
    pub video_addr: u32,
    pub max_rows: i32,
    pub max_cols: i32,
    pub default_attr: u8,
    pub ctrl_port: u16,
    pub data_port: u16,
}

pub const SCREEN_VGA_DEFAULT: ScreenConfig = ScreenConfig {
    video_addr: 0xb8000,
    max_rows: 25,
    max_cols: 80,
    default_attr: 0x0f,
    ctrl_port: 0x3d4,
    data_port: 0x3d5,
};

static mut CFG: ScreenConfig = ScreenConfig {
    video_addr: 0xb8000,
    max_rows: 25,
    max_cols: 80,
    default_attr: 0x0f,
    ctrl_port: 0x3d4,
    data_port: 0x3d5,
};

pub unsafe fn screen_init(c: ScreenConfig) {
    CFG = c;
}

pub unsafe fn kprint(message: *const u8) {
    kprint_at(message, -1, -1);
}

pub unsafe fn kprint_at(message: *const u8, mut col: i32, mut row: i32) {
    if col < 0 || row < 0 {
        let o = get_screen_offset();
        row = get_offset_row(o);
        col = get_offset_col(o);
    }
    let mut i = 0;
    while *message.add(i) != 0 {
        let offset = print_char(*message.add(i), col, row, CFG.default_attr);
        row = get_offset_row(offset);
        col = get_offset_col(offset);
        i += 1;
    }
}

pub unsafe fn kprint_backspace() {
    let offset = get_screen_offset() - 2;
    let row = get_offset_row(offset);
    let col = get_offset_col(offset);
    print_char(0x08, col, row, CFG.default_attr);
}

unsafe fn print_char(c: u8, col: i32, row: i32, attr: u8) -> i32 {
    let vidmem = CFG.video_addr as *mut u8;
    let attr = if attr == 0 { CFG.default_attr } else { attr };

    if col >= CFG.max_cols || row >= CFG.max_rows {
        let last = (2 * CFG.max_cols * CFG.max_rows - 2) as usize;
        core::ptr::write_volatile(vidmem.add(last), b'E');
        core::ptr::write_volatile(vidmem.add(last + 1), RED_ON_WHITE);
        return get_offset(col, row);
    }

    let mut offset = if col >= 0 && row >= 0 {
        get_offset(col, row)
    } else {
        get_screen_offset()
    };

    if c == b'\n' {
        let row = get_offset_row(offset);
        offset = get_offset(0, row + 1);
    } else if c == 0x08 {
        core::ptr::write_volatile(vidmem.add(offset as usize), b' ');
        core::ptr::write_volatile(vidmem.add(offset as usize + 1), attr);
    } else {
        core::ptr::write_volatile(vidmem.add(offset as usize), c);
        core::ptr::write_volatile(vidmem.add(offset as usize + 1), attr);
        offset += 2;
    }

    if offset >= CFG.max_rows * CFG.max_cols * 2 {
        for i in 1..CFG.max_rows {
            memory_copy(
                (get_offset(0, i) + CFG.video_addr as i32) as *const u8,
                (get_offset(0, i - 1) + CFG.video_addr as i32) as *mut u8,
                (CFG.max_cols * 2) as usize,
            );
        }
        let last_line = (get_offset(0, CFG.max_rows - 1) + CFG.video_addr as i32) as *mut u8;
        for i in 0..(CFG.max_cols * 2) as usize {
            core::ptr::write_volatile(last_line.add(i), 0);
        }
        offset -= 2 * CFG.max_cols;
    }

    set_screen_offset(offset);
    offset
}

unsafe fn get_screen_offset() -> i32 {
    port_byte_out(CFG.ctrl_port, 14);
    let offset = (port_byte_in(CFG.data_port) as i32) << 8;
    port_byte_out(CFG.ctrl_port, 15);
    (offset + port_byte_in(CFG.data_port) as i32) * 2
}

unsafe fn set_screen_offset(offset: i32) {
    let offset = offset / 2;
    port_byte_out(CFG.ctrl_port, 14);
    port_byte_out(CFG.data_port, (offset >> 8) as u8);
    port_byte_out(CFG.ctrl_port, 15);
    port_byte_out(CFG.data_port, (offset & 0xff) as u8);
}

pub unsafe fn clear_screen() {
    let screen_size = (CFG.max_cols * CFG.max_rows) as usize;
    let screen = CFG.video_addr as *mut u8;
    for i in 0..screen_size {
        core::ptr::write_volatile(screen.add(i * 2), b' ');
        core::ptr::write_volatile(screen.add(i * 2 + 1), CFG.default_attr);
    }
    set_screen_offset(get_offset(0, 0));
}

pub unsafe fn screen_write_at(msg: *const u8, col: i32, row: i32) {
    let vidmem = CFG.video_addr as *mut u8;
    let mut i = 0;
    while *msg.add(i) != 0 {
        let offset = get_offset(col + i as i32, row) as usize;
        core::ptr::write_volatile(vidmem.add(offset), *msg.add(i));
        core::ptr::write_volatile(vidmem.add(offset + 1), CFG.default_attr);
        i += 1;
    }
}

fn get_offset(col: i32, row: i32) -> i32 {
    2 * (row * unsafe { CFG.max_cols } + col)
}

fn get_offset_row(offset: i32) -> i32 {
    offset / (2 * unsafe { CFG.max_cols })
}

fn get_offset_col(offset: i32) -> i32 {
    let row = get_offset_row(offset);
    (offset - row * 2 * unsafe { CFG.max_cols }) / 2
}
