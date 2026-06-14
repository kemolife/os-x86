//! Minimal ELF32 loader: read a program off the FAT12 disk and run it in ring 3.
//!
//! ELF = Executable and Linkable Format. We only handle the simplest case: a
//! 32-bit executable with PT_LOAD program segments whose virtual addresses fall
//! inside the (user-accessible) identity-mapped region. We copy each loadable
//! segment to its virtual address, then enter ring 3 at the entry point.

use alloc::vec;
use crate::cpu::gdt::enter_user_mode;
use crate::fs::fat12;
use crate::mm::heap::kmalloc;

fn u16le(b: &[u8], o: usize) -> u32 {
    b[o] as u32 | (b[o + 1] as u32) << 8
}
fn u32le(b: &[u8], o: usize) -> u32 {
    b[o] as u32 | (b[o + 1] as u32) << 8 | (b[o + 2] as u32) << 16 | (b[o + 3] as u32) << 24
}

const PT_LOAD: u32 = 1;

/// Load and run `name` (an 11-byte 8.3 name). On success this enters ring 3 and
/// never returns; it returns `false` if the file is missing or not a valid
/// ELF32 executable.
pub unsafe fn exec(name: &[u8; 11]) -> bool {
    let mut buf = vec![0u8; 64 * 1024];
    let n = match fat12::read_file(name, buf.as_mut_ptr(), buf.len()) {
        Some(n) => n,
        None => return false,
    };
    if n < 52 || buf[0] != 0x7F || buf[1] != b'E' || buf[2] != b'L' || buf[3] != b'F' {
        return false;
    }

    let entry = u32le(&buf, 0x18);
    let phoff = u32le(&buf, 0x1C) as usize;
    let phentsize = u16le(&buf, 0x2A) as usize;
    let phnum = u16le(&buf, 0x2C) as usize;

    for i in 0..phnum {
        let ph = phoff + i * phentsize;
        if u32le(&buf, ph) != PT_LOAD {
            continue;
        }
        let off = u32le(&buf, ph + 0x04) as usize;
        let vaddr = u32le(&buf, ph + 0x08);
        let filesz = u32le(&buf, ph + 0x10) as usize;
        let memsz = u32le(&buf, ph + 0x14) as usize;

        let dest = vaddr as *mut u8;
        for k in 0..filesz {
            *dest.add(k) = buf[off + k];
        }
        for k in filesz..memsz {
            *dest.add(k) = 0; // .bss: zero-fill the part not present in the file
        }
    }

    let stack = kmalloc(4096) as u32 + 4096;
    enter_user_mode(entry, stack);
}
