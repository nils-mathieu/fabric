OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(x86-64)
ENTRY(_)

PHDRS
{
    text    PT_LOAD    FLAGS((1 << 0) | (1 << 2)); /* Execute + Read */
    rodata  PT_LOAD    FLAGS((1 << 2));            /* Read only */
    data    PT_LOAD    FLAGS((1 << 1) | (1 << 2)); /* Write + Read */
}

SECTIONS
{
    . = 0xffffffff80000000;
    PROVIDE(__fabric_image_begin = .);

    .limine_reqs : {
        KEEP(*(.limine_reqs))
    }

    .text : {
        *(.text .text.*)
    } :text

    . = ALIGN(4096);

    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    . = ALIGN(4096);

    .data : {
        *(.data .data.*)
    } :data

    .bss : {
        *(COMMON)
        *(.bss .bss.*)
    } :data

    PROVIDE(__fabric_image_end = .);

    . = ALIGN(4096);

    PROVIDE(__fabric_public_data_address = .);

    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
    }
}
