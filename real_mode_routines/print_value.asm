[org 0x7c00]

mov dx, 0x1fb7    ; Set the value we want to print to dx
call print_hex    ; Print the hex value
jmp $             ; Hang once we're done

%include "print/print_hex.asm"

; Padding and magic number
times 510-($-$$) db 0
dw 0xaa55