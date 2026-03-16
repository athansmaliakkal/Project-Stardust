/*
 * STARDUST KERNEL - SECURITY AND ACCESS CONTROL SUBSYSTEM
 * 
 * The security subsystem provides the core authorization and authentication 
 * primitives for the Stardust executive. It follows a capability-based 
 * security model, where access to kernel objects is mediated by tokens 
 * rather than user identity alone.
 * 
 * Submodules:
 * - token: Core capability representations and right mask definitions.
 * - cdt: The Capability Descriptor Table, which manages the lifecycle 
 *        of all active security tokens in the system.
 */

pub mod token;
pub mod cdt;
