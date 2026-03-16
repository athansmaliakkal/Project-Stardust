use crate::oracle;
use super::thread::{Thread, ThreadState};
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::arch::ArchitectureCpu;
use crate::hal::cpu::CpuManager;

const MAX_THREADS: usize = 64;

pub struct Scheduler {
    threads: [Thread; MAX_THREADS],
    pub thread_count: u64,
    pub ticks: u64,
}

impl Scheduler {
    pub const fn new() -> Self {
        Scheduler {
            threads: [Thread::empty(); MAX_THREADS],
            thread_count: 0,
            ticks: 0,
        }
    }

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

    pub fn pop_ready_thread(&mut self) -> Option<(u64, u64)> {
        for i in 0..MAX_THREADS {
            if self.threads[i].state == ThreadState::Ready {
                self.threads[i].state = ThreadState::Running;
                return Some((self.threads[i].instruction_pointer, self.threads[i].stack_pointer));
            }
        }
        None
    }

    pub fn tick(&mut self) {
        self.ticks += 1;
    }
}

pub static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

pub fn spawn_thread(entry_point: u64, stack_ptr: u64) {
    let mut sched = SCHEDULER.lock();
    sched.spawn_thread(1, stack_ptr, entry_point);
}

pub fn init() {
    oracle::speak("[*] Booting the Round-Robin Process Scheduler...\n");
}

#[derive(Debug, PartialEq, Eq)]
pub enum CpuRole {
    ControlPlane,
    ComputeNode,
}

pub static SUPERVISOR_MAILBOX: AtomicU64 = AtomicU64::new(0);
static mut GLOBAL_IDTR: [u8; 10] = [0; 10];

pub fn lock_core0_as_supervisor(god_entry_point: u64) -> ! {
    oracle::speak("\n[*] CORE 0: Boot sequence complete. Locking into Supervisor Mode...\n");
    
    unsafe { core::arch::asm!("sidt [{}]", in(reg) &raw mut GLOBAL_IDTR); }
    
    let god_stack = 0x0000_7FFF_FFFF_0000;
    SCHEDULER.lock().spawn_thread(0, god_stack, god_entry_point);

    let cpu = ArchitectureCpu::new();
    unsafe { core::arch::asm!("sti"); }
    
    loop {
        cpu.monitor(&SUPERVISOR_MAILBOX);
        if SUPERVISOR_MAILBOX.load(Ordering::SeqCst) == 0 {
            cpu.mwait();
        }
        
        let msg = SUPERVISOR_MAILBOX.swap(0, Ordering::SeqCst);
        if msg != 0 {
            match msg {
                1 => oracle::speak("[+] CORE 0: Handshake acknowledged.\n"),
                2 => {
                    let target_mask: u32 = 0b0000; 
                    crate::arch::x86_64::apic::broadcast_tlb_shootdown(0x4000_1000, target_mask);
                },
                _ => {}
            }
        }
    }
}

pub fn lock_ap_as_worker(logical_core_id: u32) -> ! {
    unsafe { core::arch::asm!("lidt [{}]", in(reg) &raw const GLOBAL_IDTR); }
    crate::arch::x86_64::syscall::init_ap();
    
    unsafe {
        let mut cr0: u64;
        core::arch::asm!("mov {}, cr0", out(reg) cr0);
        cr0 &= !(1 << 2); 
        cr0 |= 1 << 1;    
        core::arch::asm!("mov cr0, {}", in(reg) cr0);

        core::arch::asm!("clts"); 

        let mut cr4: u64;
        core::arch::asm!("mov {}, cr4", out(reg) cr4);
        
        // FIX: Removed unnecessary parentheses to clear compiler warnings
        cr4 |= 1 << 9;  
        cr4 |= 1 << 10; 
        
        core::arch::asm!("mov cr4, {}", in(reg) cr4);

        core::arch::asm!("fninit");
    }
    
    unsafe { 
        use x86_64::registers::model_specific::Msr;
        let mut apic_base_msr = Msr::new(0x1B);
        let mut val = apic_base_msr.read();
        val &= !(1 << 11); 
        apic_base_msr.write(val);
        core::arch::asm!("cli"); 
    }
    
    if logical_core_id > 3 {
        loop { unsafe { core::arch::asm!("hlt"); } }
    }
    
    oracle::speak("\n[*] CORE ");
    oracle::speak_u64(logical_core_id as u64);
    oracle::speak(" ONLINE: Compute Node ready for tasks.\n");

    loop {
        let mut sched = SCHEDULER.lock();
        if let Some((ep, sp)) = sched.pop_ready_thread() {
            drop(sched); 
            
            oracle::speak("\n[*] CORE ");
            oracle::speak_u64(logical_core_id as u64);
            oracle::speak(": Task Assignment Received! Dropping to Ring 3...\n");
            
            crate::task::process::drop_to_user_mode(ep, sp);
        }
        drop(sched);
        
        unsafe { core::arch::asm!("pause"); }
    }
}