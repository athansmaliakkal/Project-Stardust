/*
 * Hardware Abstraction Layer (HAL).
 *
 * The HAL provides a consistent interface for the kernel to interact with
 * underlying hardware, regardless of the specific processor architecture.
 * This allows the higher-level kernel logic to remain portable and agnostic
 * of low-level hardware details.
 */

pub mod cpu;
pub mod display;

/// High-level interface for processor control operations.
pub trait CpuManager {
    /// Puts the processor into a halt state.
    fn halt(&self);
    
    /// Disables hardware interrupts on the local processor.
    fn disable_interrupts(&self);
    
    /// Enables hardware interrupts on the local processor.
    fn enable_interrupts(&self);
}
