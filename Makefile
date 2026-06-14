# Two kernels share one boot + HAL (oscore). Build/run either separately:
#   make mono      / make run-mono        — monolithic kernel (shell, fs, ...)
#   make micro     / make run-micro       — microkernel (IPC echo demo)
GDB = gdb
LD  = i686-linux-gnu-ld

RUST_TARGET = i686-kernel
CARGO_BUILD = cargo +nightly build \
    -Z json-target-spec \
    --target $(RUST_TARGET).json \
    -Z build-std=core,compiler_builtins,alloc \
    -Z build-std-features=compiler-builtins-mem \
    --release

ASM_OBJ = boot/kernel_entry.o cpu/interrupt.o cpu/switch.o
LDFLAGS = --gc-sections

.PHONY: mono micro run-mono run-micro user.elf clean dir

# ---- monolithic kernel ----
mono: os-image-mono.bin

os-image-mono.bin: boot/bootstrap.bin $(ASM_OBJ) | dir
	$(CARGO_BUILD) -p mono
	$(LD) -T kernel.ld $(LDFLAGS) -o bin/kernel/mono.bin $(ASM_OBJ) \
	    target/$(RUST_TARGET)/release/libmono.a --oformat binary
	cat boot/bootstrap.bin bin/kernel/mono.bin > os-image-mono.bin
	truncate -s 1474560 os-image-mono.bin

run-mono: mono
	qemu-system-i386 -m 128 -drive file=os-image-mono.bin,format=raw,if=floppy -nographic

# ---- microkernel ----
micro: os-image-micro.bin

os-image-micro.bin: boot/bootstrap.bin $(ASM_OBJ) | dir
	$(CARGO_BUILD) -p micro
	$(LD) -T kernel.ld $(LDFLAGS) -o bin/kernel/micro.bin $(ASM_OBJ) \
	    target/$(RUST_TARGET)/release/libmicro.a --oformat binary
	cat boot/bootstrap.bin bin/kernel/micro.bin > os-image-micro.bin
	truncate -s 1474560 os-image-micro.bin

run-micro: micro
	qemu-system-i386 -m 128 -drive file=os-image-micro.bin,format=raw,if=floppy -nographic

# Default = the monolithic kernel.
all: mono

# Freestanding ring-3 user program (ELF32) for the monolithic kernel's `run`.
user.elf:
	mkdir -p bin/user
	nasm -f elf32 user/program.asm -o bin/user/program.o
	$(LD) -m elf_i386 -Ttext 0x40000000 -e _start -o bin/user/init.elf bin/user/program.o

%.o: %.asm | dir
	nasm $< -f elf -o $@

%.bin: %.asm | dir
	nasm $< -f bin -o $@

dir:
	mkdir -p bin/boot bin/kernel

clean:
	rm -rf bin os-image-mono.bin os-image-micro.bin
	rm -f boot/*.bin boot/*.o cpu/*.o
	cargo clean
