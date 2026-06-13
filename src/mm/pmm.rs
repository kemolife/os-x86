//! Physical Memory Manager — a bitmap over 4KB physical frames.
//!
//! One bit per frame: 1 = used, 0 = free. The bitmap is placed immediately
//! after the kernel image (the `kernel_end` symbol from kernel.ld). All memory
//! below 1MB — which holds the BIOS area, the kernel, the bitmap itself, and
//! the bump heap — is reserved; frames are only handed out from extended RAM.

use crate::mm::e820;

pub const FRAME_SIZE: usize = 4096;

extern "C" {
    static kernel_end: u8;
}

static mut BITMAP: *mut u8 = core::ptr::null_mut();
static mut TOTAL_FRAMES: usize = 0;
static mut FREE_FRAMES: usize = 0;

fn align_up(x: usize) -> usize {
    (x + FRAME_SIZE - 1) & !(FRAME_SIZE - 1)
}

fn frame_index(addr: u64) -> usize {
    (addr / FRAME_SIZE as u64) as usize
}

unsafe fn mark_used(i: usize) {
    *BITMAP.add(i / 8) |= 1 << (i % 8);
}

unsafe fn mark_free(i: usize) {
    *BITMAP.add(i / 8) &= !(1 << (i % 8));
}

unsafe fn is_used(i: usize) -> bool {
    *BITMAP.add(i / 8) & (1 << (i % 8)) != 0
}

pub unsafe fn init() {
    let entries = e820::entries();

    // Highest usable physical address determines how many frames we track.
    let mut top: u64 = 0;
    for e in entries {
        if e.kind == e820::E820_USABLE {
            let end = e.base + e.length;
            if end > top {
                top = end;
            }
        }
    }
    TOTAL_FRAMES = frame_index(top);
    BITMAP = (&kernel_end as *const u8) as *mut u8;
    let bitmap_len = (TOTAL_FRAMES + 7) / 8;

    // Start with everything reserved, then free the usable regions.
    core::ptr::write_bytes(BITMAP, 0xFF, bitmap_len);
    for e in entries {
        if e.kind == e820::E820_USABLE {
            let start = frame_index(e.base);
            let end = frame_index(e.base + e.length);
            let mut i = start;
            while i < end && i < TOTAL_FRAMES {
                mark_free(i);
                i += 1;
            }
        }
    }

    // Reserve low memory + the kernel + the bitmap (all live below this line).
    let bitmap_end = BITMAP as usize + bitmap_len;
    let reserve_top = core::cmp::max(0x100000, align_up(bitmap_end));
    let reserved = (reserve_top / FRAME_SIZE).min(TOTAL_FRAMES);
    for i in 0..reserved {
        mark_used(i);
    }

    FREE_FRAMES = 0;
    for i in 0..TOTAL_FRAMES {
        if !is_used(i) {
            FREE_FRAMES += 1;
        }
    }
}

/// Allocate one physical frame; returns its physical base address.
pub unsafe fn alloc_frame() -> Option<u64> {
    for i in 0..TOTAL_FRAMES {
        if !is_used(i) {
            mark_used(i);
            FREE_FRAMES -= 1;
            return Some((i * FRAME_SIZE) as u64);
        }
    }
    None
}

/// Return a frame to the pool. `addr` must be 4KB-aligned and previously allocated.
pub unsafe fn free_frame(addr: u64) {
    let i = frame_index(addr);
    if i < TOTAL_FRAMES && is_used(i) {
        mark_free(i);
        FREE_FRAMES += 1;
    }
}

pub unsafe fn total_frames() -> usize {
    TOTAL_FRAMES
}

pub unsafe fn free_frames() -> usize {
    FREE_FRAMES
}

pub unsafe fn print_stats() {
    use crate::drivers::serial::serial_write_str;
    use crate::libc::string::int_to_ascii;
    let mut buf = [0u8; 12];
    serial_write_str(b"PMM: \0".as_ptr());
    int_to_ascii(free_frames() as i32, buf.as_mut_ptr());
    serial_write_str(buf.as_ptr());
    serial_write_str(b" free / \0".as_ptr());
    int_to_ascii(total_frames() as i32, buf.as_mut_ptr());
    serial_write_str(buf.as_ptr());
    serial_write_str(b" frames (4KB each)\n\0".as_ptr());
}

/// Allocate two frames, free one, re-allocate; confirm the freed frame is reused.
pub unsafe fn selftest() {
    use crate::drivers::serial::serial_write_str;
    use crate::libc::string::hex_to_ascii;
    let mut buf = [0u8; 16];

    let a = alloc_frame();
    let b = alloc_frame();
    if let (Some(a), Some(b)) = (a, b) {
        serial_write_str(b"PMM selftest: a=\0".as_ptr());
        buf[0] = 0; hex_to_ascii(a as i32, buf.as_mut_ptr());
        serial_write_str(buf.as_ptr());
        serial_write_str(b" b=\0".as_ptr());
        buf[0] = 0; hex_to_ascii(b as i32, buf.as_mut_ptr());
        serial_write_str(buf.as_ptr());

        free_frame(a);
        let c = alloc_frame();
        serial_write_str(b" reused=\0".as_ptr());
        buf[0] = 0; hex_to_ascii(c.unwrap_or(0) as i32, buf.as_mut_ptr());
        serial_write_str(buf.as_ptr());
        serial_write_str(if c == Some(a) { b" OK\n\0".as_ptr() } else { b" FAIL\n\0".as_ptr() });

        free_frame(b);
        free_frame(c.unwrap_or(0));
    }
}
