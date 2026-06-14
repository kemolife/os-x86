//! BIOS E820 memory map, as collected by boot/detect_memory.asm.
//!
//! The bootloader writes the entry count as a u32 at 0x8000 and an array of
//! 24-byte entries starting at 0x8004, then switches to protected mode. We
//! read that region directly here.

use crate::drivers::serial::{serial_write, serial_write_str};

const MMAP_COUNT: *const u32 = 0x8000 as *const u32;
const MMAP_ENTRIES: *const E820Entry = 0x8004 as *const E820Entry;

/// Region type 1 = normal, usable RAM. Other values are reserved / ACPI / etc.
pub const E820_USABLE: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct E820Entry {
    pub base: u64,
    pub length: u64,
    pub kind: u32,
    pub attr: u32,
}

pub fn count() -> usize {
    unsafe { *MMAP_COUNT as usize }
}

pub fn entries() -> &'static [E820Entry] {
    unsafe { core::slice::from_raw_parts(MMAP_ENTRIES, count()) }
}

unsafe fn write_hex64(v: u64) {
    serial_write_str(b"0x\0".as_ptr());
    let mut shift: i32 = 60;
    let mut started = false;
    while shift >= 0 {
        let nib = ((v >> shift) & 0xF) as u8;
        if nib != 0 || started || shift == 0 {
            started = true;
            serial_write(if nib < 10 { b'0' + nib } else { b'A' + nib - 10 });
        }
        shift -= 4;
    }
}

/// Dump the map to the serial console — verifies the real-mode → kernel pipeline.
pub unsafe fn print_map() {
    serial_write_str(b"E820 memory map:\n\0".as_ptr());
    for e in entries() {
        serial_write_str(b"  base=\0".as_ptr());
        write_hex64(e.base);
        serial_write_str(b" len=\0".as_ptr());
        write_hex64(e.length);
        serial_write_str(b" type=\0".as_ptr());
        serial_write(b'0' + (e.kind as u8 % 10));
        if e.kind == E820_USABLE {
            serial_write_str(b" (usable)\0".as_ptr());
        }
        serial_write(b'\n');
    }
}
