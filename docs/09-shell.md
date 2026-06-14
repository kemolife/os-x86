# 09 — Kernel Command Shell

## Why we need it

A shell is the interactive face of the OS — it turns key presses into actions
and ties every subsystem together: memory stats, the task list, the filesystem,
and the ELF loader. Ours is a *kernel* shell (it runs at ring 0, inside the
keyboard interrupt handler). A later step is a *user-space* shell — a program
in ring 3 that reads input via syscalls.

## File structure

```
src/shell.rs           command parsing + dispatch
src/drivers/keyboard.rs lowercase + Shift layout, line buffering
```

## How it works

The keyboard driver buffers characters until Enter, then calls the registered
line handler — `shell::run`. `run` parses the first word as a command and the
rest as an argument, then dispatches. Output goes to both the VGA screen (what
you see) and the serial log.

The keyboard now tracks **Shift** and has two layouts (lowercase / shifted), so
you type normally. Filenames are converted to FAT 8.3 form (`hello.txt` ->
`HELLO   TXT`) before lookup.

## Commands

| Command | What it does | Subsystem |
|---------|--------------|-----------|
| `help` | list commands | — |
| `mem` | PMM frames free/total, heap free/used/blocks | [memory](04-memory.md) |
| `ps` | list tasks (id + state) | [multitasking](05-multitasking.md) |
| `ls` | list files on the FAT12 disk | [filesystem](07-filesystem.md) |
| `cat <file>` | print a file's contents | filesystem |
| `run <file>` | load + run an ELF program in ring 3 | [user space](08-userspace.md) |
| `uptime` | seconds since boot | timer |
| `clear` | clear the screen | screen |

`run` doesn't exec inline (that would hijack the keyboard-IRQ context into ring
3). Instead it spawns a task that ELF-execs the file, so the program runs under
the scheduler and the shell stays responsive.

## How to test it

The shell is interactive, so run with a display:

```bash
# build kernel + user program + a FAT12 disk with both files
docker run --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 bash -c '
  make && make user.elf
  dd if=/dev/zero of=fat.img bs=512 count=2880 2>/dev/null
  mkfs.fat -F 12 fat.img >/dev/null 2>&1
  echo "hello world" > h.txt; mcopy -i fat.img h.txt ::HELLO.TXT
  mcopy -i fat.img bin/user/init.elf ::INIT.ELF'

# run with a curses display so you can type
docker run -it --rm --platform=linux/amd64 -v "$(pwd)":/os -w /os os-x86 \
  qemu-system-i386 -m 128 -boot a -drive file=os-image.bin,format=raw,if=floppy \
  -hda fat.img -display curses
```

At the `>` prompt try: `help`, `mem`, `ps`, `ls`, `cat hello.txt`, `run init.elf`.

**What it means:** typing flows keyboard IRQ -> driver -> `shell::run` ->
command. `ls`/`cat` exercise the ATA driver + FAT12. `run` spawns a task that
loads an ELF and drops to ring 3, where it talks back only through `int 0x80`
syscalls. `ps` then shows that task as `finished`. One prompt, the whole OS.
