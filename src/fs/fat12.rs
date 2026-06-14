//! Minimal read-only FAT12 filesystem on the primary ATA disk.
//!
//! FAT = File Allocation Table. FAT12 stores 12 bits per cluster in that table,
//! chaining the clusters of each file together. Layout of the disk:
//!
//!   [ boot sector + BPB ][ FAT copies ][ root directory ][ data clusters ]
//!
//! BPB = BIOS Parameter Block: geometry fields inside the boot sector. We read
//! it to locate the FAT, the root directory, and the data region, then walk a
//! file's cluster chain.

use alloc::vec;
use crate::drivers::ata;

fn u16le(b: &[u8], off: usize) -> u32 {
    b[off] as u32 | (b[off + 1] as u32) << 8
}

fn u32le(b: &[u8], off: usize) -> u32 {
    b[off] as u32 | (b[off + 1] as u32) << 8 | (b[off + 2] as u32) << 16 | (b[off + 3] as u32) << 24
}

/// List the root directory: fill up to `max` 11-byte 8.3 names, return the count.
pub unsafe fn read_dir(out: *mut [u8; 11], max: usize) -> usize {
    let mut bs = [0u8; 512];
    if !ata::read_sectors(0, 1, bs.as_mut_ptr()) || u16le(&bs, 0x0B) != 512 {
        return 0;
    }
    let reserved = u16le(&bs, 0x0E);
    let num_fats = bs[0x10] as u32;
    let root_entries = u16le(&bs, 0x11);
    let sec_per_fat = u16le(&bs, 0x16);
    let root_start = reserved + num_fats * sec_per_fat;
    let root_sectors = (root_entries * 32 + 511) / 512;

    let mut count = 0usize;
    'outer: for s in 0..root_sectors {
        let mut sec = [0u8; 512];
        if !ata::read_sectors(root_start + s, 1, sec.as_mut_ptr()) {
            break;
        }
        for e in 0..16 {
            let off = e * 32;
            if sec[off] == 0x00 {
                break 'outer; // no more entries
            }
            if sec[off] == 0xE5 {
                continue; // deleted
            }
            let attr = sec[off + 0x0B];
            if attr & 0x0F == 0x0F || attr & 0x08 != 0 {
                continue; // long-filename fragment or volume label
            }
            if count >= max {
                break 'outer;
            }
            for k in 0..11 {
                (*out.add(count))[k] = sec[off + k];
            }
            count += 1;
        }
    }
    count
}

/// Read file `name` (an 11-byte 8.3 name, space-padded, e.g. b"HELLO   TXT")
/// into `out` (up to `max` bytes). Returns the file size on success.
pub unsafe fn read_file(name: &[u8; 11], out: *mut u8, max: usize) -> Option<usize> {
    let mut bs = [0u8; 512];
    if !ata::read_sectors(0, 1, bs.as_mut_ptr()) {
        return None;
    }

    let bytes_per_sec = u16le(&bs, 0x0B);
    let sec_per_clus = bs[0x0D] as u32;
    let reserved = u16le(&bs, 0x0E);
    let num_fats = bs[0x10] as u32;
    let root_entries = u16le(&bs, 0x11);
    let sec_per_fat = u16le(&bs, 0x16);
    if bytes_per_sec != 512 {
        return None; // we only support 512-byte sectors
    }

    let fat_start = reserved;
    let root_start = reserved + num_fats * sec_per_fat;
    let root_sectors = (root_entries * 32 + bytes_per_sec - 1) / bytes_per_sec;
    let data_start = root_start + root_sectors;

    // --- locate the directory entry ---
    let mut first_cluster = 0u32;
    let mut size = 0u32;
    let mut found = false;
    'search: for s in 0..root_sectors {
        let mut sec = [0u8; 512];
        if !ata::read_sectors(root_start + s, 1, sec.as_mut_ptr()) {
            return None;
        }
        for e in 0..(512 / 32) {
            let off = e * 32;
            if sec[off] == 0x00 {
                break 'search; // 0x00 = no further entries
            }
            if sec[off] == 0xE5 {
                continue; // deleted entry
            }
            if sec[off + 0x0B] & 0x0F == 0x0F {
                continue; // long-filename fragment
            }
            let mut name_matches = true;
            for k in 0..11 {
                if sec[off + k] != name[k] {
                    name_matches = false;
                    break;
                }
            }
            if name_matches {
                first_cluster = u16le(&sec, off + 0x1A);
                size = u32le(&sec, off + 0x1C);
                found = true;
                break 'search;
            }
        }
    }
    if !found {
        return None;
    }

    // --- load the whole FAT into a heap buffer ---
    let fat_bytes = (sec_per_fat * 512) as usize;
    let mut fat = vec![0u8; fat_bytes];
    for s in 0..sec_per_fat {
        if !ata::read_sectors(fat_start + s, 1, fat.as_mut_ptr().add((s * 512) as usize)) {
            return None;
        }
    }

    // --- follow the cluster chain, copying data out ---
    let limit = core::cmp::min(size as usize, max);
    let mut written = 0usize;
    let mut cluster = first_cluster;
    while cluster >= 2 && cluster < 0xFF8 && written < limit {
        let clus_sector = data_start + (cluster - 2) * sec_per_clus;
        for s in 0..sec_per_clus {
            let mut sec = [0u8; 512];
            if !ata::read_sectors(clus_sector + s, 1, sec.as_mut_ptr()) {
                return None;
            }
            let mut i = 0;
            while i < 512 && written < limit {
                *out.add(written) = sec[i];
                written += 1;
                i += 1;
            }
        }
        // next cluster: 12-bit entry at offset cluster * 1.5
        let fo = (cluster + cluster / 2) as usize;
        let raw = fat[fo] as u32 | (fat[fo + 1] as u32) << 8;
        cluster = if cluster & 1 == 1 { raw >> 4 } else { raw & 0xFFF };
    }

    Some(size as usize)
}
