/*
 * Stardust ELF64 Loader Subsystem
 * 
 * This module provides the capability to parse and load Executable and 
 * Linkable Format (ELF) binaries into the process's virtual address space.
 * It specifically targets the 64-bit ELF variant (ELF64) as used by the 
 * Stardust userland.
 *
 * Mechanism:
 * 1. Header Validation: Verifies the ELF magic and basic structure.
 * 2. Program Header Iteration: Scans the Program Header Table for 
 *    loadable segments (PT_LOAD).
 * 3. Virtual Memory Mapping: Uses system calls to request physical 
 *    backing for the segments' virtual address ranges.
 * 4. Segment Loading: Copies the segment data from the binary image 
 *    to the target virtual memory, ensuring BSS (uninitialized data) 
 *    regions are zero-filled.
 */

/*
 * Elf64Ehdr: ELF64 Executable Header
 * 
 * The entry point to the ELF file structure, containing architectural 
 * metadata and offsets to the program and section header tables.
 */
#[repr(C)]
pub struct Elf64Ehdr {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

/*
 * Elf64Phdr: ELF64 Program Header
 * 
 * Describes a segment of the process image. PT_LOAD segments are 
 * mapped into the process's virtual memory.
 */
#[repr(C)]
pub struct Elf64Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

/*
 * load: Parses an ELF binary image and populates the address space.
 * 
 * @param data: The raw bytes of the ELF file.
 * @return: The virtual address of the entry point if successful.
 */
pub fn load(data: &[u8]) -> Option<u64> {
    if data.len() < core::mem::size_of::<Elf64Ehdr>() { return None; }

    let ehdr = unsafe { &*(data.as_ptr() as *const Elf64Ehdr) };
    
    // Validate ELF Magic: 0x7F 'E' 'L' 'F'
    if ehdr.e_ident[0..4] != [0x7F, b'E', b'L', b'F'] { return None; }

    let phdr_ptr = unsafe { data.as_ptr().add(ehdr.e_phoff as usize) } as *const Elf64Phdr;

    for i in 0..ehdr.e_phnum {
        let phdr = unsafe { &*phdr_ptr.add(i as usize) };
        
        /*
         * PT_LOAD (Type 1): Loadable Segment
         * 
         * This segment contains code or data that must be present in 
         * virtual memory during execution.
         */
        if phdr.p_type == 1 {
            // Calculate required page count for this segment
            let pages = (phdr.p_memsz + 4095) / 4096;
            
            // Map the segment into the virtual address space
            for p in 0..pages {
                let vaddr = phdr.p_vaddr + (p * 4096);
                let _ = crate::sys_grant_shared_memory(0, vaddr);
            }

            let dest = phdr.p_vaddr as *mut u8;
            let src = unsafe { data.as_ptr().add(phdr.p_offset as usize) };
            
            unsafe {
                // Copy initialized data from the ELF file
                core::ptr::copy_nonoverlapping(src, dest, phdr.p_filesz as usize);
                
                /*
                 * Zero-Initialize BSS
                 * 
                 * If the segment's memory size is larger than its file size,
                 * the remaining space is considered the BSS section and 
                 * must be zeroed out.
                 */
                if phdr.p_memsz > phdr.p_filesz {
                    let bss_start = dest.add(phdr.p_filesz as usize);
                    let bss_size = (phdr.p_memsz - phdr.p_filesz) as usize;
                    core::ptr::write_bytes(bss_start, 0, bss_size);
                }
            }
        }
    }
    
    // Return the virtual address of the entry point (_start)
    Some(ehdr.e_entry)
}
