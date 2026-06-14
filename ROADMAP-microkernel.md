# Microkernel Roadmap

A second kernel, built **in parallel** with the monolithic one. Both share the
`kcore` hardware-abstraction layer and the same bootloader; they differ in
*where the work lives*.

```
boot/ + cpu/*.asm        shared assembly (boot, ISR stubs, context switch)
kcore/   (lib)          shared HAL: cpu, drivers, mm, libc + kernel hooks
mono/     (staticlib)    monolithic kernel — drivers/fs/shell in the kernel
micro/    (staticlib)    microkernel    — kernel routes messages; work in tasks
```

Build / run either, separately:

```bash
make mono   && make run-mono     # monolithic OS (shell, fs, ELF, ...)
make micro  && make run-micro     # microkernel (IPC echo demo)
```

## Philosophy

The kernel does almost nothing except pass **messages** between tasks. Drivers,
the filesystem, and services run as ordinary (eventually ring-3) tasks; a client
asks for work by **sending a message**, not by calling into the kernel. A
crashed service is one restartable task, not a dead kernel.

Real-world users of this style: QNX (cars), L4/OKL4 (phone modems), seL4
(defense, formally verified), INTEGRITY (avionics), Fuchsia/Zircon (Google).

## Steps

| # | Step | Detail | Status |
|---|------|--------|--------|
| 1 | **IPC + echo server** | `send(dst, val)` / `recv() -> (src, val)`; a tiny round-robin scheduler (reusing the shared `switch_context`); a server task echoes, a client task sends + prints replies | ✓ done |
| 2 | Ring-3 servers | Run the server/client as ring-3 ELF programs (own address spaces), `int 0x80` becomes the IPC trap | todo |
| 3 | Message payloads | Pass buffers, not just a `u32` — copy a message body between address spaces | todo |
| 4 | Filesystem server | Move the FAT12/disk driver out of the kernel into a user-space **FS server**; clients read files by messaging it (the real lesson) | todo |
| 5 | Ports / capabilities | Address servers by unforgeable handles instead of raw task indices; restrict who may message whom | todo |
| 6 | Server restart | Detect a crashed server and restart it without taking down the kernel (fault isolation) | todo |
| 7 | Name server | A registry task so clients can look services up by name | todo |

## Step 1 — what it shows (done)

`micro/` boots the shared HAL, then runs two tasks that communicate only via
`ipc::send` / `ipc::recv`:

```
micro: IPC echo demo (server + client tasks)
client: echo reply = 0xab00
client: echo reply = 0xab01
client: echo reply = 0xab02
client: done
```

- `recv` **blocks** the caller (reusing a Blocked task state); `send` wakes the
  receiver. The kernel is just the courier.
- The "server" is a separate task — the seed of moving drivers out of the kernel.

Files: `micro/src/ipc.rs`, `micro/src/sched.rs`, `micro/src/kernel.rs`.

## How it relates to the monolithic roadmap

```
shared: boot + kcore (cpu/drivers/mm/libc) + multitasking primitives
   ├─ mono  → VFS, drivers-in-kernel, fork/exec, user shell      (ROADMAP.md)
   └─ micro → IPC → ring-3 servers → FS server → capabilities    (this file)
```
