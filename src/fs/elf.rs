//! Minimal ELF32 loader: read a program off the FAT12 disk and run it in ring 3
//! inside its own address space.
//!
//! ELF = Executable and Linkable Format. We handle a 32-bit executable with
//! PT_LOAD segments. Each program gets a fresh page directory (sharing the
//! kernel's supervisor identity map); its segments and a user stack are backed
//! by freshly allocated physical frames mapped USER-accessible. So two programs
//! can't see each other's memory or the kernel's.

use alloc::vec;
use crate::cpu::gdt::enter_user_mode;
use crate::fs::fat12;
use crate::mm::{paging, pmm};

const USER_STACK_ADDR: u32 = 0x5000_0000;
const USER_STACK_TOP: u32 = USER_STACK_ADDR + 0x1000; // one page
const PT_LOAD: u32 = 1;
const FRAME: u32 = 4096;

fn u16le(b: &[u8], o: usize) -> u32 {
    b[o] as u32 | (b[o + 1] as u32) << 8
}
fn u32le(b: &[u8], o: usize) -> u32 {
    b[o] as u32 | (b[o + 1] as u32) << 8 | (b[o + 2] as u32) << 16 | (b[o + 3] as u32) << 24
}

/// Load and run `name` (an 11-byte 8.3 name). On success this enters ring 3 in
/// the program's own address space and never returns; returns `false` if the
/// file is missing or not a valid ELF32 executable.
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

    // Build a new address space for this program.
    let dir = paging::new_address_space();

    for i in 0..phnum {
        let ph = phoff + i * phentsize;
        if u32le(&buf, ph) != PT_LOAD {
            continue;
        }
        let off = u32le(&buf, ph + 0x04) as usize;
        let vaddr = u32le(&buf, ph + 0x08);
        let filesz = u32le(&buf, ph + 0x10) as usize;
        let memsz = u32le(&buf, ph + 0x14) as usize;

        // Map and fill one page at a time across [vaddr, vaddr+memsz).
        let mut page = vaddr & !(FRAME - 1);
        let end = vaddr + memsz as u32;
        while page < end {
            let frame = pmm::alloc_frame().expect("no frame for user segment") as u32;
            // Frames come from low RAM, so they're identity-mapped in the kernel
            // space we're currently in — fill them via their physical address.
            core::ptr::write_bytes(frame as *mut u8, 0, FRAME as usize);
            let mut b = 0u32;
            while b < FRAME {
                let va = page + b;
                if va >= vaddr && (va - vaddr) < filesz as u32 {
                    *((frame + b) as *mut u8) = buf[off + (va - vaddr) as usize];
                }
                b += 1;
            }
            paging::map_user_page(dir, page, frame);
            page += FRAME;
        }
    }

    // User stack: one fresh user-mapped page.
    let stack_frame = pmm::alloc_frame().expect("no frame for user stack") as u32;
    core::ptr::write_bytes(stack_frame as *mut u8, 0, FRAME as usize);
    paging::map_user_page(dir, USER_STACK_ADDR, stack_frame);

    // Bind the address space to this task, activate it, and drop to ring 3.
    crate::proc::task::set_current_page_dir(dir);
    paging::switch_address_space(dir);
    enter_user_mode(entry, USER_STACK_TOP);
}
