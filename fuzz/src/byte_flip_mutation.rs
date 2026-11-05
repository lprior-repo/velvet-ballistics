//! Byte-flipping mutation framework for fuzzing deserializers.
//!
//! This module provides utilities for systematically mutating serialized
//! data by flipping individual bytes and feeding the mutations to
//! decoders. The goal is to verify that every mutation either:
//! - Decodes successfully to a valid value (equivalent mutation), or
//! - Produces a typed error (never panics, never undefined behavior).

/// Flip a single byte at the given offset.
pub fn flip_byte_at(data: &[u8], offset: usize) -> Vec<u8> {
    let mut out = data.to_vec();
    if offset < out.len() {
        out[offset] ^= 0xFF; // Flip all bits in the byte
    }
    out
}

/// Flip each byte individually, yielding one mutant per byte position.
pub fn all_byte_flip_mutants(data: &[u8]) -> impl Iterator<Item = (usize, Vec<u8>)> {
    data.iter()
        .enumerate()
        .map(move |(i, _)| (i, flip_byte_at(data, i)))
}

/// Flip a random byte based on seed bytes.
pub fn seeded_byte_flip(data: &[u8], seed: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }
    let offset: usize =
        seed.iter()
            .fold(0u64, |acc, b| acc.wrapping_mul(256).wrapping_add(*b as u64)) as usize;
    let pos = offset % data.len();
    flip_byte_at(data, pos)
}

/// Flip multiple bytes based on a bitmask pattern.
/// Each bit in the pattern that is set to 1 causes a flip at that position.
pub fn flip_bytes_by_mask(data: &[u8], pattern: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    for (i, byte) in out.iter_mut().enumerate() {
        if i < pattern.len() && (pattern[i] & 0x01) != 0 {
            *byte ^= 0xFF;
        }
    }
    out
}

/// Generate a mutation pattern from seed bytes.
pub fn mutation_pattern(seed: &[u8], size: usize) -> Vec<u8> {
    let mut pattern = vec![0u8; size];
    for (i, byte) in pattern.iter_mut().enumerate() {
        if let Some(s) = seed.get(i % seed.len()) {
            *byte = *s;
        }
    }
    pattern
}

/// Flip bits within a single byte rather than the whole byte.
/// Useful for testing bitfield validation.
pub fn flip_bit_at_byte_offset(data: &[u8], byte_offset: usize, bit_index: u8) -> Vec<u8> {
    let mut out = data.to_vec();
    if byte_offset < out.len() && bit_index < 8 {
        out[byte_offset] ^= 1u8 << bit_index;
    }
    out
}

/// Generate all single-bit-flip mutants within a byte.
pub fn all_bit_flip_mutants(
    data: &[u8],
    byte_offset: usize,
) -> impl Iterator<Item = (u8, Vec<u8>)> {
    (0..8).map(move |bit| (bit, flip_bit_at_byte_offset(data, byte_offset, bit)))
}

/// Systematically corrupt data using the given seed to determine
/// which corruption strategy to use.
pub fn corrupted_version(data: &[u8], seed: &[u8]) -> Vec<u8> {
    if data.is_empty() {
        return Vec::new();
    }

    // Default to strategy 0 (single byte flip) if seed is empty
    let strategy: u8 = if seed.is_empty() { 0 } else { seed[0] };

    match strategy {
        0..=31 => {
            // Single byte flip
            seeded_byte_flip(data, seed)
        }
        32..=63 => {
            // Multi-byte flip using pattern
            flip_bytes_by_mask(data, &mutation_pattern(seed, data.len()))
        }
        64..=95 => {
            // Single bit flip
            let byte_pos = if seed.len() > 1 {
                seed[1] as usize % data.len()
            } else {
                0
            };
            let bit_pos = if seed.len() > 2 { seed[2] & 0x07 } else { 0 };
            flip_bit_at_byte_offset(data, byte_pos, bit_pos)
        }
        96..=127 => {
            // Flip first byte only
            flip_byte_at(data, 0)
        }
        128..=159 => {
            // Flip last byte only
            flip_byte_at(data, data.len() - 1)
        }
        160..=191 => {
            // Flip middle byte
            let mid = data.len() / 2;
            flip_byte_at(data, mid)
        }
        192..=223 => {
            // Flip a few random bytes
            let mut result = data.to_vec();
            let count = if seed.len() > 1 {
                (seed[1] & 0x07) as usize + 1
            } else {
                1
            };
            for j in 0..count {
                let pos = if seed.len() > j + 1 {
                    seed[j + 1] as usize
                } else {
                    j
                };
                result[pos % data.len()] ^= 0xFF;
            }
            result
        }
        224..=255 => {
            // Complement entire data
            data.iter().map(|b| !b).collect()
        }
    }
}
