/*
 * STARDUST KERNEL - THREAD MANAGEMENT SYSTEM
 * 
 * This module defines the core thread abstractions for the Stardust executive.
 * In this architecture, a thread represents the smallest unit of schedulable 
 * execution. Unlike processes, which serve as resource containers (address 
 * spaces, security tokens), threads are purely execution contexts consisting 
 * of a register set, a stack, and a scheduling state.
 *
 * The threading model follows a 1:1 mapping with kernel-managed entities, 
 * allowing the scheduler to manage preemption and affinity at the thread level.
 */

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    /// The thread slot is unallocated or has been reaped.
    Empty,
    
    /// The thread is eligible for execution and is currently in the 
    /// scheduler's runqueue awaiting CPU time.
    Ready,
    
    /// The thread is currently dispatched to a processor core and 
    /// is actively executing.
    Running,
    
    /// The thread is suspended awaiting an external event, such as 
    /// I/O completion, a mutex release, or an IPC message.
    Blocked,
}

/// Represents the execution context of a single thread.
/// 
/// The Thread structure maintains the hardware-level state required to 
/// perform context switches. It is decoupled from the Process structure 
/// to allow for multi-threaded processes where multiple threads share 
/// the same virtual memory space but maintain independent execution paths.
#[derive(Debug, Clone, Copy)]
pub struct Thread {
    /// Unique identifier for the thread across the entire system.
    pub id: u64,
    
    /// The identifier of the process that owns this thread and 
    /// provides its virtual memory context.
    pub parent_process_id: u64,
    
    /// Current scheduling state of the thread.
    pub state: ThreadState,
    
    /// The saved stack pointer (RSP on x86_64) used to restore the 
    /// execution state during a context switch.
    pub stack_pointer: u64,
    
    /// The instruction pointer (RIP on x86_64) representing the 
    /// next instruction to be executed when the thread is resumed.
    pub instruction_pointer: u64, 
}

impl Thread {
    /// Initializes a null thread structure.
    /// 
    /// This is typically used to pre-allocate thread slots in the 
    /// global thread table or to reset a thread slot during reaping.
    pub const fn empty() -> Self {
        Thread {
            id: 0,
            parent_process_id: 0,
            state: ThreadState::Empty,
            stack_pointer: 0,
            instruction_pointer: 0,
        }
    }
}
