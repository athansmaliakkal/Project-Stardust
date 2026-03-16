/*
 * Interrupt Descriptor Table (IDT) and Exception Handling.
 *
 * This module defines how the processor responds to hardware interrupts and
 * software exceptions. It implements a Microkernel-style IRQ notification system.
 *
 * Architecture:
 * Instead of processing hardware events directly in the kernel, Stardust
 * signals waiting userspace drivers by setting bits in the `IRQ_MAILBOXES`.
 */

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::registers::control::Cr2;
use lazy_static::lazy_static;
use crate::oracle::speak;
use crate::arch::x86_64::gdt;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub const TIMER_INTERRUPT_ID: u8 = 32;
pub const KEYBOARD_INTERRUPT_VECTOR: u8 = 33; 
pub const SPURIOUS_INTERRUPT_ID: u8 = 255;

pub static SHOOTDOWN_ADDR: AtomicU64 = AtomicU64::new(0);
pub static SHOOTDOWN_ACK: AtomicUsize = AtomicUsize::new(0);

/// Global array for IRQ notification. Drivers poll or wait on these mailboxes.
pub static IRQ_MAILBOXES: [AtomicU64; 256] = [const { AtomicU64::new(0) }; 256];

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        
        // Standard Processor Exceptions
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.general_protection_fault.set_handler_fn(gpf_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.device_not_available.set_handler_fn(device_not_available_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        
        // Hardware Interrupts
        idt[TIMER_INTERRUPT_ID as usize].set_handler_fn(timer_interrupt_handler);
        idt[crate::arch::x86_64::apic::TLB_SHOOTDOWN_VECTOR as usize].set_handler_fn(tlb_shootdown_handler);
        idt[KEYBOARD_INTERRUPT_VECTOR as usize].set_handler_fn(generic_irq_handler_33);
        idt[SPURIOUS_INTERRUPT_ID as usize].set_handler_fn(spurious_handler);
        idt
    };
}

/// Loads the IDT into the processor.
pub fn init() { IDT.load(); }

/// Generic handler for IRQ 33 (Keyboard). Notifies userspace via the mailbox.
extern "x86-interrupt" fn generic_irq_handler_33(_stack_frame: InterruptStackFrame) {
    IRQ_MAILBOXES[33].store(1, Ordering::SeqCst);
    crate::arch::x86_64::apic::end_of_interrupt();
}

/// Handler for FPU/SSE 'Device Not Available' exceptions.
/// Enables lazy state loading for floating-point registers.
extern "x86-interrupt" fn device_not_available_handler(_stack_frame: InterruptStackFrame) {
    crate::arch::x86_64::cpu::clear_task_switched();
    speak("\n[*] CORE 1: Lazy FPU Exception Caught! Loading Math Registers on-demand...\n");
}

extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {
    speak("\n[!] ORACLE INTERCEPT: Breakpoint Exception Caught!\n");
}

extern "x86-interrupt" fn double_fault_handler(_stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    speak("\n[!!!] FATAL: DOUBLE FAULT.\n");
    crate::hal::cpu::CpuManager::halt(&crate::arch::ArchitectureCpu::new()); loop {}
}

extern "x86-interrupt" fn gpf_handler(_stack_frame: InterruptStackFrame, _error_code: u64) {
    speak("\n[!!!] FATAL: GENERAL PROTECTION FAULT (GPF).\n");
    crate::hal::cpu::CpuManager::halt(&crate::arch::ArchitectureCpu::new()); loop {}
}

/// Critical error handler for illegal memory accesses.
extern "x86-interrupt" fn page_fault_handler(_stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    let arch_cpu = crate::arch::ArchitectureCpu::new();
    crate::hal::cpu::CpuManager::disable_interrupts(&arch_cpu); 
    speak("\n[!!!] FATAL: PAGE FAULT. Unmapped memory accessed at 0x");
    crate::oracle::speak_hex(Cr2::read().as_u64()); speak("\n");
    if error_code.contains(PageFaultErrorCode::USER_MODE) { speak("[+] CONFIRMED: Fault originated from RING 3!\n"); } 
    else { speak("[!] FATAL: Fault originated from RING 0!\n"); }
    crate::hal::cpu::CpuManager::halt(&arch_cpu); loop {}
}

extern "x86-interrupt" fn invalid_opcode_handler(_stack_frame: InterruptStackFrame) {
    speak("\n[!!!] INVALID OPCODE FAULT (#UD)\n");
    crate::hal::cpu::CpuManager::halt(&crate::arch::ArchitectureCpu::new()); loop {}
}

extern "x86-interrupt" fn spurious_handler(_stack_frame: InterruptStackFrame) {}

/// System timer interrupt. Triggers a scheduler tick.
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    crate::task::scheduler::SCHEDULER.lock().tick();
    crate::arch::x86_64::apic::end_of_interrupt();
}

/// Synchronizes TLB state across multiple cores.
extern "x86-interrupt" fn tlb_shootdown_handler(_stack_frame: InterruptStackFrame) {
    let vaddr = SHOOTDOWN_ADDR.load(Ordering::SeqCst);
    x86_64::instructions::tlb::flush(x86_64::VirtAddr::new(vaddr));
    SHOOTDOWN_ACK.fetch_add(1, Ordering::SeqCst);
    crate::arch::x86_64::apic::end_of_interrupt();
}
