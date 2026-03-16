/*
 * Stardust Userland Dynamic Memory Allocator
 * 
 * This module implements the global heap allocator for userland processes 
 * and libraries. It provides a standard 'malloc/free' interface (via 
 * Rust's GlobalAlloc) backed by the kernel's shared memory grant system.
 *
 * Architecture:
 * 1. Region Management: Manages a fixed virtual address range starting 
 *    at HEAP_START.
 * 2. System Backing: Requests physical frames from the kernel's memory 
 *    manager during initialization.
 * 3. Allocation Strategy: Employs a 'Linked List Allocator' for simple 
 *    and efficient management of small-to-medium objects in a 
 *    constrained environment.
 */

use linked_list_allocator::LockedHeap;

/*
 * Global Userland Allocator
 */
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/*
 * Memory Region Definitions
 * 
 * HEAP_START: Canonical base address for the userland dynamic heap.
 * HEAP_SIZE: Initial memory capacity (64 KB for standard libraries).
 */
pub const HEAP_START: usize = 0x0000_4000_0000_0000;
pub const HEAP_SIZE: usize = 64 * 1024; 

/*
 * init_heap: Initializes the dynamic memory subsystem.
 * 
 * This function must be called during process bootstrap before any 
 * allocations (e.g., Vec, String, Box) are performed.
 */
pub fn init_heap() {
    let pages = HEAP_SIZE / 4096;
    
    // Explicitly populate the virtual address space with physical frames
    for i in 0..pages {
        let vaddr = (HEAP_START + (i * 4096)) as u64;
        if crate::sys_grant_shared_memory(0, vaddr).is_err() {
            // Failure usually indicates the kernel is out of physical 
            // frames or the VMM policy denied the request.
            panic!("FATAL: Kernel refused to grant heap memory to Userland!");
        }
    }

    unsafe {
        // Hand off the mapped memory region to the Linked List Allocator
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }
}
