/*
 * Hardware Abstraction Layer for Display and Graphics.
 *
 * This module facilitates communication with the system's graphics hardware.
 * It uses the Limine bootloader's framebuffer request to acquire the physical
 * address and dimensions of the display device.
 */

use limine::request::FramebufferRequest;
use core::sync::atomic::Ordering;

/// Request for a graphical framebuffer from the bootloader.
#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

/// Retrieves the hardware-specific details of the active graphical framebuffer.
///
/// Returns an Option containing:
/// (Physical Address, Width, Height, Pitch/Scanline length in bytes)
pub fn get_framebuffer_details() -> Option<(u64, u64, u64, u64)> {
    if let Some(response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(fb) = response.framebuffers().next() {
            let virt_addr = fb.addr() as u64;
            let hhdm = crate::memory::paging::HHDM_OFFSET.load(Ordering::SeqCst);
            
            // Translate the virtual address provided by Limine back to its
            // raw physical address using the HHDM offset.
            let phys_addr = virt_addr.saturating_sub(hhdm);
            
            return Some((
                phys_addr, 
                fb.width() as u64, 
                fb.height() as u64, 
                fb.pitch() as u64
            ));
        }
    }
    None
}
