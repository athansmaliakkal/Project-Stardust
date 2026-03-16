/*
 * x86_64 System Call Interface and Dispatcher.
 *
 * This module implements the `SYSCALL`/`SYSRET` fast-path mechanism for
 * transitioning between userspace (Ring 3) and kernelspace (Ring 0).
 *
 * Architectural Design:
 * Stardust follows a microkernel-inspired design where most services are
 * accessed via IPC or specific capability-guarded system calls.
 */

use core::arch::naked_asm;
use x86_64::registers::model_specific::Msr;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::oracle;

// Model Specific Registers (MSRs) for SYSCALL configuration.
const IA32_EFER: u32 = 0xC0000080;
const IA32_STAR: u32 = 0xC0000081;
const IA32_LSTAR: u32 = 0xC0000082;
const IA32_FMASK: u32 = 0xC0000084;

// System Call Identification Numbers.
pub const SYS_GRANT_SHARED_MEMORY: u64 = 1;
pub const SYS_IOMMU_LOCKDOWN: u64 = 2;
pub const SYS_MAP_FRAMEBUFFER: u64 = 3;
pub const SYS_BATCH_EXECUTE: u64 = 4; 
pub const SYS_PORT_IN: u64 = 5; 
pub const SYS_CHECK_IRQ: u64 = 6; 
pub const SYS_SPAWN_PROCESS: u64 = 7;
pub const SYS_PORT_OUT: u64 = 8;
pub const SYS_IPC_SEND: u64 = 9;
pub const SYS_IPC_RECEIVE: u64 = 10;

/// Global mailbox for simple cross-process communication.
pub static GLOBAL_IPC_MAILBOX: AtomicU64 = AtomicU64::new(0);

/// Validates whether a userspace pointer is within the canonical lower-half
/// of the address space to prevent kernel-space exploitation.
fn is_safe_user_ptr(vaddr: u64, size: u64) -> bool {
    let end_addr = vaddr.saturating_add(size);
    end_addr < 0x0000_7FFF_FFFF_F000 && vaddr < end_addr
}

/// Configures the processor's MSRs to enable the SYSCALL instruction.
fn configure_msrs() {
    let selectors = &crate::arch::x86_64::gdt::GDT.1;
    
    // The STAR MSR contains the segment selectors for the kernel and userspace code/data.
    let kernel_base = selectors.kernel_code.0 as u64; 
    let user_base = (selectors.user_data.0 - 8) as u64; 
    let star_value = (user_base << 48) | (kernel_base << 32);

    unsafe {
        // Enable syscall/sysret in EFER.
        let mut efer = Msr::new(IA32_EFER); efer.write(efer.read() | 1);
        // Set the STAR MSR with the appropriate selectors.
        let mut star = Msr::new(IA32_STAR); star.write(star_value);
        // Set the LSTAR MSR to the entry point of our assembly stub.
        let mut lstar = Msr::new(IA32_LSTAR); lstar.write(syscall_entry as *const () as usize as u64);
        // Set FMASK to ensure interrupts are disabled during syscall entry (rflags bit 9).
        let mut fmask = Msr::new(IA32_FMASK); fmask.write(0x200); 
    }
}

pub fn init() { configure_msrs(); }
pub fn init_ap() { configure_msrs(); }

/// Low-level assembly entry point for the SYSCALL instruction.
/// This function preserves the userspace register state before calling the Rust dispatcher.
#[unsafe(naked)]
pub extern "C" fn syscall_entry() {
    naked_asm!(
        "push rcx", "push r11", "push rbp", "push rbx", "push r12", "push r13", "push r14", "push r15",
        "call syscall_dispatcher",
        "pop r15", "pop r14", "pop r13", "pop r12", "pop rbx", "pop rbp", "pop r11", "pop rcx",
        "sysretq"
    );
}

