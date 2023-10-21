[org 0x7c00]
call print_string

mov bx, boot_message
call print_char

mov bx, exit_message
call print_char

jmp $

%include "print_string.asm"

;Data
boot_message:
    db "Booting ", 0
exit_message:
    db "Exit", 0


times 510 -( $ - $$ ) db 0
dw 0xaa55