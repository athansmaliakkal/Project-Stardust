/*
 * STARDUST KERNEL - TASK MANAGEMENT SUBSYSTEM
 * 
 * The tasking subsystem is the core of the Stardust executive, responsible for 
 * managing the lifecycle of both kernel and user execution contexts. 
 * This module exports the fundamental abstractions used throughout the kernel.
 * 
 * Submodules:
 * - process: High-level process management and privilege level transitions.
 * - thread: Thread control blocks (TCB) and state management.
 * - scheduler: The dispatching engine and cross-core load distribution.
 * - loader: Facilities for initializing processes from binary images.
 * - elf: Parser for the Executable and Linkable Format (ELF).
 * - context: Low-level architecture-specific register state and stack management.
 */

pub mod process;
pub mod thread;
pub mod scheduler;
pub mod loader;
pub mod elf;
pub mod context;
