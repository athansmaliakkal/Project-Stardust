/*
 * Shared Memory and Inter-Process Communication (IPC) primitives.
 *
 * This module provides the mechanism for sharing memory frames between the kernel
 * and userspace, or between different processes. Access is gated by the Security
 * Engine (CDT).
 *
 * Mechanism:
 * 1. A process provides a Security Token for verification.
 * 2. The kernel allocates a physical frame from the global allocator.
 * 3. The frame is mapped into the requested virtual address space with appropriate flags.
 */

use crate::oracle;
use crate::security::token::{Token, RIGHT_GOD};
use crate::security::cdt::CDT;
use x86_64::structures::paging::PageTableFlags;

/// Grants a physical frame to a process at a specific virtual address.
/// Requires a valid Security Token with appropriate rights (currently RIGHT_GOD).
/// 
/// Returns the virtual address where the frame was mapped, or 0 on failure.
pub fn grant_frame(token_id: u64, requested_vaddr: u64) -> u64 {
    let token = Token { id: token_id, rights: RIGHT_GOD };
    
    let tree = CDT.lock();
    if !tree.validate(0, token) { 
        oracle::speak("[!] IPC REJECTED: Invalid Security Token!\n");
        return 0; 
    }
    
    // Allocation of physical backplane.
    // We lock the global frame allocator to retrieve a free page.
    let phys_frame = crate::memory::frame::ALLOCATOR
        .lock()
        .allocate_frame_raw()
        .expect("FATAL: Out of Physical Memory!");
    
    // Define memory protection flags.
    // PRESENT: Page is available in memory.
    // WRITABLE: Process can read and write to this frame.
    // USER_ACCESSIBLE: Allows Ring 3 access (userspace).
    let flags = PageTableFlags::PRESENT 
              | PageTableFlags::WRITABLE 
              | PageTableFlags::USER_ACCESSIBLE;
              
    // Update the active page tables to include this new mapping.
    crate::memory::mapper::map_page(phys_frame, requested_vaddr, flags);
    
    requested_vaddr
}
