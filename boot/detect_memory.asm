[bits 16]
; detect_memory: query the BIOS E820 memory map (int 0x15, EAX=0xE820).
;
; Must run in real mode (before the protected-mode switch). Results are left
; in low memory for the kernel to read once it is running:
;   [MMAP_COUNT]   dword  number of entries stored
;   [MMAP_ENTRIES] array  of 24-byte entries: base(8) length(8) type(4) attr(4)
MMAP_COUNT   equ 0x8000
MMAP_ENTRIES equ 0x8004

detect_memory:
    pusha
    xor ax, ax
    mov es, ax                  ; ES = 0, so ES:DI addresses the 0x8000 region
    mov dword [MMAP_COUNT], 0

    mov di, MMAP_ENTRIES
    xor ebx, ebx                ; EBX = continuation value, 0 = start
    xor bp, bp                  ; BP = entry counter
    mov edx, 0x534D4150         ; 'SMAP'
    mov eax, 0xE820
    mov dword [es:di + 20], 1   ; default ACPI "valid" attribute for old BIOSes
    mov ecx, 24
    int 0x15
    jc .done                    ; carry on first call = E820 unsupported
    cmp eax, 0x534D4150         ; BIOS echoes 'SMAP' in EAX on success
    jne .done
    jmp .process

.next:
    mov eax, 0xE820
    mov dword [es:di + 20], 1
    mov ecx, 24
    int 0x15
    jc .done                    ; carry = end of list reached
.process:
    jcxz .skip                  ; 0-length BIOS reply, ignore
    cmp cl, 20                  ; 24-byte (ACPI 3.0) entry?
    jbe .store                  ; no extended attrs -> always keep
    test byte [es:di + 20], 1   ; extended attr bit 0 clear -> ignore entry
    je .skip
.store:
    mov eax, [es:di + 8]        ; length low
    or  eax, [es:di + 12]       ; | length high
    jz .skip                    ; zero-length region -> skip
    inc bp
    add di, 24                  ; advance to next buffer slot
.skip:
    test ebx, ebx               ; EBX = 0 means that was the last entry
    jne .next
.done:
    mov [MMAP_COUNT], bp
    popa
    ret
