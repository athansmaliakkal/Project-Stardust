/*
 * x86_64 Processor Control and Instruction Wrappers.
 *
 * This module provides a concrete implementation of the `CpuManager` trait
 * for x86_64 processors. It contains low-level assembly wrappers for
 * fundamental CPU instructions and configuration of architectural features.
 */

use crate::hal::cpu::CpuManager;
use core::arch::asm;
use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};
use crate::oracle;

/// Concrete handle for an x86_64 processor core.
pub struct X86Cpu;

impl X86Cpu {
    pub const fn new() -> Self {
        X86Cpu {}
    }
}

impl CpuManager for X86Cpu {
    /// Puts the processor into a low-power halt state.
    fn halt(&self) {
        unsafe {
            asm!("hlt", options(nomem, nostack, preserves_flags));
        }
    }

    /// Disables maskable interrupts (CLI).
    fn disable_interrupts(&self) {
        unsafe {
            asm!("cli", options(nomem, nostack));
        }
    }

    /// Enables maskable interrupts (STI).
    fn enable_interrupts(&self) {
        unsafe {
            asm!("sti", options(nomem, nostack));
        }
    }

    /// Sets up a hardware monitor on a specific memory address (MONITOR).
    fn monitor(&self, ptr: *const core::sync::atomic::AtomicU64) {
        unsafe {
            core::arch::asm!(
                "monitor",
                in("rax") ptr as usize,
                in("rcx") 0,
                in("rdx") 0,
                options(nostack, preserves_flags)
            );
        }
    }

    /// Puts the CPU into an optimized sleep state until the monitored address is modified (MWAIT).
    fn mwait(&self) {
        unsafe {
            core::arch::asm!(
                "mwait",
                in("rax") 0,
                in("rcx") 0,
                options(nostack, preserves_flags)
            );
        }
    }
}

/// Enables Process-Context Identifiers (PCID) and SSE features.
/// PCIDs allow the CPU to cache TLB entries across address space switches,
/// significantly reducing the performance penalty of context switching.
pub fn enable_pcid() {
    oracle::speak("[*] Checking for PCID hardware support...\n");
    let mut flags = Cr4::read();
    flags |= Cr4Flags::PCID;
    
    // Enable SSE and exception handling for SIMD instructions.
    flags |= Cr4Flags::OSFXSR;
    flags |= Cr4Flags::OSXMMEXCPT_ENABLE; 
    
    unsafe { Cr4::write(flags); }
    oracle::speak("[+] PCID Memory Acceleration enabled in CR4.\n");
    oracle::speak("[+] SSE Hardware Instructions Unlocked in CR4.\n");
}

/// AP-specific feature enablement (PCID, SSE).
pub fn enable_pcid_ap() {
    let mut flags = Cr4::read();
    flags |= Cr4Flags::PCID;
    flags |= Cr4Flags::OSFXSR;
    flags |= Cr4Flags::OSXMMEXCPT_ENABLE; 
    unsafe { Cr4::write(flags); }
}

/// Sets the Task Switched (TS) bit in CR0.
/// Used for lazy FPU state saving/restoring.
pub fn set_task_switched() {
    unsafe {
        Cr0::write(Cr0::read() | Cr0Flags::TASK_SWITCHED);
    }
}

/// Clears the Task Switched (TS) bit in CR0.
pub fn clear_task_switched() {
    unsafe {
        core::arch::asm!("clts", options(nomem, nostack, preserves_flags));
    }
}
