use crate::drivers::screen::{screen_init, kprint, SCREEN_VGA_DEFAULT};
use crate::libc::mem::{mem_init, kmalloc};
use crate::cpu::isr::{isr_install, irq_install};
use crate::cpu::timer::init_timer;
use crate::drivers::keyboard::{init_keyboard, keyboard_set_handler};
use crate::drivers::serial::{init_serial, serial_write_str};
use crate::libc::string::{hex_to_ascii, strcmp};

#[no_mangle]
pub unsafe extern "C" fn kernel_main() {
    init_serial();
    crate::mm::e820::print_map();
    crate::mm::pmm::init();
    crate::mm::pmm::print_stats();
    crate::mm::paging::init();
    serial_write_str(b"paging: enabled (identity-mapped low 16MB)\n\0".as_ptr());
    crate::mm::heap::init();
    crate::mm::heap::print_stats();
    screen_init(SCREEN_VGA_DEFAULT);
    mem_init(0x50000); // heap above the kernel (~0x2f200) and below the stack (0x90000)
    isr_install();
    irq_install();
    init_timer(50);
    init_keyboard();
    keyboard_set_handler(user_input);

    core::arch::asm!("int 2", options(nostack));
    core::arch::asm!("int 3", options(nostack));
    core::arch::asm!("int 1", options(nostack));

    kprint(b"Type something, it will go through the kernel\n\0".as_ptr());
    kprint(b"Type END to halt the CPU or PAGE to request a kmalloc()\n> \0".as_ptr());
    serial_write_str(b"os-x86 ready. serial I/O active.\n\0".as_ptr());
}

/// Exercise the global allocator: Box + a growing Vec, then drop and confirm
/// the heap reclaims the space.
unsafe fn heap_selftest() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use crate::libc::string::int_to_ascii;

    crate::mm::heap::print_stats();
    {
        let boxed = Box::new(0xCAFEu32);
        let mut v: Vec<u32> = Vec::new();
        for i in 0..16u32 {
            v.push(i * i);
        }
        let sum: u32 = v.iter().sum();
        let mut buf = [0u8; 12];
        serial_write_str(b"heap selftest: box=\0".as_ptr());
        int_to_ascii(*boxed as i32, buf.as_mut_ptr());
        serial_write_str(buf.as_ptr());
        serial_write_str(b" vec_sum=\0".as_ptr());
        int_to_ascii(sum as i32, buf.as_mut_ptr());
        serial_write_str(buf.as_ptr());
        serial_write_str(b"\n\0".as_ptr());
        crate::mm::heap::print_stats();
    }
    // boxed + v dropped here -> space returned and coalesced
    crate::mm::heap::print_stats();
}

fn user_input(input: *mut u8) {
    unsafe {
        serial_write_str(b"cmd: \0".as_ptr());
        serial_write_str(input);
        serial_write_str(b"\n\0".as_ptr());

        if strcmp(input, b"END\0".as_ptr()) == 0 {
            kprint(b"Stopping the CPU. Bye!\n\0".as_ptr());
            serial_write_str(b"halt.\n\0".as_ptr());
            core::arch::asm!("hlt", options(nostack, nomem));
        } else if strcmp(input, b"PAGE\0".as_ptr()) == 0 {
            let mut phys_addr: u32 = 0;
            let page = kmalloc(1000, 1, &mut phys_addr as *mut u32);
            let mut page_str = [0u8; 16];
            let mut phys_str = [0u8; 16];
            hex_to_ascii(page as i32, page_str.as_mut_ptr());
            hex_to_ascii(phys_addr as i32, phys_str.as_mut_ptr());
            kprint(b"Page: \0".as_ptr());
            kprint(page_str.as_ptr());
            kprint(b", physical address: \0".as_ptr());
            kprint(phys_str.as_ptr());
            kprint(b"\n\0".as_ptr());
        }
        kprint(b"You said: \0".as_ptr());
        kprint(input);
        kprint(b"\n> \0".as_ptr());
    }
}
