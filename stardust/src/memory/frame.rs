/*
 * Physical Frame Allocator (Bump Allocator).
 *
 * This module manages the allocation of physical memory pages (frames).
 * It implements a simple bump-allocation strategy based on the memory map
 * provided by the bootloader.
 *
 * Memory Safety:
 * The allocator is protected by a global Mutex to ensure that physical frames
 * are never double-allocated across different cores or threads.
 */

use crate::oracle;
use limine::memory_map::EntryType;
use spin::Mutex;
use super::physical::MEMMAP_REQUEST;

use x86_64::structures::paging::{FrameAllocator as X86FrameAllocator, PhysFrame, Size4KiB};
use x86_64::PhysAddr;

/// Standard x86_64 page size (4 KiB).
pub const PAGE_SIZE: u64 = 4096;

/// State for the physical memory bump allocator.
pub struct FrameAllocator {
    /// The index of the current memory region being allocated from.
    region_index: usize,
    /// The next available physical address within the current region.
    current_addr: u64,
}

impl FrameAllocator {
    /// Creates a new, uninitialized frame allocator.
    pub const fn new() -> Self {
        FrameAllocator { region_index: 0, current_addr: 0 }
    }

    /// Allocates a single 4KB physical frame and returns its raw address.
    /// Iterates through usable memory regions provided by the bootloader.
    pub fn allocate_frame_raw(&mut self) -> Option<u64> {
        let response = MEMMAP_REQUEST.get_response().unwrap();
        let entries = response.entries();

        while self.region_index < entries.len() {
            let entry = &entries[self.region_index];

            // Skip non-usable memory regions (reserved, ACPI, etc.)
            if entry.entry_type != EntryType::USABLE {
                self.region_index += 1;
                continue;
            }

            // Initialize current_addr to the base of the region if it's 0.
            if self.current_addr == 0 {
                self.current_addr = entry.base;
            }

            // Ensure 4KB alignment.
            let aligned_addr = (self.current_addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
            let region_end = entry.base + entry.length;

            // Check if there is enough space in the current region for a 4KB frame.
            if aligned_addr + PAGE_SIZE <= region_end {
                self.current_addr = aligned_addr + PAGE_SIZE;
                return Some(aligned_addr);
            } else {
                // Move to the next memory region.
                self.region_index += 1;
                self.current_addr = 0;
            }
        }
        None
    }
}

/// Implement the x86_64 crate's FrameAllocator trait.
/// This allows the `x86_64` structures (like OffsetPageTable) to use our
/// allocator when they need to allocate frames for new page tables.
unsafe impl X86FrameAllocator<Size4KiB> for FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.allocate_frame_raw().map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

/// The global system frame allocator instance.
pub static ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator::new());

/// Initializes the frame allocator and performs a brief connectivity test.
pub fn init() {
    oracle::speak("[*] Initializing Physical Frame Allocator...\n");
    let mut alloc = ALLOCATOR.lock();
    
    // Diagnostic: Attempt to allocate the first few frames to verify memory map integrity.
    for i in 1..=3 {
        if let Some(frame) = alloc.allocate_frame_raw() {
            oracle::speak("[+] Allocated 4KB Frame ");
            oracle::speak_u64(i);
            oracle::speak(" at physical address: 0x");
            oracle::speak_hex(frame);
            oracle::speak("\n");
        }
    }
}
