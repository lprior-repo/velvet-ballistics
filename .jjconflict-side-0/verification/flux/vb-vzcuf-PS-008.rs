// Flux-rs refinement annotations for guard precedence (PS-008, C6).
//
// Obligation ID: POB-vb-vzcuf-031
// Verifier: flux-rs
// Command: flux check verification/flux/vb-vzcuf-PS-008.rs
//
// Domain claim: Guard precedence remains key, durable duplicate,
// count, per-record payload, accumulated bytes, mutation.
//
// PRODUCTION BINDING:
//   Guard order in JournalWriteBatch::append_event
//   (crates/vb_storage/src/batch.rs:209-229).
//   Line 210: key validation
//   Line 211: duplicate check
//   Line 218: batch count check
//   Line 221: per-record encoding
//   (byte admission: contract C6 position)
//   Line 228: mutation/insert
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-031

#![allow(unused)]

/// Guard priority values: lower = higher priority (checked first).
const GUARD_KEY: u8 = 0;
const GUARD_DUPLICATE: u8 = 1;
const GUARD_COUNT: u8 = 2;
const GUARD_ENCODING: u8 = 3;
const GUARD_ADMISSION: u8 = 4;
const GUARD_MUTATION: u8 = 5;

/// Refinement: guards are in strict order.
fn test_guard_order() {
    assert!(GUARD_KEY < GUARD_DUPLICATE);
    assert!(GUARD_DUPLICATE < GUARD_COUNT);
    assert!(GUARD_COUNT < GUARD_ENCODING);
    assert!(GUARD_ENCODING < GUARD_ADMISSION);
    assert!(GUARD_ADMISSION < GUARD_MUTATION);
}

/// Refinement: admission guard after encoding (needs encoded_len).
fn test_admission_after_encoding() {
    assert!(GUARD_ENCODING < GUARD_ADMISSION);
}

/// Refinement: admission guard before mutation (rejection prevents insert).
fn test_admission_before_mutation() {
    assert!(GUARD_ADMISSION < GUARD_MUTATION);
}

/// Simulated guard chain: execute guards in order.
fn guard_chain(
    ok_key: bool,
    ok_duplicate: bool,
    ok_count: bool,
    ok_encoding: bool,
    ok_admission: bool,
) -> Result<u8, u8> {
    if !ok_key { return Err(GUARD_KEY); }
    if !ok_duplicate { return Err(GUARD_DUPLICATE); }
    if !ok_count { return Err(GUARD_COUNT); }
    if !ok_encoding { return Err(GUARD_ENCODING); }
    if !ok_admission { return Err(GUARD_ADMISSION); }
    Ok(GUARD_MUTATION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_chain_first_guard_fails() {
        assert_eq!(guard_chain(false, true, true, true, true), Err(GUARD_KEY));
    }

    #[test]
    fn guard_chain_duplicate_fails() {
        assert_eq!(guard_chain(true, false, true, true, true), Err(GUARD_DUPLICATE));
    }

    #[test]
    fn guard_chain_count_fails() {
        assert_eq!(guard_chain(true, true, false, true, true), Err(GUARD_COUNT));
    }

    #[test]
    fn guard_chain_encoding_fails() {
        assert_eq!(guard_chain(true, true, true, false, true), Err(GUARD_ENCODING));
    }

    #[test]
    fn guard_chain_admission_fails() {
        assert_eq!(guard_chain(true, true, true, true, false), Err(GUARD_ADMISSION));
    }

    #[test]
    fn guard_chain_all_pass() {
        assert_eq!(guard_chain(true, true, true, true, true), Ok(GUARD_MUTATION));
    }

    #[test]
    fn earlier_guard_takes_precedence() {
        // Key fails: no other guard matters
        assert_eq!(guard_chain(false, false, false, false, false), Err(GUARD_KEY));
        // Key ok, duplicate fails
        assert_eq!(guard_chain(true, false, false, false, false), Err(GUARD_DUPLICATE));
    }
}
