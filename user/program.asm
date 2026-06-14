; A freestanding ring-3 user program. Talks to the kernel only via int 0x80.
; Linked at virtual address 0x400000 (4MB) — inside the user-accessible
; identity map. Built into an ELF32 executable and placed on the FAT12 disk;
; the kernel's ELF loader reads it, loads its segment, and enters it at ring 3.
[bits 32]
global _start

_start:
    ; write(fd=1, msg, len)
    mov eax, 1              ; SYS_WRITE
    mov ebx, 1             ; fd
    mov ecx, msg
    mov edx, msg_len
    int 0x80

    ; getpid() -> eax
    mov eax, 3             ; SYS_GETPID
    int 0x80

    ; exit(pid) — the kernel prints the code, so we see our own task id
    mov ebx, eax
    mov eax, 2             ; SYS_EXIT
    int 0x80

.hang:
    jmp .hang             ; never reached (exit terminates the task)

msg     db "Hello from an ELF program on disk!", 10
msg_len equ $ - msg
