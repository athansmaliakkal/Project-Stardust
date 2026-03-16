/*
 * Memory Management Subsystem.
 *
 * This module serves as the primary gateway for the kernel's memory management
 * architecture, including physical frame allocation, virtual paging, and
 * shared memory (IPC).
 */

pub mod physical;
pub mod frame;
pub mod paging;
pub mod mapper;
pub mod shared;
