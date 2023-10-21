mov ah, 0x0e
mov bx, 10000
; if (bx <= 4) {
; mov al , ’A ’
; } else if (bx < 40) {
; mov al , ’B ’
; } else {
; mov al , ’C ’
; }
cmp bx, 4
jle less_equal
cmp bx, 40
jl less
jmp else

less_equal:
    mov al, 'A'
    jmp the_end
less:
    mov al, 'B'
    jmp the_end
else:
    mov al, 'C'
    jmp the_end

the_end:
    int 0x10
    jmp $

times 510 -( $ - $$ ) db 0
dw 0xaa55