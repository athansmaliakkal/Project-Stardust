/*
 * Symmetric Multi-Processing (SMP) Coordination.
 *
 * This module manages the lifecycle of application processors (APs). It uses
 * the Limine MP request to discover the system's core topology and boots
 * each core into the kernel's worker loop.
 */

use crate::oracle;
use limine::request::MpRequest; 
use limine::mp::Cpu;            
use core::sync::atomic::{AtomicU64, Ordering};
use crate::hal::cpu::CpuManager; 

/// MP Request for Limine to provide CPU topology information.
#[used]
#[unsafe(link_section = ".requests")]
static MP_REQUEST: MpRequest = MpRequest::new();

/// Counter for synchronized AP startup.
static AP_READY_COUNT: AtomicU64 = AtomicU64::new(0);

/// Assigns a unique logical ID to each core as it comes online.
static LOGICAL_CORE_COUNT: AtomicU64 = AtomicU64::new(1); 

/// Discovery and activation of all non-boot processors.
pub fn init() {
    oracle::speak("[*] Interrogating Limine for CPU Core Topology...\n");

    let response = MP_REQUEST.get_response().expect("FATAL: MP not supported!");
    let cpus = response.cpus();
    
    oracle::speak("[+] SMP topology retrieved.\n");
    oracle::speak("[+] Total CPU Cores detected: ");
    oracle::speak_u64(cpus.len() as u64);
    oracle::speak("\n[+] Boot Processor (BSP) APIC ID: ");
    oracle::speak_u64(response.bsp_lapic_id() as u64);
    oracle::speak("\n");

    oracle::speak("[*] Awakening Application Processors (APs)...\n");

    // Tell each AP to jump to the `ap_main` entry point once booted.
    for cpu in cpus {
        if cpu.lapic_id != response.bsp_lapic_id() {
            cpu.goto_address.write(ap_main);
        }
    }

    // Wait for all APs to signal that they have successfully reached `ap_main`.
    let expected_aps = (cpus.len() - 1) as u64;
    while AP_READY_COUNT.load(Ordering::SeqCst) < expected_aps {
        core::hint::spin_loop();
    }

    oracle::speak("[+] All CPU Cores are now online and synchronized!\n");
}

/// Entry point for Application Processors (APs).
/// This function is called by the bootloader on each secondary core.
extern "C" fn ap_main(_info: &Cpu) -> ! {
    // Acquire a unique logical ID for this core.
    let logical_id = LOGICAL_CORE_COUNT.fetch_add(1, Ordering::SeqCst) as u32;

    // Initialize architecture-specific hardware features for this core.
    crate::arch::init_ap_hardware(logical_id);
    crate::arch::init_ap_features();
    crate::arch::init_ap_apic();

    // Signal to the BSP that this core is ready.
    AP_READY_COUNT.fetch_add(1, Ordering::SeqCst);

    let arch_cpu = crate::arch::ArchitectureCpu::new();
    arch_cpu.disable_interrupts();
    
    // Transition this core to the worker loop where it will execute threads.
    crate::task::scheduler::lock_ap_as_worker(logical_id);
}
