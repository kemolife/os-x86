//! ATA (AT Attachment, a.k.a. IDE) hard-disk driver, PIO mode, LBA28.
//!
//! "PIO" = Programmed I/O: the CPU moves every word through an I/O port (no
//! DMA). "LBA28" = 28-bit Linear Block Addressing — sectors are numbered
//! 0,1,2,... instead of cylinder/head/sector, up to 2^28 sectors (128GB).
//!
//! We only drive the primary bus, master drive. Register ports:
//!   0x1F0 data (16-bit)   0x1F2 sector count   0x1F3-5 LBA low/mid/high
//!   0x1F6 drive/head      0x1F7 status (read) / command (write)

use crate::cpu::ports::{port_byte_in, port_byte_out, port_word_in};

const DATA: u16 = 0x1F0;
const SECCOUNT: u16 = 0x1F2;
const LBA_LO: u16 = 0x1F3;
const LBA_MID: u16 = 0x1F4;
const LBA_HI: u16 = 0x1F5;
const DRIVE: u16 = 0x1F6;
const STATUS_CMD: u16 = 0x1F7;

const CMD_READ_SECTORS: u8 = 0x20;

const ST_ERR: u8 = 1 << 0; // error
const ST_DRQ: u8 = 1 << 3; // data request ready
const ST_BSY: u8 = 1 << 7; // busy

const TIMEOUT: u32 = 1_000_000;

unsafe fn wait_not_busy() -> bool {
    let mut spin = 0u32;
    loop {
        let s = port_byte_in(STATUS_CMD);
        if s == 0xFF {
            return false; // floating bus = no drive present
        }
        if s & ST_BSY == 0 {
            return true;
        }
        spin += 1;
        if spin > TIMEOUT {
            return false;
        }
    }
}

unsafe fn wait_data_ready() -> bool {
    let mut spin = 0u32;
    loop {
        let s = port_byte_in(STATUS_CMD);
        if s == 0xFF || s & ST_ERR != 0 {
            return false;
        }
        if s & ST_BSY == 0 && s & ST_DRQ != 0 {
            return true;
        }
        spin += 1;
        if spin > TIMEOUT {
            return false;
        }
    }
}

/// Read `count` 512-byte sectors starting at LBA `lba` (primary master) into
/// `buf` (must hold count*512 bytes). Returns false on a drive error.
pub unsafe fn read_sectors(lba: u32, count: u8, buf: *mut u8) -> bool {
    if !wait_not_busy() {
        return false;
    }
    // 0xE0 = LBA mode, master drive; top 4 LBA bits go in the low nibble.
    port_byte_out(DRIVE, 0xE0 | ((lba >> 24) & 0x0F) as u8);
    // 400ns settle after drive select: read the status port a few times so the
    // BSY/DRQ bits reflect the newly selected drive, not the previous command.
    for _ in 0..4 {
        let _ = port_byte_in(STATUS_CMD);
    }
    port_byte_out(SECCOUNT, count);
    port_byte_out(LBA_LO, (lba & 0xFF) as u8);
    port_byte_out(LBA_MID, ((lba >> 8) & 0xFF) as u8);
    port_byte_out(LBA_HI, ((lba >> 16) & 0xFF) as u8);
    port_byte_out(STATUS_CMD, CMD_READ_SECTORS);

    let mut off = 0usize;
    for _ in 0..count {
        if !wait_data_ready() {
            return false;
        }
        for _ in 0..256 {
            let w = port_word_in(DATA);
            *buf.add(off) = (w & 0xFF) as u8;
            *buf.add(off + 1) = (w >> 8) as u8;
            off += 2;
        }
    }
    true
}

/// Read sector 0 of the primary master and dump the first 16 bytes to serial.
pub unsafe fn probe() {
    use crate::drivers::serial::serial_write_str;
    use crate::libc::string::hex_to_ascii;

    let mut sector = [0u8; 512];
    if !read_sectors(0, 1, sector.as_mut_ptr()) {
        serial_write_str(b"ata: read error (no disk?)\n\0".as_ptr());
        return;
    }
    serial_write_str(b"ata: sector 0 =\0".as_ptr());
    for i in 0..16 {
        let mut buf = [0u8; 8];
        hex_to_ascii(sector[i] as i32, buf.as_mut_ptr());
        serial_write_str(b" \0".as_ptr());
        serial_write_str(buf.as_ptr());
    }
    serial_write_str(b"\n\0".as_ptr());
}
