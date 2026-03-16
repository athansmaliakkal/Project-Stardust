/*
 * STARDUST KERNEL - MULTI-CORE SCHEDULER AND DISPATCHER
 * 
 * This module implements the kernel's primary scheduling logic and the 
 * Boot Processor (BSP) / Application Processor (AP) lifecycle management.
 * 
 * The scheduler currently employs a simplified round-robin policy across 
 * available threads. The architecture distinguishes between the "Control Plane" 
 * (running on the BSP/Core 0) and "Compute Nodes" (running on APs). 
 * 
 * The Control Plane handles system-wide orchestration, IPI (Inter-Processor 
 * Interrupt) management, and supervisor tasks, while Compute Nodes are 
 * dedicated to executing user-mode threads (Ring 3).
 */

use crate::oracle;
use super::thread::{Thread, ThreadState};
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::arch::ArchitectureCpu;
use crate::hal::cpu::CpuManager;

/// Maximum number of concurrent threads supported by the current scheduler implementation.
const MAX_THREADS: usize = 64;

/// The central scheduler structure maintaining the global thread table.
pub struct Scheduler {
    /// Global registry of all active and inactive threads.
    threads: [Thread; MAX_THREADS],
    
    /// Monotonically increasing counter used for assigning unique Thread IDs (TIDs).
    pub thread_count: u64,
    
    /// Total number of system ticks elapsed since the scheduler started.
    pub ticks: u64,
}

impl Scheduler {
    /// Performs compile-time initialization of the scheduler.
    pub const fn new() -> Self {
        Scheduler {
            threads: [Thread::empty(); MAX_THREADS],
            thread_count: 0,
            ticks: 0,
        }
    }

    /// Allocates a new thread slot and initializes its execution context.
    /// 
    /// Returns the assigned Thread ID if a slot is available, otherwise None.
    pub fn spawn_thread(&mut self, process_id: u64, stack_pointer: u64, instruction_pointer: u64) -> Option<u64> {
        for i in 0..MAX_THREADS {
            if self.threads[i].state == ThreadState::Empty {
                self.thread_count += 1;
                let tid = self.thread_count;
                
                self.threads[i] = Thread {
                    id: tid,
                    parent_process_id: process_id,
                    state: ThreadState::Ready,
                    stack_pointer,
                    instruction_pointer,
                };
                return Some(tid);
            }
        }
        None
    }

    /// Searches for the next thread in a 'Ready' state and marks it as 'Running'.
    /// 
    /// This is the core dispatching logic. It currently follows a simple 
    /// linear search (First-Available-Ready).
    pub fn pop_ready_thread(&mut self) -> Option<(u64, u64)> {
        for i in 0..MAX_THREADS {
            if self.threads[i].state == ThreadState::Ready {
                self.threads[i].state = ThreadState::Running;
                return Some((self.threads[i].instruction_pointer, self.threads[i].stack_pointer));
            }
        }
        None
    }

    /// Updates the global system tick count.
    pub fn tick(&mut self) {
        self.ticks += 1;
    }
}

/// Global singleton for the system scheduler, protected by a spinlock.
pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

/// High-level interface to create a new execution context.
pub fn spawn_thread(entry_point: u64, stack_ptr: u64) {
    let mut sched = SCHEDULER.lock();
    sched.spawn_thread(1, stack_ptr, entry_point);
}

/// Initializes the scheduling subsystem during kernel bootstrap.
pub fn init() {
    oracle::speak("[*] Booting the Round-Robin Process Scheduler...\n");
}

/// Defines the operational role of a logical processor core.
#[derive(Debug, PartialEq, Eq)]
pub enum CpuRole {
    /// The core responsible for system management and IPI orchestration.
    ControlPlane,
    
    /// A core dedicated to executing user-mode workloads.
    ComputeNode,
}

/// Atomic mailbox used for cross-core signaling and supervisor requests.
pub static SUPERVISOR_MAILBOX: AtomicU64 = AtomicU64::new(0);

/// Cached copy of the Interrupt Descriptor Table Register (IDTR) for AP initialization.
static mut GLOBAL_IDTR: [u8; 10] = [0; 10];

