;hello.asm 
section .data
    msg 
section .bss
section .text global main
main:
mov rax, 1 ; 1 = запись.
mov rdi, 1 mov rsi, msg mov rdx, 12 syscall
mov rax, 60 mov rdi, 0 syscall
db
"hello, world",0
; 1 = в поток стандартного вывода stdout. ; Выводимая строка в регистре rsi.
; Длина строки без конечного 0.
; Вывод строки.
; 60 = код выхода из программы.
; 0 = код успешного завершения программы. ; Выход из программы.