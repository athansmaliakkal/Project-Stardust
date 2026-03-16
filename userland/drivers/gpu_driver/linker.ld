/*
 * Stardust Userland Driver Linker Configuration (GPU Driver)
 * 
 * This script defines the memory layout and symbol mapping for the Stardust 
 * GPU driver. It ensures that the driver's executable image is correctly 
 * structured for loading into a unique, isolated virtual address space.
 *
 * Design:
 * - Address Isolation: Offsets the driver's base address (0x800000) to 
 *   prevent collisions with other userland drivers or core services.
 * - Section Management: Groups binary data into standard ELF sections 
 *   (.text, .rodata, .data, .bss) for correct memory attribute handling.
 * - BSS Tracking: Exports markers (__bss_start, __bss_end) to enable 
 *   automatic zero-initialization of uninitialized variables during bootstrap.
 */

ENTRY(_start)

SECTIONS {
    /*
     * The base address for the GPU driver image. 
     * Offset to 0x800000 to maintain separation from other driver 
     * regions (e.g., PS/2 at 0x400000).
     */
    . = 0x800000;

    /*
     * Executable Code Section
     */
    .text : { 
        *(.text .text.*) 
    }

    /*
     * Read-Only Data (Constants)
     */
    .rodata : { 
        *(.rodata .rodata.*) 
    }

    /*
     * Initialized Read-Write Data
     */
    .data : { 
        *(.data .data.*) 
    }

    /*
     * Uninitialized Read-Write Data (BSS)
     */
    .bss : {
        __bss_start = .;
        *(.bss .bss.*)
        __bss_end = .;
    }
}
