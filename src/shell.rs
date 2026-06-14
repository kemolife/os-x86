//! A small kernel command shell. Runs in the keyboard-IRQ context (ring 0) and
//! ties together memory, the scheduler, the filesystem, and the ELF loader.
//!
//! Commands: help, mem, ps, ls, cat <file>, run <file>, uptime, clear.
//! Output goes to both the VGA screen (what you see) and the serial log.

use crate::drivers::screen::{kprint, clear_screen};
use crate::drivers::serial::serial_write_str;
use crate::libc::string::int_to_ascii;
use crate::mm::{pmm, heap};
use crate::fs::{fat12, elf};
use crate::cpu::timer;
use crate::proc;

static mut PENDING_EXEC: [u8; 11] = [b' '; 11];

unsafe fn puts(s: *const u8) {
    kprint(s);
    serial_write_str(s);
}

unsafe fn putn(n: i32) {
    let mut b = [0u8; 12];
    int_to_ascii(n, b.as_mut_ptr());
    puts(b.as_ptr());
}

fn upper(c: u8) -> u8 {
    if c >= b'a' && c <= b'z' { c - 32 } else { c }
}

/// Does the first word of the command line `s` equal `lit`?
unsafe fn cmd_is(s: *const u8, lit: &[u8]) -> bool {
    let mut i = 0;
    while i < lit.len() {
        if *s.add(i) != lit[i] {
            return false;
        }
        i += 1;
    }
    let c = *s.add(lit.len());
    c == 0 || c == b' '
}

/// Pointer to the argument (text after the first word + spaces).
unsafe fn arg_of(s: *const u8) -> *const u8 {
    let mut i = 0;
    while *s.add(i) != 0 && *s.add(i) != b' ' {
        i += 1;
    }
    while *s.add(i) == b' ' {
        i += 1;
    }
    s.add(i)
}

/// Convert a typed name like "hello.txt" to a padded 8.3 name "HELLO   TXT".
unsafe fn to_83(name: *const u8) -> [u8; 11] {
    let mut out = [b' '; 11];
    let mut i = 0;
    let mut o = 0;
    while *name.add(i) != 0 && *name.add(i) != b'.' && o < 8 {
        out[o] = upper(*name.add(i));
        i += 1;
        o += 1;
    }
    while *name.add(i) != 0 && *name.add(i) != b'.' {
        i += 1;
    }
    if *name.add(i) == b'.' {
        i += 1;
    }
    let mut e = 8;
    while *name.add(i) != 0 && e < 11 {
        out[e] = upper(*name.add(i));
        i += 1;
        e += 1;
    }
    out
}

extern "C" fn exec_launcher() {
    unsafe {
        let name = PENDING_EXEC; // copy out of the static before borrowing
        if !elf::exec(&name) {
            serial_write_str(b"run: program not found\n\0".as_ptr());
        }
        proc::task::exit_current();
    }
}

/// The keyboard line handler.
pub fn run(input: *mut u8) {
    unsafe {
        let s = input as *const u8;

        if *s == 0 {
            // empty line
        } else if cmd_is(s, b"help") {
            puts(b"commands: help mem ps ls cat <f> run <f> uptime clear\n\0".as_ptr());
        } else if cmd_is(s, b"mem") {
            puts(b"pmm: \0".as_ptr());
            putn(pmm::free_frames() as i32);
            puts(b" / \0".as_ptr());
            putn(pmm::total_frames() as i32);
            puts(b" frames free\n\0".as_ptr());
            let (free, used, blocks) = heap::stats();
            puts(b"heap: \0".as_ptr());
            putn(free as i32);
            puts(b" free / \0".as_ptr());
            putn(used as i32);
            puts(b" used bytes, \0".as_ptr());
            putn(blocks as i32);
            puts(b" blocks\n\0".as_ptr());
        } else if cmd_is(s, b"ps") {
            let n = proc::task::count();
            for i in 0..n {
                let (id, st) = proc::task::get(i);
                puts(b"task \0".as_ptr());
                putn(id as i32);
                puts(b": \0".as_ptr());
                puts(state_name(st));
                puts(b"\n\0".as_ptr());
            }
        } else if cmd_is(s, b"ls") {
            let mut names = [[0u8; 11]; 32];
            let n = fat12::read_dir(names.as_mut_ptr(), 32);
            if n == 0 {
                puts(b"(no files / no disk)\n\0".as_ptr());
            }
            for i in 0..n {
                print_83(&names[i]);
                puts(b"\n\0".as_ptr());
            }
        } else if cmd_is(s, b"cat") {
            let name = to_83(arg_of(s));
            let mut buf = [0u8; 512];
            match fat12::read_file(&name, buf.as_mut_ptr(), 511) {
                Some(n) => {
                    buf[n.min(511)] = 0;
                    puts(buf.as_ptr());
                    puts(b"\n\0".as_ptr());
                }
                None => puts(b"cat: file not found\n\0".as_ptr()),
            }
        } else if cmd_is(s, b"run") {
            PENDING_EXEC = to_83(arg_of(s));
            proc::spawn(exec_launcher);
            puts(b"run: launched\n\0".as_ptr());
        } else if cmd_is(s, b"uptime") {
            putn((timer::ticks() / timer::TIMER_HZ) as i32);
            puts(b"s\n\0".as_ptr());
        } else if cmd_is(s, b"clear") {
            clear_screen();
        } else {
            puts(b"unknown command (try: help)\n\0".as_ptr());
        }

        kprint(b"> \0".as_ptr());
    }
}

fn state_name(code: u8) -> *const u8 {
    match code {
        1 => b"ready\0".as_ptr(),
        2 => b"running\0".as_ptr(),
        3 => b"blocked\0".as_ptr(),
        4 => b"finished\0".as_ptr(),
        _ => b"unused\0".as_ptr(),
    }
}

/// Print an 11-byte 8.3 name as "NAME.EXT".
unsafe fn print_83(name: &[u8; 11]) {
    for i in 0..8 {
        if name[i] != b' ' {
            let c = [name[i], 0];
            puts(c.as_ptr());
        }
    }
    if name[8] != b' ' {
        puts(b".\0".as_ptr());
        for i in 8..11 {
            if name[i] != b' ' {
                let c = [name[i], 0];
                puts(c.as_ptr());
            }
        }
    }
}
