# 07 — Filesystem (FAT12, read-only)

## Why we need it

The ATA driver ([06-storage](06-storage.md)) reads raw numbered sectors. A
**filesystem** turns those sectors into named **files**. We implement read-only
**FAT12** — the simplest widely-supported format, used on floppies and small
disks — so the kernel can do `read_file("HELLO.TXT")`. This is the prerequisite
for loading user programs from disk later.

**FAT** = File Allocation Table: a table that records, for each storage unit
(**cluster**), which cluster comes next — chaining a file's clusters together.
FAT**12** stores 12 bits per entry.

## File structure

```
mono/src/fs/fat12.rs    BPB parse, root-dir search, FAT chain walk, read_file()
mono/src/fs/mod.rs      module
oscore/src/drivers/ata.rs the underlying sector reads
```

## Disk layout

A FAT12 volume is four regions, back to back:

```
[ boot sector + BPB ] [ FAT copies ] [ root directory ] [ data clusters ]
```

- **BPB** (BIOS Parameter Block): geometry fields inside sector 0 — bytes per
  sector, sectors per cluster, reserved sectors, number of FATs, root-dir entry
  count, sectors per FAT.
- From those we compute: `fat_start = reserved`,
  `root_start = reserved + num_fats × sectors_per_fat`,
  `data_start = root_start + root_dir_sectors`.

## How `read_file` works

1. Read sector 0, parse the BPB, compute the region offsets.
2. Scan the **root directory** (32-byte entries) for one whose 11-byte 8.3 name
   matches. Skip deleted (`0xE5`) and long-filename (attribute `0x0F`) entries;
   stop at the first `0x00` (no more entries). Record the file's first cluster
   and size.
3. Load the FAT into a heap buffer.
4. Walk the **cluster chain**: read the file's clusters into the output buffer.
   The next cluster is a 12-bit value at byte offset `cluster × 1.5` in the FAT;
   values `≥ 0xFF8` mean end-of-chain.

```rust
let fo = (cluster + cluster / 2) as usize;        // cluster * 1.5
let raw = fat[fo] as u32 | (fat[fo + 1] as u32) << 8;
cluster = if cluster & 1 == 1 { raw >> 4 } else { raw & 0xFFF };
```

### Two bugs worth remembering

- **ATA back-to-back reads** hung until we added the ATA "400ns delay" (read the
  status port 4× after drive-select) so a fresh read doesn't see the previous
  command's stale BSY/DRQ bits.
- The name match must be an explicit byte loop. Comparing a slice to an array
  reference (`&sec[a..b] == &[u8;11]`) did **not** compare element-wise as
  expected, so the file was never found.

## How to test it

Create a FAT12 image with a file, attach it as the primary IDE disk (`if=ide`),
and force floppy boot (the FAT image has its own boot signature, so without
`-boot a` the BIOS would try to boot *it*):

```bash
docker run --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 bash -c '
  make mono >/dev/null 2>&1
  dd if=/dev/zero of=/tmp/fat.img bs=512 count=2880 2>/dev/null
  mkfs.fat -F 12 /tmp/fat.img >/dev/null 2>&1
  printf "Hello from FAT12 filesystem!" > /tmp/h.txt
  mcopy -i /tmp/fat.img /tmp/h.txt ::HELLO.TXT
  timeout 6 qemu-system-i386 -m 128 -boot a \
    -drive file=os-image-mono.bin,format=raw,if=floppy -drive file=/tmp/fat.img,format=raw,if=ide \
    -nographic -serial file:/tmp/r.log -monitor null 2>/dev/null || true
  tr -d "\000" < /tmp/r.log | grep "fs: HELLO"'
```

Expected:

```
fs: HELLO.TXT = Hello from FAT12 filesystem!
```

**What it means:** the kernel parsed the FAT12 BPB, found the `HELLO.TXT`
directory entry, followed its cluster chain through the FAT, and read the file's
bytes off the disk — a real "open a file by name" end to end.

## Writing files

`write_file(name, data, len)` is the write path (shell command `save <file>`):

1. Read the BPB; load the FAT into memory.
2. Scan the FAT for enough free clusters (entry value 0).
3. Write the data into those clusters (zero-padding the last) via
   `ata::write_sectors`.
4. Link the clusters into a chain (last entry = `0xFFF`) and write every FAT
   copy back to disk.
5. Write a directory entry (name, attribute, first cluster, size) into the first
   free root-dir slot.

After `save notes.txt` in the shell, host `mtools` on the same disk image sees
and reads it:

```
$ mdir  -i fat.img ::         # NOTES.TXT  26  ...
$ mtype -i fat.img ::NOTES.TXT
saved by the os-x86 shell
```

That cross-check (an independent FAT implementation reads what we wrote) proves
the write actually hit the disk, not just a cache. Not yet supported: overwrite,
delete, and subdirectories.
