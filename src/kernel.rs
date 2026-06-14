use crate::drivers::screen::{screen_init, kprint, SCREEN_VGA_DEFAULT};
use crate::libc::mem::mem_init;
use crate::cpu::isr::{isr_install, irq_install};
use crate::cpu::timer::init_timer;
use crate::drivers::keyboard::{init_keyboard, keyboard_set_handler};
use crate::drivers::serial::{init_serial, serial_write_str};

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
    crate::cpu::gdt::init();
    serial_write_str(b"gdt: kernel+user segments + TSS loaded\n\0".as_ptr());

    screen_init(SCREEN_VGA_DEFAULT);
    mem_init(0x50000); // legacy bump heap, kept for compatibility
    isr_install();
    irq_install();
    init_timer(50);
    init_keyboard();
    keyboard_set_handler(crate::shell::run);
    crate::cpu::isr::syscall_install();

    // Bring up the scheduler (spawns the idle task); the shell `run` command
    // launches user programs as tasks.
    crate::proc::init();
    crate::proc::enable();

    serial_write_str(b"os-x86 ready. serial I/O active.\n\0".as_ptr());
    let banner = b"os-x86 kernel shell. type 'help'.\n> \0";
    kprint(banner.as_ptr());
    serial_write_str(banner.as_ptr());
}
