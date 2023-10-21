loop:
    jmp loop

; Bootsector padding
times 510 -( $ - $$ ) db 0
dw 0xaa55