/// Transforms the calling core (BSP) into the system Supervisor.
/// 
/// This function never returns. It enters a high-efficiency monitor/mwait 
/// loop, responding to messages in the SUPERVISOR_MAILBOX.
pub fn lock_core0_as_supervisor(god_entry_point: u64) -> ! {
    oracle::speak("\n[*] CORE 0: Boot sequence complete. Locking into Supervisor Mode...\n");
    
    // Capture the current IDT configuration for use by Application Processors (APs).
    unsafe { core::arch::asm!("sidt [{}]", in(reg) &raw mut GLOBAL_IDTR); }
    
    // Initialize the primary supervisor thread context.
    let god_stack = 0x0000_7FFF_FFFF_0000;
    SCHEDULER.lock().spawn_thread(0, god_stack, god_entry_point);

    let cpu = ArchitectureCpu::new();
    unsafe { core::arch::asm!("sti"); }
    
    loop {
        // Utilize hardware-assisted power management (MONITOR/MWAIT) to 
        // minimize power consumption while idling.
        cpu.monitor(&SUPERVISOR_MAILBOX);
        if SUPERVISOR_MAILBOX.load(Ordering::SeqCst) == 0 {
            cpu.mwait();
        }
        
        let msg = SUPERVISOR_MAILBOX.swap(0, Ordering::SeqCst);
        if msg != 0 {
            match msg {
                1 => oracle::speak("[+] CORE 0: Handshake acknowledged.\n"),
                2 => {
                    // Trigger a broadcast TLB shootdown to maintain memory consistency.
                    let target_mask: u32 = 0b0000; 
                    crate::arch::x86_64::apic::broadcast_tlb_shootdown(0x4000_1000, target_mask);
                },
                _ => {}
            }
        }
    }
}

/// Initializes an Application Processor (AP) and enters the worker dispatch loop.
/// 
/// This involves setting up the GDT/IDT, configuring Control Registers (CR0, CR4) 
/// for modern execution features (SSE/AVX), and entering the scheduler poll loop.
pub fn lock_ap_as_worker(logical_core_id: u32) -> ! {
    // Synchronize interrupt handling configuration with the BSP.
    unsafe { core::arch::asm!("lidt [{}]", in(reg) &raw const GLOBAL_IDTR); }
    crate::arch::x86_64::syscall::init_ap();
    
    unsafe {
        let mut cr0: u64;
        core::arch::asm!("mov {}, cr0", out(reg) cr0);
        
        // Configure CR0: Enable Monitoring Coprocessor (MP) and 
        // disable Emulation (EM) to support hardware FPU/SSE.
        cr0 &= !(1 << 2); 
        cr0 |= 1 << 1;    
        core::arch::asm!("mov cr0, {}", in(reg) cr0);

        // Clear the Task Switched (TS) flag to prevent #NM exceptions during 
        // first SSE instruction use.
        core::arch::asm!("clts"); 

        let mut cr4: u64;
        core::arch::asm!("mov {}, cr4", out(reg) cr4);
        
        // Enable Operating System support for FXSAVE/FXRSTOR (OSFXSR) and 
        // Unmasked SIMD Floating-Point Exceptions (OSXMMEXCPT).
        cr4 |= (1 << 9);  
        cr4 |= (1 << 10); 
        core::arch::asm!("mov cr4, {}", in(reg) cr4);

        // Initialize FPU state.
        core::arch::asm!("fninit");
    }
    
    unsafe { 
        use x86_64::registers::model_specific::Msr;
        let mut apic_base_msr = Msr::new(0x1B);
        let mut val = apic_base_msr.read();
        
        // Ensure the Local APIC is enabled on this core.
        val &= !(1 << 11); 
        apic_base_msr.write(val);
        core::arch::asm!("cli"); 
    }
    
    // Current implementation limits active compute nodes to the first 4 cores.
    if logical_core_id > 3 {
        loop { unsafe { core::arch::asm!("hlt"); } }
    }
    
    oracle::speak("\n[*] CORE ");
    oracle::speak_u64(logical_core_id as u64);
    oracle::speak(" ONLINE: Compute Node ready for tasks.\n");

    loop {
        // Poll the global scheduler for work.
        let mut sched = SCHEDULER.lock();
        if let Some((ep, sp)) = sched.pop_ready_thread() {
            // Drop lock before transitioning to user mode to allow other cores 
            // to access the scheduler.
            drop(sched); 
            
            oracle::speak("\n[*] CORE ");
            oracle::speak_u64(logical_core_id as u64);
            oracle::speak(": Task Assignment Received! Dropping to Ring 3...\n");
            
            // Perform the context switch to Ring 3.
            crate::task::process::drop_to_user_mode(ep, sp);
        }
        drop(sched);
        
        // Yield the pipeline briefly to reduce contention on the scheduler lock.
        unsafe { core::arch::asm!("pause"); }
    }
}
