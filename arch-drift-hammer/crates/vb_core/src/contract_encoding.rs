//! Canonical encoding of `ResourceContract` for digest computation.
//!
//! This is the SINGLE authoritative encoding used by `canonical_digest()`
//! and by ALL verification harnesses. This module is blake3-free; the caller
//! feeds the returned bytes into their hasher.
//!
//! Proof obligations served: PO-K01, K02, K03, K04, K07, K08, K10, K12, K13, K14
//! (all Kani digest-level proofs), plus proptest and Verus models.

#![forbid(unsafe_code)]

use crate::workflow::ResourceContract;

/// Produces the canonical, deterministic byte encoding of a `ResourceContract`
/// suitable for feeding into `blake3::Hasher::update`.
///
/// Each of the 17 fields is encoded as `[field_tag_bytes][value_bytes]` in a
/// fixed canonical order. Field tags are unique static ASCII strings that
/// provide domain separation. Multi-byte values use little-endian encoding.
///
/// # Determinism guarantee
///
/// This function has no internal state, no I/O, and no non-deterministic
/// operations. Calling it twice with the same contract always produces the
/// same byte sequence.
#[must_use]
pub fn encode_contract_bytes(contract: &ResourceContract) -> Vec<u8> {
    // 17 fields * (max tag len 24 + value len 8) + header ~= 512 bytes
    let mut buf = Vec::with_capacity(512);
    buf.extend_from_slice(b"resource_contract");

    buf.extend_from_slice(b"max_steps");
    buf.extend_from_slice(&contract.max_steps.to_le_bytes());

    buf.extend_from_slice(b"max_slots");
    buf.extend_from_slice(&contract.max_slots.to_le_bytes());

    buf.extend_from_slice(b"max_constants");
    buf.extend_from_slice(&contract.max_constants.to_le_bytes());

    buf.extend_from_slice(b"max_accessors");
    buf.extend_from_slice(&contract.max_accessors.to_le_bytes());

    buf.extend_from_slice(b"max_expressions");
    buf.extend_from_slice(&contract.max_expressions.to_le_bytes());

    buf.extend_from_slice(b"max_expr_stack");
    buf.extend_from_slice(&[contract.max_expr_stack]);

    buf.extend_from_slice(b"max_step_budget_per_tick");
    buf.extend_from_slice(&contract.max_step_budget_per_tick.to_le_bytes());

    buf.extend_from_slice(b"max_transitions_per_tick");
    buf.extend_from_slice(&contract.max_transitions_per_tick.to_le_bytes());

    buf.extend_from_slice(b"max_input_bytes");
    buf.extend_from_slice(&contract.max_input_bytes.to_le_bytes());

    buf.extend_from_slice(b"max_output_bytes");
    buf.extend_from_slice(&contract.max_output_bytes.to_le_bytes());

    buf.extend_from_slice(b"max_blob_bytes");
    buf.extend_from_slice(&contract.max_blob_bytes.to_le_bytes());

    buf.extend_from_slice(b"max_ipc_payload_bytes");
    buf.extend_from_slice(&contract.max_ipc_payload_bytes.to_le_bytes());

    buf.extend_from_slice(b"max_retry_attempts");
    buf.extend_from_slice(&contract.max_retry_attempts.to_le_bytes());

    buf.extend_from_slice(b"max_fanout");
    buf.extend_from_slice(&contract.max_fanout.to_le_bytes());

    buf.extend_from_slice(b"max_collect_items");
    buf.extend_from_slice(&contract.max_collect_items.to_le_bytes());

    buf.extend_from_slice(b"max_queue_depth");
    buf.extend_from_slice(&contract.max_queue_depth.to_le_bytes());

    buf.extend_from_slice(b"max_journal_batch_bytes");
    buf.extend_from_slice(&contract.max_journal_batch_bytes.to_le_bytes());

    buf.extend_from_slice(b"allows_secret_results");
    buf.push(u8::from(contract.allows_secret_results));

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // I1: Encoding determinism
    // ---------------------------------------------------------------------------

    #[test]
    fn encode_contract_bytes_is_deterministic() {
        let contract = ResourceContract::DEFAULT;
        let enc1 = encode_contract_bytes(&contract);
        let enc2 = encode_contract_bytes(&contract);
        assert_eq!(
            enc1, enc2,
            "encode_contract_bytes must produce identical output for identical input"
        );
    }

    #[test]
    fn encode_contract_bytes_is_deterministic_for_random_contract() {
        let mut contract = ResourceContract::DEFAULT;
        contract.max_steps = 500;
        contract.max_slots = 64;
        contract.allows_secret_results = true;
        let enc1 = encode_contract_bytes(&contract);
        let enc2 = encode_contract_bytes(&contract);
        assert_eq!(
            enc1, enc2,
            "encode_contract_bytes must be deterministic for any contract"
        );
    }

    // ---------------------------------------------------------------------------
    // I2: All 17 field tags present in canonical order
    // ---------------------------------------------------------------------------

    #[test]
    fn encode_contract_bytes_contains_all_17_field_tags_in_order() {
        let contract = ResourceContract::DEFAULT;
        let bytes = encode_contract_bytes(&contract);
        // The first tag is the header "resource_contract"
        assert!(
            bytes
                .windows(b"resource_contract".len())
                .any(|w| w == b"resource_contract"),
            "Must contain 'resource_contract' header"
        );

        let expected_tags: &[&[u8]] = &[
            b"max_steps",
            b"max_slots",
            b"max_constants",
            b"max_accessors",
            b"max_expressions",
            b"max_expr_stack",
            b"max_step_budget_per_tick",
            b"max_transitions_per_tick",
            b"max_input_bytes",
            b"max_output_bytes",
            b"max_blob_bytes",
            b"max_ipc_payload_bytes",
            b"max_retry_attempts",
            b"max_fanout",
            b"max_collect_items",
            b"max_queue_depth",
            b"max_journal_batch_bytes",
            b"allows_secret_results",
        ];

        // Each tag must appear in order
        let mut prev_pos = 0usize;
        for (i, tag) in expected_tags.iter().enumerate() {
            if let Some(pos) = bytes[prev_pos..].windows(tag.len()).position(|w| w == *tag) {
                let _actual_pos = prev_pos + pos;
                prev_pos = prev_pos + pos + tag.len();
                // Verify the tag is found
                assert!(
                    pos < bytes.len(),
                    "Tag {} ({}) must be present in encoding output",
                    i,
                    std::str::from_utf8(tag).unwrap_or("?")
                );
            } else {
                panic!(
                    "Tag {}/{} ({}) missing from encode_contract_bytes output",
                    i,
                    expected_tags.len(),
                    std::str::from_utf8(tag).unwrap_or("?")
                );
            }
        }
    }

    // ---------------------------------------------------------------------------
    // I3: Little-endian encoding for multi-byte values
    // ---------------------------------------------------------------------------

    #[test]
    fn encode_contract_bytes_uses_little_endian_for_max_steps_u16() {
        let mut contract = ResourceContract::DEFAULT;
        contract.max_steps = 0x0102u16;
        let bytes = encode_contract_bytes(&contract);
        // Find the value bytes after "max_steps" tag
        let tag = b"max_steps";
        let tag_pos = bytes
            .windows(tag.len())
            .position(|w| w == tag)
            .expect("max_steps tag must be present");
        let val_start = tag_pos + tag.len();
        // The u16 value should be two bytes: 0x02, 0x01 (little-endian)
        assert_eq!(bytes[val_start], 0x02, "u16 LSB should be first byte");
        assert_eq!(bytes[val_start + 1], 0x01, "u16 MSB should be second byte");
    }

    #[test]
    fn encode_contract_bytes_uses_little_endian_for_max_transitions_per_tick_u64() {
        let mut contract = ResourceContract::DEFAULT;
        contract.max_transitions_per_tick = 0x0102_0304_0506_0708u64;
        let bytes = encode_contract_bytes(&contract);
        let tag = b"max_transitions_per_tick";
        let tag_pos = bytes
            .windows(tag.len())
            .position(|w| w == tag)
            .expect("max_transitions_per_tick tag must be present");
        let val_start = tag_pos + tag.len();
        // u64 LE: lowest byte first
        assert_eq!(bytes[val_start], 0x08, "u64 LSB should be first byte");
        assert_eq!(bytes[val_start + 7], 0x01, "u64 MSB should be last byte");
    }

    #[test]
    fn encode_contract_bytes_uses_little_endian_for_max_blob_bytes_u64() {
        let mut contract = ResourceContract::DEFAULT;
        contract.max_blob_bytes = 0xDEADBEEFu64;
        let bytes = encode_contract_bytes(&contract);
        let tag = b"max_blob_bytes";
        let tag_pos = bytes
            .windows(tag.len())
            .position(|w| w == tag)
            .expect("max_blob_bytes tag must be present");
        let val_start = tag_pos + tag.len();
        // 0xDEADBEEF in LE: EF BE AD DE 00 00 00 00
        assert_eq!(bytes[val_start], 0xEF, "u64 LSB should be first byte");
        assert_eq!(bytes[val_start + 1], 0xBE);
        assert_eq!(bytes[val_start + 2], 0xAD);
        assert_eq!(bytes[val_start + 3], 0xDE);
    }

    // ---------------------------------------------------------------------------
    // I4: Unique domain tags
    // ---------------------------------------------------------------------------

    #[test]
    fn encode_contract_bytes_field_tags_are_unique() {
        let _tags: &[&[u8]] = &[
            b"max_steps",
            b"max_slots",
            b"max_constants",
            b"max_accessors",
            b"max_expressions",
            b"max_expr_stack",
            b"max_step_budget_per_tick",
            b"max_transitions_per_tick",
            b"max_input_bytes",
            b"max_output_bytes",
            b"max_blob_bytes",
            b"max_ipc_payload_bytes",
            b"max_retry_attempts",
            b"max_fanout",
            b"max_collect_items",
            b"max_queue_depth",
            b"max_journal_batch_bytes",
            b"allows_secret_results",
        ];

        // All 17 field tags (plus header "resource_contract") must be unique
        let all_tags: &[&[u8]] = &[
            b"resource_contract",
            b"max_steps",
            b"max_slots",
            b"max_constants",
            b"max_accessors",
            b"max_expressions",
            b"max_expr_stack",
            b"max_step_budget_per_tick",
            b"max_transitions_per_tick",
            b"max_input_bytes",
            b"max_output_bytes",
            b"max_blob_bytes",
            b"max_ipc_payload_bytes",
            b"max_retry_attempts",
            b"max_fanout",
            b"max_collect_items",
            b"max_queue_depth",
            b"max_journal_batch_bytes",
            b"allows_secret_results",
        ];

        for (i, a) in all_tags.iter().enumerate() {
            for (j, b) in all_tags.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        a, b,
                        "Field tags at positions {} and {} must be unique",
                        i, j
                    );
                }
            }
        }
    }

    // ---------------------------------------------------------------------------
    // I5: Encoding injectivity
    // ---------------------------------------------------------------------------

    #[test]
    fn encode_contract_bytes_is_injective_for_distinct_contracts() {
        let contract_a = ResourceContract::DEFAULT;
        let mut contract_b = ResourceContract::DEFAULT;
        contract_b.max_steps = contract_b.max_steps.saturating_add(1);
        let enc_a = encode_contract_bytes(&contract_a);
        let enc_b = encode_contract_bytes(&contract_b);
        assert_ne!(
            enc_a, enc_b,
            "Contracts differing in max_steps must produce different encodings"
        );
    }

    #[test]
    fn encode_contract_bytes_differs_when_allows_secret_results_toggled() {
        let mut contract_true = ResourceContract::DEFAULT;
        contract_true.allows_secret_results = true;
        let mut contract_false = ResourceContract::DEFAULT;
        contract_false.allows_secret_results = false;
        let enc_true = encode_contract_bytes(&contract_true);
        let enc_false = encode_contract_bytes(&contract_false);
        assert_ne!(
            enc_true, enc_false,
            "Contracts differing only in allows_secret_results must differ in encoding"
        );
    }

    #[test]
    fn encode_contract_bytes_differs_when_two_fields_change() {
        let contract_a = ResourceContract::DEFAULT;
        let mut contract_b = ResourceContract::DEFAULT;
        contract_b.max_steps = 1;
        contract_b.max_slots = 1;
        let enc_a = encode_contract_bytes(&contract_a);
        let enc_b = encode_contract_bytes(&contract_b);
        assert_ne!(
            enc_a, enc_b,
            "Contracts differing in two fields must produce different encodings"
        );
    }

    // ---------------------------------------------------------------------------
    // I6: No panic on extreme values
    // ---------------------------------------------------------------------------

    #[test]
    fn encode_contract_bytes_does_not_panic_for_extreme_contract_values() {
        // All zeros
        let mut all_zero = ResourceContract::DEFAULT;
        all_zero.max_steps = 0;
        all_zero.max_slots = 0;
        all_zero.max_constants = 0;
        all_zero.max_accessors = 0;
        all_zero.max_expressions = 0;
        all_zero.max_expr_stack = 0;
        all_zero.max_step_budget_per_tick = 0;
        all_zero.max_transitions_per_tick = 0;
        all_zero.max_input_bytes = 0;
        all_zero.max_output_bytes = 0;
        all_zero.max_blob_bytes = 0;
        all_zero.max_ipc_payload_bytes = 0;
        all_zero.max_retry_attempts = 0;
        all_zero.max_fanout = 0;
        all_zero.max_collect_items = 0;
        all_zero.max_queue_depth = 0;
        all_zero.max_journal_batch_bytes = 0;
        all_zero.allows_secret_results = false;
        let _bytes_all_zero = encode_contract_bytes(&all_zero);

        // All max values
        let mut all_max = ResourceContract::DEFAULT;
        all_max.max_steps = u16::MAX;
        all_max.max_slots = u16::MAX;
        all_max.max_constants = u16::MAX;
        all_max.max_accessors = u16::MAX;
        all_max.max_expressions = u16::MAX;
        all_max.max_expr_stack = u8::MAX;
        all_max.max_step_budget_per_tick = u64::MAX;
        all_max.max_transitions_per_tick = u64::MAX;
        all_max.max_input_bytes = u32::MAX;
        all_max.max_output_bytes = u32::MAX;
        all_max.max_blob_bytes = u64::MAX;
        all_max.max_ipc_payload_bytes = u32::MAX;
        all_max.max_retry_attempts = u16::MAX;
        all_max.max_fanout = u16::MAX;
        all_max.max_collect_items = u32::MAX;
        all_max.max_queue_depth = u32::MAX;
        all_max.max_journal_batch_bytes = u32::MAX;
        all_max.allows_secret_results = true;
        let _bytes_all_max = encode_contract_bytes(&all_max);

        // DEFAULT contract must always work
        let _bytes_default = encode_contract_bytes(&ResourceContract::DEFAULT);
    }

    #[test]
    fn encode_contract_bytes_output_is_not_empty() {
        let contract = ResourceContract::DEFAULT;
        let bytes = encode_contract_bytes(&contract);
        assert!(
            !bytes.is_empty(),
            "encoded contract bytes must not be empty"
        );
        // The encoding must be at least the header + all tags
        assert!(
            bytes.len() > 50,
            "encoded contract must contain meaningful data, got {} bytes",
            bytes.len()
        );
    }

    // ---------------------------------------------------------------------------
    // Additional: encoding stability and prefix uniqueness
    // ---------------------------------------------------------------------------

    #[test]
    fn encode_contract_bytes_no_tag_is_prefix_of_another_tag() {
        // Domain tags: "max_steps" must not be a prefix of "max_step_budget_per_tick" etc.
        let tags: &[&str] = &[
            "max_steps",
            "max_slots",
            "max_constants",
            "max_accessors",
            "max_expressions",
            "max_expr_stack",
            "max_step_budget_per_tick",
            "max_transitions_per_tick",
            "max_input_bytes",
            "max_output_bytes",
            "max_blob_bytes",
            "max_ipc_payload_bytes",
            "max_retry_attempts",
            "max_fanout",
            "max_collect_items",
            "max_queue_depth",
            "max_journal_batch_bytes",
            "allows_secret_results",
        ];

        for i in 0..tags.len() {
            for j in 0..tags.len() {
                if i != j {
                    assert!(
                        !tags[i].starts_with(tags[j]),
                        "Tag '{}' must not be a prefix of '{}' (collision risk)",
                        tags[j],
                        tags[i]
                    );
                }
            }
        }
    }
}