/// The primary system call dispatcher.
///
/// Dispatches execution to the appropriate kernel service based on the `syscall_id`.
/// Arguments are passed in registers according to the x86_64 ABI.
#[unsafe(no_mangle)]
pub extern "C" fn syscall_dispatcher(syscall_id: u64, arg1: u64, arg2: u64) -> u64 {
    match syscall_id {
        SYS_GRANT_SHARED_MEMORY => {
            if !is_safe_user_ptr(arg2, 4096) { return u64::MAX; }
            crate::memory::shared::grant_frame(arg1, arg2)
        },
        SYS_IOMMU_LOCKDOWN => {
            if !is_safe_user_ptr(arg2, 0x200000) { return u64::MAX; }
            if crate::arch::x86_64::iommu::map_device_dma(arg1 as u16, arg2) { 1 } else { 0 }
        },
        SYS_MAP_FRAMEBUFFER => {
            if let Some((phys_addr, _width, height, pitch)) = crate::hal::display::get_framebuffer_details() {
                let size_in_bytes = height * pitch;
                if !is_safe_user_ptr(arg1, size_in_bytes) { return u64::MAX; }
                let pages = (size_in_bytes + 4095) / 4096;
                use x86_64::structures::paging::PageTableFlags;
                let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::NO_CACHE;
                for i in 0..pages {
                    crate::memory::mapper::map_page(phys_addr + (i * 4096), arg1 + (i * 4096), flags);
                }
                (pitch << 32) | height
            } else { 0 }
        },
        SYS_BATCH_EXECUTE => {
            let batch_ptr = arg1 as *const u64;
            let count = arg2 as usize;
            if !is_safe_user_ptr(arg1, (count * 8) as u64) { return u64::MAX; }
            
            oracle::speak("\n[*] IPC ROUTER: Processing Batched Syscalls...\n");
            unsafe {
                for i in 0..count {
                    let cmd = core::ptr::read(batch_ptr.add(i));
                    let action = cmd >> 32;
                    let payload = cmd & 0xFFFF_FFFF;
                    oracle::speak("    -> Action 0x");
                    oracle::speak_hex(action);
                    oracle::speak(", Payload 0x");
                    oracle::speak_hex(payload);
                    oracle::speak("\n");
                }
            }
            0
        },
        SYS_PORT_IN => {
            let port = arg1 as u16;
            let size = arg2;
            let mut val: u32 = 0;
            unsafe {
                match size {
                    1 => { let mut v: u8; core::arch::asm!("in al, dx", out("al") v, in("dx") port, options(nomem, nostack, preserves_flags)); val = v as u32; },
                    2 => { let mut v: u16; core::arch::asm!("in ax, dx", out("ax") v, in("dx") port, options(nomem, nostack, preserves_flags)); val = v as u32; },
                    4 => { core::arch::asm!("in eax, dx", out("eax") val, in("dx") port, options(nomem, nostack, preserves_flags)); },
                    _ => return u64::MAX,
                }
            }
            val as u64
        },
        SYS_CHECK_IRQ => {
            let vector = arg1 as usize;
            if vector < 256 {
                crate::arch::x86_64::idt::IRQ_MAILBOXES[vector].swap(0, Ordering::SeqCst)
            } else { 0 }
        },
        SYS_SPAWN_PROCESS => {
            crate::task::scheduler::spawn_thread(arg1, arg2);
            0
        },
        SYS_PORT_OUT => {
            let port = (arg1 & 0xFFFF) as u16;
            let size = (arg1 >> 16) & 0xFFFF;
            unsafe {
                match size {
                    1 => core::arch::asm!("out dx, al", in("dx") port, in("al") arg2 as u8, options(nomem, nostack, preserves_flags)),
                    2 => core::arch::asm!("out dx, ax", in("dx") port, in("ax") arg2 as u16, options(nomem, nostack, preserves_flags)),
                    4 => core::arch::asm!("out dx, eax", in("dx") port, in("eax") arg2 as u32, options(nomem, nostack, preserves_flags)),
                    _ => return u64::MAX,
                }
            }
            0
        },
        SYS_IPC_SEND => {
            GLOBAL_IPC_MAILBOX.store(arg2, Ordering::SeqCst);
            0
        },
        SYS_IPC_RECEIVE => {
            GLOBAL_IPC_MAILBOX.swap(0, Ordering::SeqCst)
        },
        _ => {
            oracle::speak("[!] IPC ROUTER: Unknown Syscall intercepted!\n");
            0 
        }
    }
}
