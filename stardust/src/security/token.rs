/*
 * STARDUST KERNEL - CAPABILITY-BASED SECURITY TOKENS
 * 
 * This module defines the core security primitive of the Stardust executive: 
 * the Security Token. Stardust utilizes a capability-based security model 
 * rather than traditional Access Control Lists (ACLs).
 * 
 * In this model, a Token is an unforgeable reference to a set of rights 
 * that a process possesses. Access to kernel objects and resources is 
 * mediated by validating whether the provided Token contains the necessary 
 * right mask for the requested operation.
 */

use crate::oracle;

/// Standard bitmask rights for the Stardust capability model.
/// 
/// These rights are granular and can be combined to define specific 
/// access policies for resources.
pub const RIGHT_NONE: u64    = 0;
pub const RIGHT_READ: u64    = 1 << 0;  // Ability to read resource state
pub const RIGHT_WRITE: u64   = 1 << 1;  // Ability to modify resource state
pub const RIGHT_EXECUTE: u64 = 1 << 2;  // Ability to execute the resource
pub const RIGHT_GRANT: u64   = 1 << 3;  // Ability to delegate this token to another process
pub const RIGHT_MAP: u64     = 1 << 4;  // Ability to map the resource into a virtual address space
pub const RIGHT_GOD: u64     = u64::MAX; // Universal system-level rights

/// Represents a security capability within the kernel.
/// 
/// A Token consists of a unique identifier and a bitwise right mask. 
/// In a mature implementation, the Token ID would be used to index 
/// into a kernel-managed Capability Descriptor Table (CDT).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    /// Unique identifier for the token.
    pub id: u64,
    
    /// Bitmask representing the set of rights granted by this token.
    pub rights: u64,
}

impl Token {
    /// Creates the initial system-level token with absolute privileges.
    /// 
    /// This is used exclusively by the kernel during the boot sequence 
    /// to initialize core services.
    pub const fn mint_genesis() -> Self {
        Token {
            id: 0,
            rights: RIGHT_GOD,
        }
    }

    /// Diagnostic helper to output token information to the system console.
    pub fn display(&self) {
        oracle::speak("[+] Token ID: 0x");
        oracle::speak_hex(self.id);
        oracle::speak(" | Rights Map: 0x");
        oracle::speak_hex(self.rights);
        oracle::speak("\n");
    }

    /// Performs a cryptographic or logical validation of the token's rights.
    /// 
    /// # Parameters
    /// * `required`: The bitmask of rights required for the operation.
    /// 
    /// # Returns
    /// `true` if the token possesses all required rights, `false` otherwise.
    pub fn has_right(&self, required: u64) -> bool {
        (self.rights & required) == required
    }
}
