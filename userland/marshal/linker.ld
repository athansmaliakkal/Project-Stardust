/*
 * Stardust OS - God-Process (Marshal) Linker Script
 * 
 * Maps the initialization process at a fixed virtual address and defines
 * section layout for proper execution in Ring 3. Includes BSS bounds
 * for manual zeroing during runtime initialization.
 */

ENTRY(_start)

SECTIONS {
    /* 
     * Base load address for the marshal process. 
     * Typically aligned to standard page boundaries.
     */
    . = 0x400000;
    
    /* Executable code section */
    .text : ALIGN(4K) {
        *(.text .text.*)
    }
    
    /* Read-only data section (strings, constants) */
    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }
    
    /* Initialized data section */
    .data : ALIGN(4K) {
        *(.data .data.*)
    }
    
    /* 
     * Uninitialized data section.
     * __bss_start and __bss_end symbols are exported to allow the
     * entry code to zero-initialize this region before invoking Rust.
     */
    .bss : ALIGN(4K) {
        __bss_start = .;
        *(.bss .bss.*)
        __bss_end = .;
    }
    
    /* Discard unnecessary metadata sections */
    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
    }
}