# 06 — Storage (ATA)

## Why we need it

To read and write persistent data — eventually a filesystem and user programs —
the kernel must talk to a disk. The standard interface on a PC is **ATA**
(AT Attachment, historically called **IDE** — Integrated Drive Electronics).

## Key terms

- **ATA / IDE** — the hard-disk interface and its register set.
- **PIO** (Programmed I/O) — the CPU moves every word of data through an I/O
  port itself. Simple but slow. The alternative, **DMA** (Direct Memory Access),
  lets the disk copy into RAM without the CPU; we don't use it yet.
- **LBA28** (Linear Block Addressing, 28-bit) — sectors are numbered
  0, 1, 2, … instead of by cylinder/head/sector. 28 bits addresses up to 2^28
  sectors ≈ 128GB.
- **sector** — the disk's unit of transfer: 512 bytes.

## File structure

```
oscore/src/drivers/ata.rs    LBA28 PIO sector reads on the primary master
oscore/src/cpu/ports.rs      port_word_in (the data port is 16-bit)
```

## How it works

The primary ATA bus exposes registers as I/O ports `0x1F0`–`0x1F7`:

| Port | Purpose |
|------|---------|
| `0x1F0` | data (16-bit, read/write the sector bytes) |
| `0x1F2` | sector count |
| `0x1F3`/`0x1F4`/`0x1F5` | LBA low / mid / high bytes |
| `0x1F6` | drive select + top LBA nibble |
| `0x1F7` | status (when read) / command (when written) |

`read_sectors(lba, count, buf)`:

1. Wait until the drive is not **BSY** (busy).
2. Write `0xE0 | (lba>>24 & 0xF)` to `0x1F6` — select LBA mode + master drive.
3. Write the sector count and the three LBA bytes.
4. Write command `0x20` (READ SECTORS) to `0x1F7`.
5. For each sector: poll the status port until **DRQ** (Data ReQuest) is set,
   then read 256 16-bit words (= 512 bytes) from the data port into the buffer.

```rust
for _ in 0..256 {
    let w = port_word_in(0x1F0);
    *buf.add(off)     = (w & 0xFF) as u8;
    *buf.add(off + 1) = (w >> 8) as u8;
    off += 2;
}
```

### Diskless safety

If no drive is attached, the status port "floats" to `0xFF` and a naive
busy-wait would loop forever. The driver treats `0xFF` as "no drive" and also
bounds every wait with a spin counter, so a normal floppy-only boot reports a
clean error instead of hanging.

## How to test it

The kernel's `ata::probe()` reads sector 0 of the primary master and dumps the
first 16 bytes to serial. Create a data disk with a known marker, attach it as
the primary IDE disk (`if=ide`), and check the bytes.

```bash
docker run --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 bash -c '
  make mono >/dev/null 2>&1
  # 1MB disk with a 16-byte ASCII marker at sector 0
  dd if=/dev/zero of=/tmp/disk.img bs=512 count=2048 2>/dev/null
  printf "ATAOKDISK0123456" | dd of=/tmp/disk.img bs=1 count=16 conv=notrunc 2>/dev/null
  timeout 6 qemu-system-i386 -m 128 \
    -drive file=os-image-mono.bin,format=raw,if=floppy \
    -drive file=/tmp/disk.img,format=raw,if=ide -nographic -serial file:/tmp/r.log -monitor null 2>/dev/null || true
  tr -d "\000" < /tmp/r.log | grep ata:'
```

Expected:

```
ata: sector 0 = 0x41 0x54 0x41 0x4f 0x4b 0x44 0x49 0x53 0x4b 0x30 0x31 0x32 0x33 0x34 0x35 0x36
```

**What it means:** those 16 hex bytes are the ASCII codes of `ATAOKDISK0123456`
(`0x41`='A', `0x54`='T', …). The driver issued a real LBA28 read command to the
emulated disk and pulled back the exact bytes we wrote — so sector reads work.

Without an IDE disk, the same probe prints:

```
ata: read error (no disk?)
```

and the boot continues normally — proving the diskless guard works.

## What's next

This is read-only, polled PIO on one drive. The storage roadmap continues with:
sector **writes**, IRQ-driven transfers, MBR partition parsing, a **FAT12/16**
filesystem (so we can read named files), and a **VFS** (Virtual File System)
layer to unify it all. See [../ROADMAP.md](../ROADMAP.md).
