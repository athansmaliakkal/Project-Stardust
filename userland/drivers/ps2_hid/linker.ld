/*
 * Stardust Userland Driver Linker Configuration (PS/2 HID Driver)
 * 
 * This script defines the memory layout and symbol mapping for the Stardust 
 * PS/2 HID driver. It ensures that the driver's executable image is correctly 
 * structured for loading into a unique, isolated virtual address space.
 *
 * Design:
 * - Address Isolation: Offsets the driver's base address (0x800000) to 
 *   prevent collisions with other userland drivers or core services.
 * - Section Management: Groups binary data into standard ELF sections 
 *   (.text, .rodata, .data, .bss) for correct memory attribute handling.
 * - BSS Tracking: Exports markers (__bss_start, __bss_end) to enable 
 *   automatic zero-initialization of uninitialized variables during bootstrap.
 * - Resource Optimization: Discards unnecessary sections like exception 
 *   frame metadata to reduce executable size.
 */

ENTRY(_start)

SECTIONS
{
    /*
     * The base address for the PS/2 HID driver image. 
     * Offset to 0x800000 to maintain separation from other driver 
     * regions or system services.
     */
    . = 0x800000;

    /*
     * Executable Code Section (4KB Aligned for paging)
     */
    .text : ALIGN(4K) { 
        *(.text .text.*) 
    }

    /*
     * Read-Only Data (Constants)
     */
    .rodata : ALIGN(4K) { 
        *(.rodata .rodata.*) 
    }

    /*
     * Initialized Read-Write Data
     */
    .data : ALIGN(4K) { 
        *(.data .data.*) 
    }
    
    /*
     * Uninitialized Read-Write Data (BSS)
     */
    .bss : ALIGN(4K) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /*
     * Resource Optimization: Discard unnecessary metadata sections.
     */
    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
    }
}
