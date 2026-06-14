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
    crate::cpu::gdt::init();
    serial_write_str(b"gdt: kernel+user segments + TSS loaded\n\0".as_ptr());
    screen_init(SCREEN_VGA_DEFAULT);
    mem_init(0x50000); // heap above the kernel (~0x2f200) and below the stack (0x90000)
    isr_install();
    irq_install();
    init_timer(50);
    init_keyboard();
    keyboard_set_handler(user_input);
    crate::cpu::isr::syscall_install();

    core::arch::asm!("int 2", options(nostack));
    core::arch::asm!("int 3", options(nostack));
    core::arch::asm!("int 1", options(nostack));

    // Multitasking demo: two ring-0 kernel threads + one ring-3 user program.
    crate::proc::init();
    crate::proc::spawn(thread_a);
    crate::proc::spawn(thread_b);
    crate::proc::spawn(user_launcher);
    crate::proc::enable();

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

// Ring-3 user program: talks to the kernel only through int 0x80 syscalls.
extern "C" fn user_program() {
    let msg = b"Hello from ring 3 via syscall!\n";
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("eax") 1u32,            // SYS_WRITE
            in("ebx") 1u32,            // fd
            in("ecx") msg.as_ptr(),
            in("edx") msg.len(),
            options(nostack),
        );
        core::arch::asm!(
            "int 0x80",
            in("eax") 2u32,            // SYS_EXIT
            in("ebx") 0u32,
            options(nostack, noreturn),
        );
    }
}

// Kernel thread: load INIT.ELF off the disk and run it in ring 3. If there is
// no disk, fall back to the built-in ring-3 program.
extern "C" fn user_launcher() {
    unsafe {
        if crate::fs::elf::exec(b"INIT    ELF") {
            return; // unreachable: exec enters ring 3 and never comes back
        }
        serial_write_str(b"elf: INIT.ELF not found, running built-in user program\n\0".as_ptr());
        let stack = crate::mm::heap::kmalloc(4096) as u32 + 4096;
        crate::cpu::gdt::enter_user_mode(user_program as *const () as u32, stack);
    }
}

fn spin_delay() {
    for _ in 0..800_000 {
        core::hint::spin_loop();
    }
}

// thread_a sleeps 1s between prints (yields the CPU — no busy waiting).
extern "C" fn thread_a() {
    unsafe {
        for _ in 0..3 {
            serial_write_str(b"[A]\0".as_ptr());
            crate::proc::sleep(1000);
        }
        serial_write_str(b"[A done]\n\0".as_ptr());
    }
}

// thread_b stays busy; its [B]s fill the gaps while thread_a is asleep.
extern "C" fn thread_b() {
    unsafe {
        for _ in 0..12 {
            serial_write_str(b"[B]\0".as_ptr());
            spin_delay();
        }
        serial_write_str(b"[B done]\n\0".as_ptr());
    }
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
