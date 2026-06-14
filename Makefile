# Rust kernel build (Docker / Linux toolchain)
GDB = gdb
LD  = i686-linux-gnu-ld

RUST_TARGET = i686-kernel
RUST_LIB    = target/$(RUST_TARGET)/release/libkernel.a
CARGO_BUILD = cargo +nightly build \
    -Z json-target-spec \
    --target $(RUST_TARGET).json \
    -Z build-std=core,compiler_builtins,alloc \
    -Z build-std-features=compiler-builtins-mem \
    --release

BIN     = bin/boot bin/kernel
ASM_OBJ = boot/kernel_entry.o cpu/interrupt.o cpu/switch.o

os-image.bin: boot/bootstrap.bin kernel.bin
	cat boot/bootstrap.bin bin/kernel/kernel.bin > os-image.bin
	# pad to a standard 1.44MB floppy (2880 * 512) so QEMU is happy and
	# reads past the kernel return zeros instead of end-of-disk errors
	truncate -s 1474560 os-image.bin

LDFLAGS = --gc-sections

kernel.bin: $(ASM_OBJ) | dir
	$(CARGO_BUILD)
	$(LD) -T kernel.ld $(LDFLAGS) -o bin/kernel/$@ $(ASM_OBJ) $(RUST_LIB) --oformat binary

kernel.elf: $(ASM_OBJ) | dir
	$(CARGO_BUILD)
	$(LD) -T kernel.ld $(LDFLAGS) -o bin/kernel/$@ $(ASM_OBJ) $(RUST_LIB)

run: os-image.bin
	qemu-system-i386 -drive file=os-image.bin,format=raw,if=floppy -curses

# Build the freestanding ring-3 user program as an ELF32 executable, linked at
# 4MB. Goes on a FAT12 data disk (as INIT.ELF), not into the OS image.
user.elf:
	mkdir -p bin/user
	nasm -f elf32 user/program.asm -o bin/user/program.o
	$(LD) -m elf_i386 -Ttext 0x400000 -e _start -o bin/user/init.elf bin/user/program.o

%.o: %.asm | dir
	nasm $< -f elf -o $@

%.bin: %.asm | dir
	nasm $< -f bin -o $@

dir:
	mkdir -p $(BIN)

clean:
	rm -rf bin os-image.bin
	rm -f boot/*.bin boot/*.o cpu/*.o
	cargo clean
