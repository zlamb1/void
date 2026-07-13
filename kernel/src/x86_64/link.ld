SECTIONS {
    __kernel_vma = 0xffffffff80000000;
    . = __kernel_vma;

    .text : ALIGN(0x1000) {
        *(.text) *(.text.*)
    }
    .data : ALIGN(0x1000) {
        *(.data) *(.data.*)
    }
    .rodata : ALIGN(0x1000) {
        *(.rodata) *(.rodata.*)
    }
    .bss : ALIGN(0x1000) {
        *(.bss) *(.bss.*)
    }
}