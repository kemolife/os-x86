[bits 16]
; disk_load: load CX sectors from drive DL into ES:0
;
; Caller passes CX = sector count (16-bit, so kernels may exceed 255 sectors),
; ES = load segment (BX is ignored; reads always target ES:0).
; Reads one sector at a time, computing CHS from a linear LBA counter each
; iteration and advancing ES by 0x20 paragraphs (512 bytes). Keeping the
; offset at 0 means a read can never straddle a 64KB segment boundary, which
; is the classic failure mode for large (>64KB) kernels loaded via int 0x13.
;
; Floppy geometry assumed: 18 sectors/track, 2 heads (CHS via 36 = 18*2).
disk_load:
    pusha

    mov [dl_count], cx
    mov [dl_drive], dl
    mov word [dl_lba], 1        ; kernel starts at LBA 1 (after the boot sector)

dl_loop:
    cmp word [dl_count], 0
    je dl_done

    ; --- LBA -> CHS ---
    mov ax, [dl_lba]
    xor dx, dx
    mov cx, 36                  ; 18 sectors * 2 heads per cylinder
    div cx                      ; ax = cylinder, dx = remainder
    mov [dl_cyl], al            ; cylinder (floppy max 79, fits in a byte)

    mov ax, dx                  ; remainder within cylinder
    xor dx, dx
    mov cx, 18
    div cx                      ; ax = head, dx = sector index (0-based)
    mov [dl_head], al
    inc dx                      ; sectors are 1-based
    mov [dl_sect], dl

    ; --- read one sector to ES:0 ---
    mov ah, 0x02
    mov al, 1
    mov ch, [dl_cyl]
    mov cl, [dl_sect]
    mov dh, [dl_head]
    mov dl, [dl_drive]
    xor bx, bx
    int 0x13
    jc disk_error

    ; advance: next destination segment, next LBA, one fewer sector
    mov ax, es
    add ax, 0x20                ; 512 bytes = 0x20 paragraphs
    mov es, ax
    inc word [dl_lba]
    dec word [dl_count]
    jmp dl_loop

dl_done:
    popa
    ret

dl_lba:     dw 0
dl_count:   dw 0
dl_drive:   db 0
dl_cyl:     db 0
dl_head:    db 0
dl_sect:    db 0

disk_error:
    mov bx, DISK_ERROR_MSG
    call print_string
    jmp $

DISK_ERROR_MSG db "Disk read error!", 0
