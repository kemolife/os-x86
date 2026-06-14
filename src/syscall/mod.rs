//! System calls — the controlled gateway from user (ring 3) into the kernel.
//!
//! A user program loads the call number into EAX and arguments into EBX/ECX/EDX
//! and executes `int 0x80`. The interrupt handler routes here; the return value
//! goes back in EAX.

use crate::drivers::serial::{serial_write, serial_write_str};
use crate::libc::string::int_to_ascii;

pub const SYS_WRITE: u32 = 1;
pub const SYS_EXIT: u32 = 2;
pub const SYS_GETPID: u32 = 3;
pub const SYS_YIELD: u32 = 4;
pub const SYS_SLEEP: u32 = 5;

pub unsafe fn dispatch(num: u32, a1: u32, a2: u32, a3: u32) -> u32 {
    match num {
        SYS_WRITE => sys_write(a1, a2 as *const u8, a3 as usize),
        SYS_EXIT => sys_exit(a1),
        SYS_GETPID => crate::proc::task::current_id(),
        SYS_YIELD => {
            crate::proc::schedule();
            0
        }
        SYS_SLEEP => {
            crate::proc::sleep(a1); // a1 = milliseconds
            0
        }
        _ => u32::MAX, // unknown syscall
    }
}

/// write(fd, buf, len) — for now everything goes to the serial console.
unsafe fn sys_write(_fd: u32, buf: *const u8, len: usize) -> u32 {
    for i in 0..len {
        serial_write(*buf.add(i));
    }
    len as u32
}

/// exit(code) — terminate the current task if one is running, else just report.
unsafe fn sys_exit(code: u32) -> u32 {
    let mut b = [0u8; 12];
    serial_write_str(b"[exit code=\0".as_ptr());
    int_to_ascii(code as i32, b.as_mut_ptr());
    serial_write_str(b.as_ptr());
    serial_write_str(b"]\n\0".as_ptr());
    crate::proc::task::exit_current();
    0
}
