[bits 32]
; void switch_context(uint32_t *save_old_esp, uint32_t new_esp)
;
; Saves the current kernel context on the current stack, records the resulting
; ESP through `save_old_esp`, then switches to `new_esp` and restores the
; context previously saved there. Returning "into" a brand-new task works
; because task setup hand-crafts a matching frame (popa block, eflags, entry).
global switch_context
switch_context:
    pushf
    pusha                   ; EDI ESI EBP ESP EBX EDX ECX EAX (32 bytes)
    mov eax, [esp + 40]     ; arg0: save_old_esp (4 ret +4 ... +36 pushed)
    mov edx, [esp + 44]     ; arg1: new_esp
    mov [eax], esp          ; *save_old_esp = current ESP
    mov esp, edx            ; switch to the new stack
    popa
    popf
    ret
