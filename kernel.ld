ENTRY(_start)
SECTIONS {
    . = 0x10000;
    .text : {
        boot/kernel_entry.o(.text*)
        cpu/interrupt.o(.text*)
        *(.text*)
    }
    .rodata ALIGN(4096) : { *(.rodata*) }
    .data   ALIGN(4096) : { *(.data*)   }
    .bss    ALIGN(4096) : {
        *(.bss*)
        *(COMMON)
    }
    /DISCARD/ : { *(.eh_frame) *(.comment) }
}
