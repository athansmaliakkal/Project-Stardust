/*
 * Stardust USTAR (TAR) Archive Parser
 * 
 * This module provides a minimal, read-only implementation of the USTAR 
 * (Universal Standard TAR) archive format. It is used by the system 
 * 'Marshal' process to extract drivers and assets from the embedded 
 * initramfs during the early userland bootstrap phase.
 *
 * Design:
 * - Linear Parsing: Iterates through the archive in 512-byte block increments.
 * - Deterministic ID Generation: Uses FNV-1a hashing to provide stable 
 *   identifiers for archive members based on their file paths.
 * - Memory Efficient: Returns slices into the original archive data to 
 *   avoid unnecessary allocations.
 */

use alloc::vec::Vec;
use alloc::string::String;
use core::str;

/*
 * Object: Represents a single file entry within the TAR archive.
 */
pub struct Object {
    pub id: u64,
    pub name: String,
    pub data: &'static [u8],
}

/*
 * generate_object_id: Fowler-Noll-Vo (FNV-1a) 64-bit Hash
 * 
 * Generates a unique, deterministic 64-bit identifier for a given string.
 * This is used to reference system objects without relying on string 
 * comparisons in performance-critical paths.
 */
fn generate_object_id(name: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in name.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/*
 * parse: Decodes a USTAR archive into a collection of Objects.
 * 
 * The TAR format consists of 512-byte headers followed by file data
 * padded to the next 512-byte boundary.
 */
pub fn parse(archive: &'static [u8]) -> Vec<Object> {
    let mut objects = Vec::new();
    let mut offset = 0;

    while offset + 512 <= archive.len() {
        let name_bytes = &archive[offset..offset + 100];
        
        // A null block indicates the end of the archive (EOF)
        if name_bytes[0] == 0 {
            break;
        }

        // Filename is a null-terminated UTF-8 string (max 100 chars)
        let mut name_len = 0;
        while name_len < 100 && name_bytes[name_len] != 0 {
            name_len += 1;
        }

        let name_str = str::from_utf8(&name_bytes[0..name_len]).unwrap_or("UNKNOWN");

        /*
         * Parse File Size
         * 
         * In USTAR, the file size is stored as an 11-byte octal ASCII 
         * string starting at offset 124.
         */
        let size_bytes = &archive[offset + 124..offset + 135];
        let mut size: usize = 0;
        for &b in size_bytes {
            if b >= b'0' && b <= b'7' {
                size = (size * 8) + (b - b'0') as usize;
            }
        }

        let data_start = offset + 512;
        let data_end = data_start + size;

        // Ensure the claimed size does not exceed the buffer bounds
        if data_end <= archive.len() {
            let data = &archive[data_start..data_end];
            objects.push(Object {
                id: generate_object_id(name_str),
                name: String::from(name_str),
                data,
            });
        }

        /*
         * Alignment: Files are aligned to 512-byte blocks. 
         * Calculate the offset to the next header block.
         */
        offset = data_start + ((size + 511) / 512) * 512;
    }

    objects
}
