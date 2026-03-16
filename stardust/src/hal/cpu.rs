/*
 * Core CPU Management Interface.
 *
 * This module defines the `CpuManager` trait, which encapsulates the essential
 * control operations that any supported processor must implement. This includes
 * interrupt management, power states, and hardware-assisted synchronization.
 */

/// The core trait that defines the architectural requirements for a processor.
pub trait CpuManager {
    /// Transitions the CPU to a low-power state until an interrupt occurs.
    fn halt(&self);
    
    /// Mask all maskable hardware interrupts on the current core.
    fn disable_interrupts(&self);
    
    /// Unmask hardware interrupts on the current core.
    fn enable_interrupts(&self);

    /// Sets up a hardware monitor on a specific memory address.
    /// Used in conjunction with `mwait` for ultra-low-latency synchronization.
    fn monitor(&self, ptr: *const core::sync::atomic::AtomicU64);
    
    /// Puts the CPU into an optimized sleep state until the monitored address is modified.
    fn mwait(&self);
}
