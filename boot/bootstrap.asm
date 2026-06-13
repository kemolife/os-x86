; A boot sector that enters 32 - bit protected mode.
[org 0x7c00]
KERNEL_OFFSET equ 0x10000 ; Load the kernel at 64KB. Must be ABOVE the boot
                          ; sector (0x7c00) so a large (>~26KB) kernel does not
                          ; overwrite this code while disk_load is still running.

mov [BOOT_DRIVE], dl   ; BIOS stores our boot drive in DL ,for later use.

mov bp, 0x9000         ; Set the stack.
mov sp, bp             ; cp base poiter to stack poiter
mov bx, MSG_REAL_MODE  ; move message about real mode to base register
call print_string      ; print message about real mode from bx (base register)
call load_kernel       ; Load kernel( C code )
call switch_to_pm      ; switch from 16 bit real mode to 32 bit portected mode 
                       ; for protect system from kernal user space programs
jmp $

%include "real_mode_routines/print/print_string.asm"
%include "boot/disk_load.asm"
%include "boot/global_descriptor_table.asm"
%include "protected_mode_routines/print/print_string.asm"
%include "boot/switch_to_protected_mode.asm"

[bits 16]
; load_kernel
load_kernel :
    mov bx, MSG_LOAD_KERNEL ; Print a message to say we are loading the kernel
    call print_string
    mov ax, KERNEL_OFFSET >> 4 ; ES = 0x1000 -> physical 0x10000 (BX can't hold
    mov es, ax                 ; a 20-bit offset, so address via the segment)
    xor bx, bx
    mov dh, 241             ; load 241 sectors (~120KB Rust kernel) excluding
    mov dl, [BOOT_DRIVE]    ; the boot sector, from the boot disk to KERNEL_OFFSET.
                            ; Keep this >= ceil(kernel.bin / 512); max 255.
    call disk_load
    ret

[bits 32]
; This is where we arrive after switching to and initialising protected mode.
BEGIN_PM:
    mov ebx , MSG_PROT_MODE ; Use our 32 - bit print routine to
    call print_string_pm    ; announce we are in protected mode
    call KERNEL_OFFSET      ; Now jump to the address of our loaded
                            ; kernel code , assume the brace position.
    jmp $

; Global variables
BOOT_DRIVE db 0
MSG_REAL_MODE db "Started in 16 - bit Real Mode", 0
MSG_PROT_MODE db "Successfully landed in 32 - bit Protected Mode", 0
MSG_LOAD_KERNEL db "Loading kernel into memory." , 0

; Bootsector padding
times 510-($-$$) db 0
dw 0xaa55