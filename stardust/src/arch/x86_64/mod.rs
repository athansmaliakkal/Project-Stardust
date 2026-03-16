/*
 * x86_64 Architecture Support.
 *
 * This module consolidates all x86_64-specific sub-components, providing a
 * unified interface for initializing architecture-dependent hardware features.
 */

pub mod cpu;
pub mod gdt;
pub mod idt;
pub mod apic;
pub mod syscall;
pub mod iommu; 

/// Initializes core hardware components for the bootstrap processor (BSP).
pub fn init_hardware() {
    gdt::init();
    idt::init();
    syscall::init();
}

/// Initializes hardware components for application processors (APs).
/// Ensures that each core has its own GDT and syscall configuration.
pub fn init_ap_hardware(logical_id: u32) {
    gdt::init_ap(logical_id);
    idt::init();
    syscall::init_ap();
}

/// Sets up the Local APIC for the current core.
pub fn init_apic() {
    apic::init();
}

/// AP-specific Local APIC initialization.
pub fn init_ap_apic() {
    apic::init_ap();
}

/// Enables advanced architectural features like PCIDs and IOMMU.
pub fn init_features() {
    cpu::enable_pcid();
    iommu::init(); 
}

/// AP-specific architectural feature initialization.
pub fn init_ap_features() {
    cpu::enable_pcid_ap();
}
