SECTIONS {
    __kernel_vma = 0xffffffff80000000;
    . = __kernel_vma;

    .text : {
        *(.text) *(.text.*)
    }
    .data : {
        *(.data) *(.data.*)
    }
    .rodata : {
        *(.rodata) *(.rodata.*)
    }
    .bss : {
        *(.bss) *(.bss.*)
    }
}