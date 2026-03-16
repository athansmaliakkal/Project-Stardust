/*
 * Stardust Kernel Entry Point.
 *
 * This is the primary initialization sequence for the Stardust OS.
 * The kernel begins execution here after being loaded by the Limine bootloader.
 * It coordinates the bring-up of all major subsystems: memory management,
 * security, multi-processing, and task scheduling.
 */

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

pub mod arch;
pub mod hal;
pub mod oracle;
pub mod memory;
pub mod smp;
pub mod security;
pub mod task;

use core::panic::PanicInfo;
use crate::hal::cpu::CpuManager;
use crate::arch::ArchitectureCpu;

/// Limine Base Revision Request.
#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: limine::BaseRevision = limine::BaseRevision::new();

/// The Global Panic Handler.
/// Captured when the kernel encounters an unrecoverable error.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    oracle::speak("\n[!] KERNEL PANIC: The system has halted.\n");
    let cpu = ArchitectureCpu::new();
    loop {
        cpu.halt();
    }
}

/// The Kernel Entry Point (_start).
/// Transitioned to from the bootloader in 64-bit Long Mode.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let cpu = ArchitectureCpu::new();

    // Verify bootloader compatibility.
    if !BASE_REVISION.is_supported() {
        loop { cpu.halt(); }
    }
    
    oracle::speak("\n========================================\n");
    oracle::speak("         STARDUST IS ALIVE!             \n");
    oracle::speak("========================================\n");
    
    // Disable interrupts during the critical boot sequence.
    cpu.disable_interrupts();
    
    // 1. Architecture-specific hardware initialization (GDT, IDT, Syscalls).
    arch::init_hardware();
    
    // 2. Memory Management Subsystem (Physical Map, Frame Allocator, Paging).
    memory::physical::init();
    memory::frame::init();
    memory::paging::init();
    memory::mapper::init();
    
    // 3. Security Engine (Capability Derivation Tree).
    security::cdt::init();
    
    // 4. Advanced CPU features (PCID, SSE).
    arch::x86_64::init_features();
    
    // 5. Symmetric Multi-Processing and Local APICs.
    arch::init_apic();
    smp::init();
    
    // 6. Task Scheduler and Userspace Loader.
    task::scheduler::init();
    let god_entry_point = task::loader::init();

    oracle::speak("[*] Engine Completion Block Finished.\n");
    oracle::speak("[+] Activating Hardware Heartbeat...\n");

    // Lock Core 0 into the Supervisor role and start the first process on Core 1.
    crate::task::scheduler::lock_core0_as_supervisor(god_entry_point);
}
