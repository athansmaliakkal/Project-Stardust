/*
 * STARDUST KERNEL - BOOT MODULE LOADER AND INITIALIZATION
 * 
 * This module is responsible for locating and initializing the system's 
 * primary user-mode process (the "Marshal") from bootloader-provided modules.
 * 
 * It interfaces with the Limine boot protocol to discover memory-mapped 
 * executable images, parses them using the kernel's ELF loader, and 
 * constructs the initial execution environment, including the user-mode stack.
 */

use crate::oracle;
use limine::request::ModuleRequest;
use x86_64::structures::paging::PageTableFlags;

/// Limine bootloader request for memory-mapped modules.
#[used]
#[unsafe(link_section = ".requests")]
static MODULE_REQUEST: ModuleRequest = ModuleRequest::new();

/// Safe abstraction for the Limine module structure.
/// 
/// This structure represents a file or binary image loaded into memory 
/// by the bootloader before kernel execution began.
#[repr(C)]
pub struct LimineModuleSafe {
    pub revision: u64,
    pub base: *const u8,
    pub length: u64,
    pub path: *const u8,
    pub cmdline: *const u8,
}

/// Initializes the primary user-mode environment.
/// 
/// Scans for the "Marshal" process in RAM, loads its ELF segments into 
/// virtual memory, and allocates a dedicated user-mode stack.
/// 
/// # Returns
/// The virtual address of the process's entry point.
pub fn init() -> u64 {
    oracle::speak("[*] Scanning RAM for Boot Modules...\n");

    // Retrieve the list of modules provided by the bootloader.
    let response = MODULE_REQUEST.get_response().expect("FATAL: Bootloader did not provide modules!");
    let modules = response.modules();

    if modules.is_empty() {
        panic!("FATAL: God-Process (Marshal) missing from RAM!");
    }

    // The first module is traditionally the 'Marshal' (init process).
    let god_process_module = modules[0];
    let safe_module = unsafe { &*(god_process_module as *const _ as *const LimineModuleSafe) };
    
    let base_address = safe_module.base as u64;
    let size = safe_module.length;

    oracle::speak("[+] God-Process (Marshal) located safely in memory!\n");
    oracle::speak("    -> Base Address: 0x");
    oracle::speak_hex(base_address);
    oracle::speak("\n    -> Size: ");
    oracle::speak_u64(size);
    oracle::speak(" bytes\n");

    // Load the ELF image and retrieve its entry point.
    let entry_point = super::elf::load_and_get_entry(base_address);

    // Construct the User-Mode Stack.
    // We allocate a 128KB (32 pages) stack in the high canonical address space.
    // The stack grows downwards from 0x0000_7FFF_FFFF_0000.
    oracle::speak("[*] Constructing 128KB Ring 3 User Stack...\n");
    for i in 1..=32 {
        let phys_frame = crate::memory::frame::ALLOCATOR
            .lock()
            .allocate_frame_raw()
            .expect("FATAL: Out of memory for User Stack!");
            
        let virt_page = 0x0000_7FFF_FFFF_0000 - (i * 4096);
        
        // Map the physical frame to the virtual stack page with User access.
        crate::memory::mapper::map_page(
            phys_frame,
            virt_page,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE
        );
    }
    oracle::speak("[+] Execution Environment completely constructed.\n");

    entry_point
}
