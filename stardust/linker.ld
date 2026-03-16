/*
 * Stardust Kernel Linker Script.
 *
 * This script defines the memory layout of the final kernel binary.
 * It ensures that the kernel is loaded into the higher half of the
 * 64-bit address space (canonical addresses) while being physically
 * placed at the 2MB mark by the bootloader.
 */

ENTRY(_start)

/* Program Headers for different memory protection zones */
PHDRS
{
    text    PT_LOAD    FLAGS((1 << 0) | (1 << 2)) ; /* Execute + Read */
    rodata  PT_LOAD    FLAGS((1 << 2)) ;            /* Read only */
    data    PT_LOAD    FLAGS((1 << 1) | (1 << 2)) ; /* Write + Read */
}

SECTIONS
{
    /* Virtual start address in the higher half direct map (HHDM) */
    . = 0xffffffff80200000;

    /* Executable code */
    .text : {
        *(.text .text.*)
    } :text

    . += CONSTANT(MAXPAGESIZE);

    /* Read-only data (constants, strings, etc.) */
    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    . += CONSTANT(MAXPAGESIZE);

    /* Initialized data */
    .data : {
        *(.data .data.*)
    } :data

    /* Uninitialized data (zeroed by the bootloader) */
    .bss : {
        *(.COMMON)
        *(.bss .bss.*)
    } :data

    /* Discard metadata sections not required for runtime execution */
    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
    }
}
