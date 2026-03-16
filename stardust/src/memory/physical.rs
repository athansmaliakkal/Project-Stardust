/*
 * Physical Memory Management and Hardware Probing.
 *
 * This module interacts with the Limine bootloader to retrieve the physical
 * memory map of the system. It identifies usable RAM regions and reports the
 * total capacity of the machine.
 */

use crate::oracle;
use limine::request::MemoryMapRequest;
use limine::memory_map::EntryType;

/// Limine Memory Map Request.
/// Stored in the .requests section to be parsed by the bootloader during early boot.
#[used]
#[unsafe(link_section = ".requests")]
pub static MEMMAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

/// Initializes physical memory discovery.
/// Iterates through the memory map provided by the firmware/bootloader to
/// calculate usable RAM and identify memory holes or reserved regions.
pub fn init() {
    oracle::speak("[*] Probing physical memory hardware...\n");

    let response = MEMMAP_REQUEST.get_response().expect("FATAL: Bootloader did not provide a memory map!");
    let entries = response.entries();

    let mut usable_bytes: u64 = 0;
    let mut total_regions: u64 = 0;

    // Scan all memory entries provided by Limine.
    for entry in entries {
        total_regions += 1;
        // Only count regions explicitly marked as USABLE (available RAM).
        if entry.entry_type == EntryType::USABLE {
            usable_bytes += entry.length;
        }
    }

    let usable_mb = usable_bytes / (1024 * 1024);

    oracle::speak("[+] Physical Memory Map retrieved from Limine.\n");
    oracle::speak("[+] Found ");
    oracle::speak_u64(total_regions);
    oracle::speak(" distinct memory regions.\n");
    oracle::speak("[+] Total Usable RAM: ");
    oracle::speak_u64(usable_mb);
    oracle::speak(" MB\n");
}
