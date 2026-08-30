//! Canonical encoding of `ResourceContract` for digest computation.
//!
//! This is the SINGLE authoritative encoding used by `canonical_digest()`
//! and by ALL verification harnesses. This module is blake3-free; the caller
//! feeds the returned bytes into their hasher.
//!
//! Proof obligations served: PO-K01, K02, K03, K04, K07, K08, K10, K12, K13, K14
//! (all Kani digest-level proofs), plus proptest and Verus models.

#![forbid(unsafe_code)]

use crate::workflow::{ResourceContract, ResultTaintPolicy};

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

    buf.extend_from_slice(b"result_taint_policy");
    buf.push(match contract.result_taint_policy {
        ResultTaintPolicy::Deny => 0,
        ResultTaintPolicy::Allow => 1,
    });

    buf
}

#[cfg(test)]
#[path = "contract_encoding/tests.rs"]
mod tests;
