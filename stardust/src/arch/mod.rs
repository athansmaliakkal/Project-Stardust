/*
 * Architecture Abstraction Module.
 *
 * This module provides the bridge between the architecture-independent kernel
 * and the specific hardware implementation (e.g., x86_64). It exports the
 * generic `ArchitectureCpu` type and initialization hooks.
 */

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

/// Re-export the architecture-specific CPU manager.
#[cfg(target_arch = "x86_64")]
pub use x86_64::cpu::X86Cpu as ArchitectureCpu;

/// Architecture-independent hardware initialization for the BSP.
#[cfg(target_arch = "x86_64")]
pub fn init_hardware() {
    x86_64::init_hardware();
}

/// Architecture-independent hardware initialization for APs.
#[cfg(target_arch = "x86_64")]
pub fn init_ap_hardware(logical_id: u32) {
    x86_64::init_ap_hardware(logical_id);
}

/// Global APIC initialization hook.
#[cfg(target_arch = "x86_64")]
pub fn init_apic() {
    x86_64::init_apic();
}

/// AP-specific APIC initialization hook.
#[cfg(target_arch = "x86_64")]
pub fn init_ap_apic() {
    x86_64::init_ap_apic();
}

/// Architecture-specific feature initialization for APs.
#[cfg(target_arch = "x86_64")]
pub fn init_ap_features() {
    x86_64::init_ap_features();
}
