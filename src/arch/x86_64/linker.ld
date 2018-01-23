ENTRY(start)

SECTIONS {
    . = 1M;

    .rodata : {
        KEEP(*(.multiboot_header))
        *(.rodata .rodata.*)
        . = ALIGN(4K);
    }

    .text : {
        *(.text .text.*)
        . = ALIGN(4K);
    }

    .data.rel.ro : {
        *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
        . = ALIGN(4K);
    }

    .bss : {
        *(.bss .bss.*)
        . = ALIGN(4K);
    }
}