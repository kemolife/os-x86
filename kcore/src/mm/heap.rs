//! Kernel heap: a first-fit free list over one contiguous, PMM-backed region
//! that lives inside the identity-mapped low 16MB.
//!
//! The whole region is a chain of `Block`s in address order with no gaps, so a
//! block's neighbour always immediately follows it in memory. Every block is
//! 16-byte aligned (`Block` is `align(16)`, header is exactly 16 bytes, the
//! region base is page-aligned), so payloads are always 16-aligned — enough
//! for any normal Rust type. Allocations requesting an alignment greater than
//! 16 are not supported.

use crate::mm::pmm;

#[repr(C, align(16))]
struct Block {
    size: usize, // payload bytes (excludes this header)
    free: bool,
    next: *mut Block,
}

const HEADER: usize = core::mem::size_of::<Block>(); // 16 with align(16)
const HEAP_FRAMES: usize = 256; // 1MB

static mut HEAD: *mut Block = core::ptr::null_mut();

fn align_up(x: usize, a: usize) -> usize {
    (x + a - 1) & !(a - 1)
}

pub unsafe fn init() {
    let base = pmm::alloc_contiguous(HEAP_FRAMES).expect("no contiguous heap region") as *mut Block;
    let total = HEAP_FRAMES * pmm::FRAME_SIZE;
    (*base).size = total - HEADER;
    (*base).free = true;
    (*base).next = core::ptr::null_mut();
    HEAD = base;
}

pub unsafe fn kmalloc(size: usize) -> *mut u8 {
    let need = align_up(size.max(1), 16);
    let mut cur = HEAD;
    while !cur.is_null() {
        if (*cur).free && (*cur).size >= need {
            // Split off the remainder if it can hold a header + a little payload.
            if (*cur).size >= need + HEADER + 16 {
                let next = ((cur as usize) + HEADER + need) as *mut Block;
                (*next).size = (*cur).size - need - HEADER;
                (*next).free = true;
                (*next).next = (*cur).next;
                (*cur).size = need;
                (*cur).next = next;
            }
            (*cur).free = false;
            return ((cur as usize) + HEADER) as *mut u8;
        }
        cur = (*cur).next;
    }
    core::ptr::null_mut()
}

pub unsafe fn kfree(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    let b = ((ptr as usize) - HEADER) as *mut Block;
    (*b).free = true;
    coalesce();
}

/// Merge each free block with a free successor (one full pass, repeated where
/// needed). Blocks are contiguous, so the successor always abuts this block.
unsafe fn coalesce() {
    let mut cur = HEAD;
    while !cur.is_null() && !(*cur).next.is_null() {
        let next = (*cur).next;
        if (*cur).free && (*next).free {
            (*cur).size += HEADER + (*next).size;
            (*cur).next = (*next).next;
            // stay on `cur` to fold in further free neighbours
        } else {
            cur = (*cur).next;
        }
    }
}

/// (free bytes, used bytes, block count).
pub unsafe fn stats() -> (usize, usize, usize) {
    let (mut free, mut used, mut blocks) = (0usize, 0usize, 0usize);
    let mut cur = HEAD;
    while !cur.is_null() {
        blocks += 1;
        if (*cur).free { free += (*cur).size; } else { used += (*cur).size; }
        cur = (*cur).next;
    }
    (free, used, blocks)
}

pub unsafe fn print_stats() {
    use crate::drivers::serial::serial_write_str;
    use crate::libc::string::int_to_ascii;
    let (mut free, mut used, mut blocks) = (0usize, 0usize, 0usize);
    let mut cur = HEAD;
    while !cur.is_null() {
        blocks += 1;
        if (*cur).free { free += (*cur).size; } else { used += (*cur).size; }
        cur = (*cur).next;
    }
    let mut buf = [0u8; 12];
    serial_write_str(b"heap: \0".as_ptr());
    int_to_ascii(free as i32, buf.as_mut_ptr()); serial_write_str(buf.as_ptr());
    serial_write_str(b" free / \0".as_ptr());
    int_to_ascii(used as i32, buf.as_mut_ptr()); serial_write_str(buf.as_ptr());
    serial_write_str(b" used bytes, \0".as_ptr());
    int_to_ascii(blocks as i32, buf.as_mut_ptr()); serial_write_str(buf.as_ptr());
    serial_write_str(b" blocks\n\0".as_ptr());
}
// The #[global_allocator] lives in kcore/src/lib.rs and calls kmalloc/kfree.
