use kcore::drivers::screen::{screen_init, kprint, SCREEN_VGA_DEFAULT};
use kcore::libc::mem::mem_init;
use kcore::cpu::isr::{isr_install, irq_install};
use kcore::cpu::timer::init_timer;
use kcore::drivers::keyboard::{init_keyboard, keyboard_set_handler};
use kcore::drivers::serial::{init_serial, serial_write_str};

// Bridge the HAL hooks to this kernel's policy (safe-fn wrappers).
fn syscall_hook(n: u32, a: u32, b: u32, c: u32) -> u32 {
    unsafe { crate::syscall::dispatch(n, a, b, c) }
}
fn tick_hook() {
    unsafe {
        if crate::proc::enabled() {
            crate::proc::task::wake_sleepers(kcore::cpu::timer::ticks());
            crate::proc::schedule();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn kernel_main() {
    kcore::hooks::set_syscall(syscall_hook);
    kcore::hooks::set_tick(tick_hook);
    init_serial();
    kcore::mm::e820::print_map();
    kcore::mm::pmm::init();
    kcore::mm::pmm::print_stats();
    kcore::mm::paging::init();
    serial_write_str(b"paging: enabled (identity-mapped low 16MB)\n\0".as_ptr());
    kcore::mm::heap::init();
    kcore::mm::heap::print_stats();
    kcore::cpu::gdt::init();
    serial_write_str(b"gdt: kernel+user segments + TSS loaded\n\0".as_ptr());

    screen_init(SCREEN_VGA_DEFAULT);
    mem_init(0x50000); // legacy bump heap, kept for compatibility
    isr_install();
    irq_install();
    init_timer(50);
    init_keyboard();
    keyboard_set_handler(crate::shell::run);
    kcore::cpu::isr::syscall_install();

    // Bring up the scheduler (spawns the idle task); the shell `run` command
    // launches user programs as tasks.
    crate::proc::init();
    crate::proc::enable();

    serial_write_str(b"os-x86 ready. serial I/O active.\n\0".as_ptr());
    let banner = b"os-x86 kernel shell. type 'help'.\n> \0";
    kprint(banner.as_ptr());
    serial_write_str(banner.as_ptr());
}
