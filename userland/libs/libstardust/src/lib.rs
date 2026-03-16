/*
 * libstardust: Stardust System Runtime Library
 * 
 * This library serves as the primary interface between userland processes 
 * and the Stardust kernel. It encapsulates the raw system call interface 
 * (x86_64 syscall instruction) and provides high-level abstractions for 
 * system services.
 *
 * System Call ABI (x86_64):
 * - Service ID: RDI
 * - Argument 1: RSI
 * - Argument 2: RDX
 * - Return Value: RAX
 * - Clobbered: RCX, R11 (by hardware)
 */

#![no_std]

extern crate alloc;
use core::arch::asm;

pub mod tar;
pub mod elf;

/*
 * Inter-Process Communication (IPC) Message Structure
 * 
 * Defines the standard layout for structured data exchange between
 * isolated address spaces.
 */
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IpcMessage {
    pub sender_id: u32,
    pub target_id: u32,
    pub action: u16,
    pub flags: u16,
    pub payload: [u8; 52],
}

/*
 * System Call Service Identifiers
 */
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

/*
 * sys_grant_shared_memory: Requests a memory grant from the kernel.
 * Maps physical frames into the caller's address space.
 */
#[inline]
pub fn sys_grant_shared_memory(process_id: u64, virtual_address: u64) -> Result<(), ()> {
    let result: u64;
    unsafe { asm!("syscall", inout("rdi") SYS_GRANT_SHARED_MEMORY => _, inout("rsi") process_id => _, inout("rdx") virtual_address => _, out("rax") result, out("rcx") _, out("r11") _, out("r8") _, out("r9") _, out("r10") _); }
    if result == u64::MAX { Err(()) } else { Ok(()) }
}

/*
 * sys_iommu_lockdown: Configures IOMMU protection for a specific device.
 */
#[inline]
pub fn sys_iommu_lockdown(device_id: u16, virtual_address: u64) -> u64 {
    let result: u64;
    unsafe { asm!("syscall", inout("rdi") SYS_IOMMU_LOCKDOWN => _, inout("rsi") device_id as u64 => _, inout("rdx") virtual_address => _, out("rax") result, out("rcx") _, out("r11") _, out("r8") _, out("r9") _, out("r10") _); }
    result
}

/*
 * sys_map_framebuffer: Requests the linear framebuffer address from the kernel.
 * Returns metadata (pitch, height) and maps the FB to virtual_address.
 */
#[inline]
pub fn sys_map_framebuffer(virtual_address: u64) -> u64 {
    let result: u64;
    unsafe { asm!("syscall", inout("rdi") SYS_MAP_FRAMEBUFFER => _, inout("rsi") virtual_address => _, out("rdx") _, out("rax") result, out("rcx") _, out("r11") _, out("r8") _, out("r9") _, out("r10") _); }
    result
}

/*
 * sys_batch_execute: Executes multiple kernel commands in a single transition.
 * Reduces the overhead of syscall context switching.
 */
#[inline]
pub fn sys_batch_execute(commands_ptr: *const u64, count: usize) -> u64 {
    let result: u64;
    unsafe { asm!("syscall", inout("rdi") SYS_BATCH_EXECUTE => _, inout("rsi") commands_ptr as u64 => _, inout("rdx") count as u64 => _, out("rax") result, out("rcx") _, out("r11") _, out("r8") _, out("r9") _, out("r10") _); }
    result
}

/*
 * sys_port_in: Reads data from an x86 I/O port.
 */
#[inline]
pub fn sys_port_in(port: u16, size: u64) -> u64 {
    let result: u64;
    unsafe { asm!("syscall", inout("rdi") SYS_PORT_IN => _, inout("rsi") port as u64 => _, inout("rdx") size => _, out("rax") result, out("rcx") _, out("r11") _, out("r8") _, out("r9") _, out("r10") _); }
    result
}

/*
 * sys_check_irq: Queries the status of a specific interrupt vector.
 */
#[inline]
pub fn sys_check_irq(vector: u8) -> bool {
    let result: u64;
    unsafe { asm!("syscall", inout("rdi") SYS_CHECK_IRQ => _, inout("rsi") vector as u64 => _, out("rdx") _, out("rax") result, out("rcx") _, out("r11") _, out("r8") _, out("r9") _, out("r10") _); }
    result != 0
}

/*
 * sys_spawn_process: Creates a new execution context (process).
 */
#[inline]
pub fn sys_spawn_process(entry_point: u64, stack_ptr: u64) -> u64 {
    let result: u64;
    unsafe { asm!("syscall", inout("rdi") SYS_SPAWN_PROCESS => _, inout("rsi") entry_point => _, inout("rdx") stack_ptr => _, out("rax") result, out("rcx") _, out("r11") _, out("r8") _, out("r9") _, out("r10") _); }
    result
}

/*
 * sys_port_out: Writes an 8-bit value to an x86 I/O port.
 */
#[inline]
pub fn sys_port_out(port: u16, data: u8) {
    let arg1 = (1_u64 << 16) | (port as u64); 
    unsafe { asm!("syscall", inout("rdi") SYS_PORT_OUT => _, inout("rsi") arg1 => _, inout("rdx") data as u64 => _, out("rax") _, out("rcx") _, out("r11") _, out("r8") _, out("r9") _, out("r10") _); }
}

/*
 * sys_port_out32: Writes a 32-bit value to an x86 I/O port.
 */
#[inline]
pub fn sys_port_out32(port: u16, data: u32) {
    let arg1 = (4_u64 << 16) | (port as u64); 
    unsafe { asm!("syscall", inout("rdi") SYS_PORT_OUT => _, inout("rsi") arg1 => _, inout("rdx") data as u64 => _, out("rax") _, out("rcx") _, out("r11") _, out("r8") _, out("r9") _, out("r10") _); }
}

/*
 * sys_ipc_send: Sends a message to another process.
 */
#[inline]
pub fn sys_ipc_send(target_id: u64, payload: u64) {
    unsafe { asm!("syscall", inout("rdi") SYS_IPC_SEND => _, inout("rsi") target_id => _, inout("rdx") payload => _, out("rax") _, out("rcx") _, out("r11") _, out("r8") _, out("r9") _, out("r10") _); }
}

/*
 * sys_ipc_receive: Receives a pending message from another process.
 */
#[inline]
pub fn sys_ipc_receive(target_id: u64) -> u64 {
    let result: u64;
    unsafe { asm!("syscall", inout("rdi") SYS_IPC_RECEIVE => _, inout("rsi") target_id => _, out("rdx") _, out("rax") result, out("rcx") _, out("r11") _, out("r8") _, out("r9") _, out("r10") _); }
    result
}
