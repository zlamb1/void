.text

.global _start
.type _start, @function

_start:
    cli
    call _init
    mov $0, %rbp
    call kernel_main
.size _start, . - _start

_init:
    mov $_boot_idt, %rdi
.rept 32
    mov $_boot_isr_stub_\+, %rsi
    movw %si, (%rdi)
    shr $16, %rsi
    movw $0x28, 2(%rdi)
    movw $0x8F00, 4(%rdi)
    movw %si, 6(%rdi)
    shr $16, %rsi
    movl %esi, 8(%rdi)
    add $16, %rdi
.endr
    lidt _boot_idtr
    ret

.rept 32
_boot_isr_stub_\+:
    and $-16, %rsp
    movq $\+, %rdi
    call boot_exception_handler
    ud2
.endr

.section .data

.align 0x10
_boot_idt:
.rept 32
    .quad 0
    .quad 0
.endr
_boot_idt_end:

.align 0x10
_boot_idtr:
    .hword (_boot_idt_end - _boot_idt) - 1
    .quad _boot_idt