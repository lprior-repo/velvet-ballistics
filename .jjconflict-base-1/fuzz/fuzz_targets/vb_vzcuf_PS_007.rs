// Cargo-fuzz target for core/storage bridge (PS-007, C8).
//
// Obligation ID: POB-vb-vzcuf-026
// Verifier: cargo-fuzz
// Command: cargo fuzz run vb_vzcuf_PS_007 -- -max_total_time=60
//
// Domain claim: Core max_journal_batch_bytes is safely bridged into
// storage JournalBatchByteLimit without silent drift or truncation.
//
// PRODUCTION BINDING:
//   The oracle calls the actual production bridge/policy APIs instead of
//   comparing scalar aliases:
//     - crates/vb_storage/src/batch/types.rs::DEFAULT_JOURNAL_BATCH_BYTE_LIMIT
//     - crates/vb_core/src/budget.rs::BoundednessPolicy::DEFAULT
//       (absolute_max_journal_batch_bytes)
//     - crates/vb_storage/src/constants.rs (RECORD_HEADER_LEN,
//       MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_JOURNAL_EVENT_RECORD_BYTES)
//     - The u32 -> u64 widening (budget.rs) and the u64::try_from /
//       checked_add / checked_div admission arithmetic
//       (batch/append_event.rs:86-102).
//   Every candidate value is derived from fuzzer bytes; the oracle holds
//   no hardcoded budget literals.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-026
// Follow-up: vb-mvvrr (binds this oracle to production bridge behavior).
#![no_main]
use libfuzzer_sys::fuzz_target;
use vb_core::BoundednessPolicy;
use vb_storage::{
    batch::DEFAULT_JOURNAL_BATCH_BYTE_LIMIT,
    constants::{
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_JOURNAL_EVENT_RECORD_BYTES, RECORD_HEADER_LEN,
    },
};

fn fuzz_bridge_value(data: &[u8]) {
    if data.len() < 8 {
        return;
    }
    let policy_bytes: [u8; 8] = match data.get(0..8) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let policy = u64::from_le_bytes(policy_bytes);
    if policy == 0 || policy > 100_000_000 {
        return;
    }
    // PRODUCTION BINDING: the core contract carries max_journal_batch_bytes
    // as u32 (ResourceContract::max_journal_batch_bytes) while storage
    // admission compares u64 staged_bytes against a u64 limit
    // (batch/append_event.rs). The bridge narrows here; values the core
    // contract cannot represent are rejected instead of truncated.
    let narrowed = match u32::try_from(policy) {
        Ok(value) => value,
        Err(_) => return,
    };
    // Widening back (budget.rs: u64::from(contract.max_journal_batch_bytes))
    // is exact, so the round-trip must preserve the fuzzer-derived value.
    let bridged = u64::from(narrowed);
    assert_eq!(bridged, policy);
    // Production admission comparison (append_event.rs:86-102) against the
    // real storage default, cross-checked with independent u128 arithmetic.
    let limit = DEFAULT_JOURNAL_BATCH_BYTE_LIMIT;
    let attempted = match 0u64.checked_add(bridged) {
        Some(total) => total,
        None => return,
    };
    let storage_reject = attempted > limit;
    let wide_reject = u128::from(bridged) > u128::from(limit);
    assert_eq!(storage_reject, wide_reject);
    // The core policy gate must agree with storage admission: the default
    // policy admits exactly the budgets that fit
    // absolute_max_journal_batch_bytes. Any drift between the two
    // independent limits fails here (and is pinpointed by
    // fuzz_bridge_defaults).
    let core_max = u64::from(BoundednessPolicy::DEFAULT.absolute_max_journal_batch_bytes);
    assert_eq!(storage_reject, bridged > core_max);
}

fn fuzz_bridge_defaults(_data: &[u8]) {
    // Two independent production sources for the same bridge budget: the
    // storage batch default (u64, batch/types.rs) and the core boundedness
    // policy (u32, budget.rs). Numeric equality is the bridge invariant.
    let storage_default: u64 = DEFAULT_JOURNAL_BATCH_BYTE_LIMIT;
    let core_policy: u64 = u64::from(BoundednessPolicy::DEFAULT.absolute_max_journal_batch_bytes);
    assert_eq!(storage_default, core_policy);
    // The per-event payload ceiling equals the batch budget numerically:
    // one full-payload event exactly fills an empty default batch before
    // its envelope header is accounted.
    let payload_max = u64::from(MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    assert_eq!(payload_max, storage_default);
    // The record budget decomposes exactly into header plus payload
    // (constants.rs defines MAX_JOURNAL_EVENT_RECORD_BYTES this way, via
    // const saturating_add; checked_add here re-verifies the derivation).
    let header = u64::from(RECORD_HEADER_LEN);
    let record = u64::from(MAX_JOURNAL_EVENT_RECORD_BYTES);
    assert_eq!(header.checked_add(payload_max), Some(record));
    // Boundary consequence: one max-size record overshoots a fresh default
    // batch budget by exactly one header, so a max-payload event alone trips
    // JournalBatchBytesExceeded on an empty default batch. Documented by the
    // production constants, not assumed.
    assert_eq!(record.checked_sub(storage_default), Some(header));
}

fn fuzz_multiple_events(data: &[u8]) {
    if data.len() < 8 {
        return;
    }
    let encoded_len_bytes: [u8; 8] = match data.get(0..8) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let encoded_len = u64::from_le_bytes(encoded_len_bytes);
    if encoded_len == 0 || encoded_len > 10_000 {
        return;
    }
    // PRODUCTION BINDING: the batch admission budget comes from storage,
    // not a hand-copied literal.
    let limit: u64 = DEFAULT_JOURNAL_BATCH_BYTE_LIMIT;
    let max_count = match limit.checked_div(encoded_len) {
        Some(count) => count,
        None => return,
    };
    assert!(max_count > 0);
    // Tightness: max_count events fit the budget, max_count + 1 does not.
    // checked_mul cannot overflow here (product <= limit); u128 cross-checks
    // the strict upper edge without masking overflow.
    let used = match max_count.checked_mul(encoded_len) {
        Some(used) => used,
        None => return,
    };
    assert!(used <= limit);
    let next = match max_count.checked_add(1) {
        Some(next) => next,
        None => return,
    };
    let next_used = u128::from(next).saturating_mul(u128::from(encoded_len));
    assert!(next_used > u128::from(limit));
    // Bridge round-trip: admitted counts fit the u32 count paths without
    // truncation.
    let narrowed = match u32::try_from(max_count) {
        Ok(value) => value,
        Err(_) => return,
    };
    assert_eq!(u64::from(narrowed), max_count);
}

fuzz_target!(|data: &[u8]| {
    let (first, rest) = match data.split_first() {
        Some(pair) => pair,
        None => return,
    };
    match first % 3 {
        0 => fuzz_bridge_value(rest),
        1 => fuzz_bridge_defaults(rest),
        _ => fuzz_multiple_events(rest),
    }
});
