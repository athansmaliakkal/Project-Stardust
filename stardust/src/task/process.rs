/*
 * STARDUST KERNEL - PROCESS MANAGEMENT AND PRIVILEGE TRANSITION
 * 
 * This module handles the high-level process lifecycle and the critical 
 * transition between kernel-mode (CPL0) and user-mode (CPL3).
 * 
 * The transition to user-mode is performed using the `IRETQ` instruction, 
 * which restores the execution state (CS, RIP, SS, RSP, RFLAGS) from a 
 * manually constructed stack frame. This is a one-way transition from 
 * the perspective of this function.
 */

use core::arch::asm;
use crate::oracle;

/// Transition the current processor core from Ring 0 to Ring 3.
/// 
/// This function performs the final stage of task dispatching by constructing 
/// an IRETQ stack frame and jumping to the provided entry point in user space.
/// 
/// # Parameters
/// * `entry_point`: The virtual address where user-mode execution should begin.
/// * `stack_pointer`: The virtual address of the user-mode stack.
/// 
/// # Safety
/// This function never returns. It completely replaces the current kernel 
/// execution context with a user-mode context.
pub fn drop_to_user_mode(entry_point: u64, stack_pointer: u64) -> ! {
    oracle::speak("\n[*] INITIATING PRIVILEGE TRANSITION TO RING 3...\n");

    // Enable Lazy FPU Context Switching.
    // By setting the Task Switched (TS) bit in CR0, any subsequent attempt 
    // to execute an FPU/SSE instruction in user mode will trigger a 
    // Device Not Available (#NM) exception. This allows the kernel to 
    // defer expensive floating-point state saves/restores until actually 
    // required by the task.
    crate::arch::x86_64::cpu::set_task_switched();

    // Segment selectors for Ring 3. 
    // These must correspond to the entries in the Global Descriptor Table (GDT).
    // The bits 0-1 represent the requested privilege level (RPL 3).
    const USER_DATA_SEGMENT: u64 = 0x1B; // GDT index 3, RPL 3
    const USER_CODE_SEGMENT: u64 = 0x23; // GDT index 4, RPL 3
    
    // RFLAGS_IF (bit 9) enables hardware interrupts in the target context.
    // RFLAGS_RESERVED (bit 1) must always be set.
    const DEFAULT_RFLAGS: u64 = 0x202;

    unsafe {
        // Construct the IRETQ stack frame.
        // The hardware expects the following order (top to bottom):
        // [RIP] [CS] [RFLAGS] [RSP] [SS]
        asm!(
            "cli",                   // Ensure interrupts are disabled during frame construction.
            "push {user_data}",      // SS (Stack Segment)
            "push {stack}",          // RSP (Stack Pointer)
            "push {rflags}",         // RFLAGS
            "push {user_code}",      // CS (Code Segment)
            "push {entry_point}",    // RIP (Instruction Pointer)
            "iretq",                 // Perform the transition.
            user_data = in(reg) USER_DATA_SEGMENT,
            stack = in(reg) stack_pointer,
            rflags = in(reg) DEFAULT_RFLAGS,
            user_code = in(reg) USER_CODE_SEGMENT,
            entry_point = in(reg) entry_point,
            options(noreturn)
        );
    }
}
