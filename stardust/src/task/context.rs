/*
 * STARDUST KERNEL - LOW-LEVEL ARCHITECTURAL CONTEXT SWITCHING
 * 
 * This module implements the assembly-level routine for switching execution 
 * between two threads. It is responsible for preserving and restoring the 
 * processor's register state such that a thread can be suspended and 
 * resumed later without loss of continuity.
 * 
 * The routine follows the x86_64 System V ABI calling convention regarding 
 * "callee-saved" registers. Registers that are not callee-saved (e.g., RAX, 
 * RCX, RDX) are assumed to be volatile and must be saved by the caller or 
 * the interrupt handler.
 */

use core::arch::naked_asm;

/// Performs a low-level context switch between two threads.
/// 
/// This function swaps the processor's stack pointer and preserves the 
/// minimum required register state to maintain execution flow.
/// 
/// # Parameters
/// * `old_stack`: A pointer to a memory location where the current thread's 
///                stack pointer (RSP) will be saved.
/// * `new_stack`: The stack pointer (RSP) of the thread being resumed.
/// 
/// # Safety
/// This is a `naked` function. It does not have a prologue or epilogue. 
/// It relies entirely on manual stack management and register preservation.
#[unsafe(naked)]
pub extern "C" fn switch_threads(_old_stack: *mut u64, _new_stack: u64) {
    // The implementation utilizes `naked_asm!` to ensure no compiler-generated 
    // stack frame interferes with the manual stack swap.
    naked_asm!(
        // 1. Save the Callee-Saved Registers of the CURRENT thread.
        // These are the registers that the System V ABI requires a function 
        // to preserve if it modifies them. By saving them here, we ensure 
        // they are restored when this thread is eventually rescheduled.
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // 2. Save the current stack pointer (RSP) into the old thread's TCB.
        // RDI contains the first argument: `old_stack`.
        "mov [rdi], rsp",

        // 3. Load the new thread's stack pointer into RSP.
        // RSI contains the second argument: `new_stack`.
        "mov rsp, rsi",

        // 4. Restore the Callee-Saved Registers for the NEW thread.
        // This effectively restores the register state to exactly what 
        // it was when the new thread was last suspended.
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",

        // 5. Resume execution.
        // The `ret` instruction pops the RIP (saved on the stack during 
        // the previous call to `switch_threads` or by the scheduler) and 
        // jumps to it, effectively continuing the new thread's execution.
        "ret"
    );
}
