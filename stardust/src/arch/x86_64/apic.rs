/*
 * Advanced Programmable Interrupt Controller (APIC) Driver.
 *
 * This module manages the APIC architecture, which has replaced the legacy
 * 8259 PIC in modern x86 systems. It handles both the Local APIC (per-core)
 * and the I/O APIC (system-wide interrupt routing).
 *
 * Architecture:
 * The Local APIC is used for inter-processor interrupts (IPIs) and timers,
 * while the I/O APIC routes hardware interrupts (like keyboard or disk)
 * to specific cores.
 */

use crate::oracle;
use x86_64::registers::model_specific::Msr;
use core::arch::asm;
use core::sync::atomic::Ordering;
use x86_64::structures::paging::PageTableFlags;

/// Calculates the virtual address of the Local APIC registers using the HHDM.
fn get_apic_base() -> u64 {
    let apic_base_msr = Msr::new(0x1B);
    let base_val = unsafe { apic_base_msr.read() };
    let phys_base = base_val & 0xFFFF_FFFF_FFFF_F000;
    let hhdm = crate::memory::paging::HHDM_OFFSET.load(Ordering::SeqCst);
    phys_base + hhdm
}

/// Writes a 32-bit value to a specific APIC register.
unsafe fn apic_write(register_offset: u64, value: u32) {
    let base = get_apic_base();
    let ptr = (base + register_offset) as *mut u32;
    unsafe { core::ptr::write_volatile(ptr, value); }
}

/// Configures the Local APIC registers for interrupt handling and timers.
fn setup_apic_registers() {
    unsafe {
        // Spurious Interrupt Vector Register.
        apic_write(0xF0, 0x100 | 0xFF);
        // LVT Timer Register.
        apic_write(0x3E0, 0x03);
        // LVT Timer Interrupt Vector.
        apic_write(0x320, 32 | 0x20000);
        // Initial Count Register for Timer.
        apic_write(0x380, 10_000_000);
    }
}

/// Initializes the I/O APIC for routing hardware interrupts.
pub fn init_ioapic() {
    let ioapic_phys: u64 = 0xFEC00000;
    let ioapic_virt: u64 = ioapic_phys + crate::memory::paging::HHDM_OFFSET.load(Ordering::SeqCst);
    
    // Map the I/O APIC registers into the kernel's virtual address space.
    crate::memory::mapper::map_page(
        ioapic_phys, 
        ioapic_virt,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE
    );

    /// Writes to an I/O APIC register via the selection/window registers.
    unsafe fn write_ioapic(base: u64, reg: u32, value: u32) {
        let ptr = base as *mut u32;
        unsafe {
            core::ptr::write_volatile(ptr, reg);
            core::ptr::write_volatile(ptr.add(4), value); 
        }
    }

    unsafe {
        // Route IRQ 1 (Keyboard) to Vector 33.
        write_ioapic(ioapic_virt, 0x10 + (1 * 2), 33);    
        write_ioapic(ioapic_virt, 0x10 + (1 * 2) + 1, 0); 
    }
    oracle::speak("[+] I/O APIC Hardware Router configured.\n");
}

/// Performs a full initialization of the APIC subsystem on the BSP.
pub fn init() {
    oracle::speak("[*] Terminating legacy IBM PIC chips...\n");
    unsafe {
        // Mask all interrupts on the legacy PICs to avoid spurious noise.
        asm!("out dx, al", in("dx") 0x21_u16, in("al") 0xFF_u8, options(nomem, nostack, preserves_flags));
        asm!("out dx, al", in("dx") 0xA1_u16, in("al") 0xFF_u8, options(nomem, nostack, preserves_flags));
    }
    oracle::speak("[*] Booting modern Local APIC timer...\n");
    
    let apic_base_msr = Msr::new(0x1B);
    let base_val = unsafe { apic_base_msr.read() };
    let phys_base = base_val & 0xFFFF_FFFF_FFFF_F000;
    let hhdm = crate::memory::paging::HHDM_OFFSET.load(Ordering::SeqCst);
    let virt_base = phys_base + hhdm;

    // Map the Local APIC registers.
    crate::memory::mapper::map_page(
        phys_base, virt_base, 
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE
    );

    setup_apic_registers();
    init_ioapic();
    oracle::speak("[+] Local APIC Heartbeat ignited.\n");
}

/// Initializes the Local APIC for an application processor.
pub fn init_ap() { setup_apic_registers(); }

/// Signals the end of interrupt (EOI) to the Local APIC.
pub fn end_of_interrupt() { unsafe { apic_write(0xB0, 0); } }

/// Vector used for TLB Shootdown inter-processor interrupts.
pub const TLB_SHOOTDOWN_VECTOR: u8 = 251;

/// Broadcasts a TLB shootdown request to other cores.
/// Used to maintain cache coherency when page tables are modified.
pub fn broadcast_tlb_shootdown(vaddr: u64, cpu_mask: u32) {
    crate::arch::x86_64::idt::SHOOTDOWN_ADDR.store(vaddr, Ordering::SeqCst);
    crate::arch::x86_64::idt::SHOOTDOWN_ACK.store(0, Ordering::SeqCst);

    let mut expected_acks = 0;
    for core_id in 1..4 {
        if (cpu_mask & (1 << core_id)) != 0 {
            expected_acks += 1;
            // Send Fixed IPI to the target core.
            let icr_high = (core_id as u32) << 24;
            let icr_low = TLB_SHOOTDOWN_VECTOR as u32; 
            unsafe { 
                apic_write(0x310, icr_high);
                apic_write(0x300, icr_low); 
            }
        }
    }
    // Wait for all targeted cores to acknowledge the TLB flush.
    while crate::arch::x86_64::idt::SHOOTDOWN_ACK.load(Ordering::SeqCst) < expected_acks { core::hint::spin_loop(); }
    oracle::speak("[+] CORE 0: Targeted TLB Shootdown Complete. Innocent cores untouched.\n");
}
