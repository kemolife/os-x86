use core::arch::asm;

pub unsafe fn port_byte_in(port: u16) -> u8 {
    let result: u8;
    asm!("in al, dx", out("al") result, in("dx") port, options(nostack, nomem, preserves_flags));
    result
}

pub unsafe fn port_byte_out(port: u16, data: u8) {
    asm!("out dx, al", in("dx") port, in("al") data, options(nostack, nomem, preserves_flags));
}

pub unsafe fn port_word_in(port: u16) -> u16 {
    let result: u16;
    asm!("in ax, dx", out("ax") result, in("dx") port, options(nostack, nomem, preserves_flags));
    result
}

pub unsafe fn port_word_out(port: u16, data: u16) {
    asm!("out dx, ax", in("dx") port, in("ax") data, options(nostack, nomem, preserves_flags));
}
