mov ah, 0x0e
mov al, 0x41
int 0x10

loop:
    cmp al, 0x7A
    je exit
    cmp al, 0x61
    jl lowc
    jg uperc
    jmp loop
   
uperc:
    inc al
    sub al, 0x20
    int 0x10
    jmp loop

lowc:
    inc al
    add al, 0x20  
    int 0x10
    jmp loop

exit:
    jmp $

times 510-($-$$) db 0
dw 0xAA55