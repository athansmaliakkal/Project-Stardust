/*
 * Capability Derivation Tree (CDT) Implementation
 *
 * This module implements the core security mechanism for Stardust, managing the
 * relationship between security tokens and their owners (Processes). The CDT
 * provides an O(1) validation mechanism for kernel capabilities.
 *
 * Architectural Design:
 * The CDT is a static array of CapabilityNodes, indexed by Token ID. This allows
 * for extremely fast validation of whether a process has the right to use a
 * particular token. Unlike a traditional ACL, the CDT focuses on the derivation
 * and ownership of capabilities.
 */

use spin::Mutex;
use crate::oracle;
use super::token::Token;

/// Maximum number of unique capability tokens that can be tracked by the kernel.
/// Increasing this increases the size of the kernel's BSS segment.
const MAX_CAPABILITIES: usize = 1024;

/// Represents a single entry in the Capability Derivation Tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityNode {
    /// The Process ID (PID) that currently owns this capability.
    pub owner_pid: u64,
    /// The actual token data, containing rights and ID.
    pub token: Token,
    /// Whether this node currently represents a valid, active capability.
    pub active: bool,
}

impl CapabilityNode {
    /// Sentinel value for uninitialized or freed capability nodes.
    const EMPTY: Self = CapabilityNode {
        owner_pid: 0,
        token: Token { id: 0, rights: 0 },
        active: false,
    };
}

/// The global container for all kernel capabilities.
pub struct CapabilityDerivationTree {
    pub nodes: [CapabilityNode; MAX_CAPABILITIES],
}

impl CapabilityDerivationTree {
    /// Creates a new, empty CDT. Used only during static initialization.
    pub const fn new() -> Self {
        CapabilityDerivationTree {
            nodes: [CapabilityNode::EMPTY; MAX_CAPABILITIES],
        }
    }

    /// Validates whether a given PID is the rightful owner of a specific Token.
    /// This is the primary security check performed by the kernel for sensitive operations.
    ///
    /// Returns true if the token is active, owned by the PID, and matches the requested rights.
    pub fn validate(&self, pid: u64, token: Token) -> bool {
        let index = token.id as usize;
        if index >= MAX_CAPABILITIES { return false; }

        let node = &self.nodes[index];
        node.active && node.owner_pid == pid && node.token.rights == token.rights
    }
}

/// Global Security Engine instance. Protected by a spinlock for thread-safe access.
/// Statically allocated in the .bss segment to avoid runtime overhead and heap dependency.
pub static CDT: Mutex<CapabilityDerivationTree> = Mutex::new(CapabilityDerivationTree::new());

/// Initializes the Security Engine and seals the Genesis Token.
/// The Genesis Token is the root of all system rights, assigned to the Kernel (PID 0).
pub fn init() {
    oracle::speak("[*] Sealing the Capability Derivation Tree (CDT)...\n");

    let mut tree = CDT.lock();
    
    // Seat the Genesis Token at index 0. This is the 'Root' capability.
    tree.nodes[0] = CapabilityNode {
        owner_pid: 0,
        token: Token::mint_genesis(),
        active: true,
    };

    oracle::speak("[+] CDT initialized. Genesis Token sealed to PID 0.\n");
    
    // Performance and Integrity Self-Test
    let test_token = Token::mint_genesis();
    if tree.validate(0, test_token) {
        oracle::speak("[+] O(1) Security Validation Test: SUCCESS.\n");
    } else {
        panic!("FATAL: Security Engine self-test failed!");
    }
}
