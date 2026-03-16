/*
 * Virtual Memory Paging and MMU Configuration.
 *
 * This module sets up the fundamental virtual memory environment for the kernel.
 * It manages the Higher Half Direct Map (HHDM), which allows the kernel to
 * access all physical memory through a constant virtual offset.
 */

use limine::request::HhdmRequest;
use x86_64::registers::control::Cr3;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::oracle;

/// Limine HHDM Request.
/// The bootloader will provide a virtual address offset that maps to physical address 0.
#[used]
#[unsafe(link_section = ".requests")]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

/// Global offset for the Higher Half Direct Map.
/// Used for translating physical addresses to virtual addresses and vice versa.
/// Example: VirtualAddr = PhysicalAddr + HHDM_OFFSET
pub static HHDM_OFFSET: AtomicU64 = AtomicU64::new(0);

/// Initializes the paging subsystem and captures the HHDM offset.
pub fn init() {
    oracle::speak("[*] Initializing Virtual Memory Pager (MMU)...\n");

    let hhdm_res = HHDM_REQUEST.get_response().expect("FATAL: Bootloader did not provide HHDM offset!");
    let offset = hhdm_res.offset();
    
    // Atomically store the offset for system-wide use.
    HHDM_OFFSET.store(offset, Ordering::SeqCst);

    oracle::speak("[+] Higher Half Direct Map (HHDM) Offset: 0x");
    oracle::speak_hex(offset);
    oracle::speak("\n");

    // Read the current PML4 (Level 4 Page Table) from the CR3 register.
    let (level_4_table_frame, _cr3_flags) = Cr3::read();
    
    oracle::speak("[+] Active PML4 Page Table physical address: 0x");
    oracle::speak_hex(level_4_table_frame.start_address().as_u64());
    oracle::speak("\n");
}
