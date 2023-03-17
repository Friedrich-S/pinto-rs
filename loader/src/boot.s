.section .boot, "awx"
.global _start

_start:
    # zero segment registers
    sub %ax, %ax
    mov %ax, %ds
    mov %ax, %ss

    # clear the direction flag (e.g. go forward in memory when using
    # instructions like lodsb)
    cld

    # initialize stack
    mov $0xf000, %esp

rust:
    # push arguments
    push %dx     # disk number
    call main