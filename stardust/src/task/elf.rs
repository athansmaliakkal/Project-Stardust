/*
 * STARDUST KERNEL - ELF64 BINARY PARSER AND LOADER
 * 
 * This module implements the kernel's loader for the Executable and Linkable 
 * Format (ELF). It is responsible for validating ELF64 headers, parsing 
 * program headers, and mapping loadable segments (PT_LOAD) into the 
 * process's virtual address space.
 * 
 * The loader also includes a relocation engine to handle binaries that 
 * may have been linked against addresses that conflict with the kernel's 
 * canonical higher-half mapping.
 */

use crate::oracle;
use x86_64::structures::paging::PageTableFlags;

/// Standard ELF64 File Header (Ehdr).
#[repr(C, packed)]
pub struct Elf64Header {
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

/// Standard ELF64 Program Header (Phdr).
#[repr(C, packed)]
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

/// Parses an ELF64 image from memory and maps its segments into virtual memory.
/// 
/// This function performs the following steps:
/// 1. Validates the ELF magic number.
/// 2. Calculates relocation offsets if a memory collision is detected.
/// 3. Iterates through Program Headers to find PT_LOAD segments.
/// 4. Allocates physical frames and maps them to the requested virtual addresses.
/// 5. Copies segment data from the source image to the newly mapped memory.
/// 
/// # Parameters
/// * `base_addr`: The physical/linear address where the ELF image is currently located.
/// 
/// # Returns
/// The virtual address of the entry point (potentially relocated).
pub fn load_and_get_entry(base_addr: u64) -> u64 {
    oracle::speak("[*] Dissecting ELF64 Binary Header...\n");
    let header = unsafe { &*(base_addr as *const Elf64Header) };

    // Validate the ELF Identification (e_ident).
    let magic = &header.e_ident[0..4];
    if magic != [0x7F, b'E', b'L', b'F'] {
        panic!("FATAL: God-Process is corrupted or not a valid ELF binary!");
    }

    oracle::speak("[*] Mapping ELF Segments into User Space...\n");
    
    // Determine if the binary requires relocation.
    // If the entry point is in the kernel's higher-half (>= 0xFFFFFFFF80000000),
    // we relocate it down to user-space (starting at 0x400000).
    let mut entry_point = header.e_entry;
    let is_colliding = header.e_entry >= 0xFFFFFFFF80000000;
    let relocation_offset = if is_colliding {
        0xFFFFFFFF80200000u64.wrapping_sub(0x400000u64)
    } else {
        0
    };

    if is_colliding {
        oracle::speak("[!] Linker collision detected! Force-relocating to User Space...\n");
        entry_point = entry_point.wrapping_sub(relocation_offset);
    }

    let phdr_base = base_addr + header.e_phoff;

    // Iterate through Program Headers.
    for i in 0..header.e_phnum {
        let phdr = unsafe { &*((phdr_base + i as u64 * header.e_phentsize as u64) as *const Elf64Phdr) };
        
        // PT_LOAD (Type 1) indicates a loadable segment.
        if phdr.p_type == 1 {
            let mut orig_vaddr = phdr.p_vaddr & !0xFFF;
            let end_vaddr = (phdr.p_vaddr + phdr.p_memsz + 0xFFF) & !0xFFF;
            
            while orig_vaddr < end_vaddr {
                // Apply the relocation offset to the target virtual address.
                let mapped_vaddr = orig_vaddr.wrapping_sub(relocation_offset);
                
                let phys_frame = crate::memory::frame::ALLOCATOR
                    .lock()
                    .allocate_frame_raw()
                    .expect("FATAL: OOM loading ELF!");
                    
                // Map the segment with USER_ACCESSIBLE permissions.
                let flags = PageTableFlags::PRESENT 
                          | PageTableFlags::WRITABLE 
                          | PageTableFlags::USER_ACCESSIBLE;
                          
                crate::memory::mapper::map_page(phys_frame, mapped_vaddr, flags);
                
                // Zero the memory to ensure a clean state (important for BSS).
                unsafe { core::ptr::write_bytes(mapped_vaddr as *mut u8, 0, 4096); }
                
                orig_vaddr += 4096;
            }
            
            // Copy segment data from the ELF image to the mapped memory.
            if phdr.p_filesz > 0 {
                let src = (base_addr + phdr.p_offset) as *const u8;
                let dst = phdr.p_vaddr.wrapping_sub(relocation_offset) as *mut u8;
                unsafe { core::ptr::copy_nonoverlapping(src, dst, phdr.p_filesz as usize); }
            }
        }
    }

    // Flush the Translation Lookaside Buffer (TLB) to ensure page table 
    // changes are visible to the CPU immediately.
    x86_64::instructions::tlb::flush_all();

    oracle::speak("[+] All ELF Segments mapped securely into Ring 3!\n");
    entry_point
}
