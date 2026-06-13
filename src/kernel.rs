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
