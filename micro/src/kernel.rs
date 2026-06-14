//! Microkernel entry. The kernel itself stays minimal: bring up the HAL, then
//! run an echo *server* and a *client* as separate tasks that talk only through
//! IPC messages — the defining microkernel pattern.

use oscore::drivers::screen::{screen_init, SCREEN_VGA_DEFAULT};
use oscore::drivers::serial::{init_serial, serial_write, serial_write_str};
use oscore::cpu::isr::{isr_install, irq_install};
use oscore::cpu::timer::init_timer;
use oscore::libc::string::hex_to_ascii;

use crate::{ipc, sched};

const SERVER: usize = 1; // task index of the echo server

static mut ENABLED: bool = false;

fn tick_hook() {
    unsafe {
        if ENABLED {
            sched::schedule();
        }
    }
}

// Echo server: receive a message, send the same value back to its sender.
extern "C" fn server() {
    unsafe {
        loop {
            let (from, val) = ipc::recv();
            ipc::send(from, val);
        }
    }
}

// Client: send three requests, print each reply, then stop.
extern "C" fn client() {
    unsafe {
        let mut i = 0u32;
        while i < 3 {
            ipc::send(SERVER, 0xAB00 + i);
            let (_, reply) = ipc::recv();
            serial_write_str(b"client: echo reply = \0".as_ptr());
            let mut buf = [0u8; 16];
            hex_to_ascii(reply as i32, buf.as_mut_ptr());
            serial_write_str(buf.as_ptr());
            serial_write(b'\n');
            i += 1;
        }
        serial_write_str(b"client: done\n\0".as_ptr());
    }
}

#[no_mangle]
pub unsafe extern "C" fn kernel_main() {
    oscore::hooks::set_tick(tick_hook);

    init_serial();
    oscore::mm::pmm::init();
    oscore::mm::paging::init();
    oscore::mm::heap::init();
    oscore::cpu::gdt::init();
    screen_init(SCREEN_VGA_DEFAULT);
    isr_install();
    irq_install();
    init_timer(50);

    sched::init();
    sched::spawn(server); // task 1
    sched::spawn(client); // task 2
    ENABLED = true;

    serial_write_str(b"micro: IPC echo demo (server + client tasks)\n\0".as_ptr());

    // Idle: the timer preempts us into the server/client tasks.
    loop {
        core::arch::asm!("hlt", options(nostack, nomem));
    }
}
