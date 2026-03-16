/*
 * Virtual Memory Mapping and Page Table Management.
 *
 * This module provides the high-level interface for manipulating the processor's
 * Memory Management Unit (MMU). It leverages the `x86_64` crate's `OffsetPageTable`
 * to perform safe and efficient mapping of physical frames to virtual pages.
 *
 * Architecture:
 * The kernel uses an Offset Page Table strategy, where the entire physical
 * address space is mapped into a contiguous virtual range (the HHDM). This
 * allows the kernel to access any page table in physical memory by simply
 * adding the HHDM offset to its physical address.
 */

use x86_64::structures::paging::{OffsetPageTable, PageTable, PhysFrame, Size4KiB, Mapper, Page, PageTableFlags};
use x86_64::{VirtAddr, PhysAddr};
use spin::Mutex;
use crate::memory::paging::HHDM_OFFSET;
use core::sync::atomic::Ordering;
use crate::oracle;

/// The global OffsetPageTable instance.
/// Wrapped in a Mutex for thread-safe access to the system's page tables.
pub static MAPPER: Mutex<Option<OffsetPageTable<'static>>> = Mutex::new(None);

/// Initializes the virtual memory mapper.
/// This function captures the active PML4 table and initializes the `OffsetPageTable`
/// which will be used for all subsequent memory mapping operations.
pub fn init() {
    oracle::speak("[*] Initializing Virtual Memory Mapper...\n");
    
    // Retrieve the HHDM offset calculated during paging initialization.
    let hhdm = HHDM_OFFSET.load(Ordering::SeqCst);
    let phys_offset = VirtAddr::new(hhdm);
    
    // Read the current PML4 physical address from the CR3 register.
    let (level_4_table_frame, _) = x86_64::registers::control::Cr3::read();

    // Calculate the virtual address of the PML4 table using the HHDM offset.
    let phys = level_4_table_frame.start_address();
    let virt = phys_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    
    // Safety: The bootloader guarantees that the page table is valid and mapped.
    let level_4_table = unsafe { &mut *page_table_ptr };
    let mapper = unsafe { OffsetPageTable::new(level_4_table, phys_offset) };
    
    *MAPPER.lock() = Some(mapper);
    oracle::speak("[+] MMU seized. Virtual Memory Mapper online.\n");
}

/// Establishes a mapping between a physical address and a virtual address.
///
/// Parameters:
/// - `phys`: The starting physical address of the frame.
/// - `virt`: The target virtual address for the mapping.
/// - `flags`: Hardware protection flags (e.g., PRESENT, WRITABLE, USER_ACCESSIBLE).
///
/// This function automatically allocates new page tables if they are required
/// to complete the mapping.
pub fn map_page(phys: u64, virt: u64, flags: PageTableFlags) {
    let phys_frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(phys));
    let page = Page::<Size4KiB>::containing_address(VirtAddr::new(virt));
    
    let mut mapper_guard = MAPPER.lock();
    let mapper = mapper_guard.as_mut().expect("Mapper not initialized!");
    
    // The frame allocator is used if the mapper needs to create new page table levels.
    let mut frame_allocator = crate::memory::frame::ALLOCATOR.lock();
    
    unsafe {
        // Perform the mapping and flush the TLB (Translation Lookaside Buffer) to
        // ensure the changes take effect immediately on the CPU.
        match mapper.map_to(page, phys_frame, flags, &mut *frame_allocator) {
            Ok(flusher) => flusher.flush(),
            Err(_) => {
                // Mapping failed or already exists.
            }
        }
    }
}
