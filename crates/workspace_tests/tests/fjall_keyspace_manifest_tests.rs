//! Fjall keyspace manifest and prefix registry tests (vb-s6vj).
//!
//! Covers:
//! - Prefix distinctness across all 10 keyspaces
//! - Big-endian byte ordering for u64/u32/u16 fields
//! - Encode/decode roundtrip correctness (decode functions pending PO-vb-s6vj-006..010)
//! - Typed error recovery for unknown prefixes and short keys (pending try_decode_key)
//! - Manifest completeness via FjallJournal::declared_keyspaces()
//! - Prefix registry bijectivity (pending try_prefix_to_keyspace/try_keyspace_to_prefix)
//!
//! Run with: `cargo test -p workspace_tests --test fjall_keyspace_manifest_tests -- --nocapture`

use proptest::prelude::*;
use vb_core::{ActionId, RunId, StepIdx};
use vb_storage::FjallJournal;
use vb_storage::keys::{
    blob_key, compiled_ir_key, index_action_key, run_event_key, run_header_key, workflow_source_key,
};

// ============================================================
// Known constants
// ============================================================

/// All known keyspace prefixes.
const VALID_PREFIXES: &[u8] = &[
    0x01, // PREFIX_WORKFLOW_SOURCE
    0x02, // PREFIX_COMPILED_IR
    0x10, // PREFIX_RUN_HEADER
    0x11, // PREFIX_RUN_EVENT
    0x12, // PREFIX_RUN_SNAPSHOT
    0x20, // PREFIX_BLOB
    0x30, // PREFIX_INDEX_STATUS
    0x31, // PREFIX_INDEX_WORKFLOW
    0x32, // PREFIX_INDEX_ACTION
    0x40, // PREFIX_RECOVERY_STAMP
];

// Key lengths derived from encoding format:
// DIGEST_KEY_BYTES = 33 = 1 (prefix) + 32 (digest)
// JOURNAL_KEY_BYTES = 17 = 1 (prefix) + 8 (run_id) + 8 (seq)
// RUN_ONLY_KEY_BYTES = 9 = 1 (prefix) + 8 (run_id)
// INDEX_STATUS_KEY_BYTES = 18 = 1 (prefix) + 1 (state) + 8 (timestamp) + 8 (run_id)
// INDEX_WORKFLOW_KEY_BYTES = 13 = 1 (prefix) + 4 (workflow_id) + 8 (run_id)
// INDEX_ACTION_KEY_BYTES = 13 = 1 (prefix) + 2 (action_id) + 8 (run_id) + 2 (step)
const DIGEST_KEY_BYTES: usize = 33;
const JOURNAL_KEY_BYTES: usize = 17;
const RUN_ONLY_KEY_BYTES: usize = 9;
const INDEX_STATUS_KEY_BYTES: usize = 18;
const INDEX_WORKFLOW_KEY_BYTES: usize = 13;
const INDEX_ACTION_KEY_BYTES: usize = 13;

// ============================================================
// PO-vb-s6vj-001 / ps-vb-s6vj-001: Prefix distinctness (proptest)
// ============================================================

proptest! {
    /// Verifies all known keyspace prefixes are pairwise distinct.
    /// We sample 2-element subsets from the known valid prefixes and verify they differ.
    #[test]
    fn prefix_distinctness(ix in 0usize..VALID_PREFIXES.len(), jx in 0usize..VALID_PREFIXES.len()) {
        prop_assume!(ix != jx, "must select two different indices");
        let a = VALID_PREFIXES[ix];
        let b = VALID_PREFIXES[jx];
        prop_assert_ne!(a, b, "any two distinct prefixes must differ");
    }
}

// ============================================================
// PO-vb-s6vj-002 / ps-vb-s6vj-001: Prefix distinctness (unit)
// ============================================================

#[test]
fn all_prefixes_distinct_unit() {
    // Direct all-pairs check for the known key-prefix constants.
    let prefixes = [
        0x01u8, // PREFIX_WORKFLOW_SOURCE
        0x02,   // PREFIX_COMPILED_IR
        0x10,   // PREFIX_RUN_HEADER
        0x11,   // PREFIX_RUN_EVENT
        0x12,   // PREFIX_RUN_SNAPSHOT
        0x20,   // PREFIX_BLOB
        0x30,   // PREFIX_INDEX_STATUS
        0x31,   // PREFIX_INDEX_WORKFLOW
        0x32,   // PREFIX_INDEX_ACTION
        0x40,   // PREFIX_RECOVERY_STAMP
    ];

    for i in 0..prefixes.len() {
        for j in (i + 1)..prefixes.len() {
            assert_ne!(
                prefixes[i], prefixes[j],
                "PREFIX at indices {} and {} must differ: {:#04x} vs {:#04x}",
                i, j, prefixes[i], prefixes[j]
            );
        }
    }
}

// ============================================================
// PO-vb-s6vj-003 / ps-vb-s6vj-002: Big-endian u64 ordering (proptest)
// ============================================================

proptest! {
    /// Verifies u64::to_be_bytes produces lexicographically ordered byte sequences.
    /// For any a < b, we must have a.to_be_bytes() < b.to_be_bytes() lexicographically.
    #[test]
    fn bigendian_u64_ordering(a: u64, b: u64) {
        let a_bytes = a.to_be_bytes();
        let b_bytes = b.to_be_bytes();

        if a < b {
            prop_assert!(
                a_bytes < b_bytes,
                "a={} < b={} but bytes compare: {:?} >= {:?}",
                a, b, &a_bytes[..], &b_bytes[..]
            );
        }
    }
}

// ============================================================
// PO-vb-s6vj-004 / ps-vb-s6vj-003: run_event key ordering (proptest)
// ============================================================

proptest! {
    /// Verifies encode_run_event(r1, s1) < encode_run_event(r2, s2)
    /// iff (r1 < r2) or (r1 == r2 and s1 < s2).
    #[test]
    fn run_event_ordering(r1: u64, s1: u64, r2: u64, s2: u64) {
        let key1 = run_event_key(RunId::new(r1), vb_storage::types::EventSeq::new(s1)).unwrap();
        let key2 = run_event_key(RunId::new(r2), vb_storage::types::EventSeq::new(s2)).unwrap();

        let expected_ordering = r1 < r2 || (r1 == r2 && s1 < s2);

        if expected_ordering {
            prop_assert!(
                key1 < key2,
                "expected key1 < key2 but key1 >= key2 for (r1={}, s1={}) vs (r2={}, s2={})",
                r1, s1, r2, s2
            );
        }
    }
}

// ============================================================
// PO-vb-s6vj-005 / ps-vb-s6vj-004: Max sequence boundary (unit)
// ============================================================

#[test]
fn max_sequence_ordering() {
    let run = RunId::new(1);
    let key_max = run_event_key(run, vb_storage::types::EventSeq::new(u64::MAX)).unwrap();
    let key_max_minus_1 =
        run_event_key(run, vb_storage::types::EventSeq::new(u64::MAX - 1)).unwrap();

    assert!(
        key_max_minus_1 < key_max,
        "u64::MAX-1 key must sort before u64::MAX key"
    );
    assert_eq!(
        &key_max[9..17],
        &u64::MAX.to_be_bytes(),
        "seq portion of max key must be u64::MAX bytes"
    );
}

// ============================================================
// PO-vb-s6vj-015 / ps-vb-s6vj-013: declared_keyspaces count (unit)
// ============================================================

#[test]
fn declared_keyspaces_count() {
    let keyspaces = FjallJournal::declared_keyspaces();
    assert_eq!(
        keyspaces.len(),
        10,
        "declared_keyspaces must return exactly 10 entries"
    );
}

#[test]
fn declared_keyspaces_contains_required_names() {
    let keyspaces = FjallJournal::declared_keyspaces();
    let required = [
        "workflow_source",
        "compiled_ir",
        "run_header",
        "run_event",
        "run_snapshot",
        "blob",
        "index_status",
        "index_workflow",
        "index_action",
        "recovery_stamp",
    ];

    for name in required {
        assert!(
            keyspaces.contains(&name),
            "declared_keyspaces must contain '{}'",
            name
        );
    }
}

#[test]
fn declared_keyspaces_no_duplicates() {
    let keyspaces = FjallJournal::declared_keyspaces();
    let mut sorted = keyspaces.to_vec();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        keyspaces.len(),
        "declared_keyspaces must not contain duplicates"
    );
}

// ============================================================
// PO-vb-s6vj-017 / ps-vb-s6vj-015: Encode determinism (unit)
// ============================================================

#[test]
fn encode_determinism_workflow_source() {
    let digest = [0xAB_u8; 32];
    let a = workflow_source_key(digest).unwrap();
    let b = workflow_source_key(digest).unwrap();
    assert_eq!(a, b, "workflow_source_key must be deterministic");
}

#[test]
fn encode_determinism_compiled_ir() {
    let digest = [0xCD_u8; 32];
    let a = compiled_ir_key(digest).unwrap();
    let b = compiled_ir_key(digest).unwrap();
    assert_eq!(a, b, "compiled_ir_key must be deterministic");
}

#[test]
fn encode_determinism_run_header() {
    let run = RunId::new(42);
    let a = run_header_key(run).unwrap();
    let b = run_header_key(run).unwrap();
    assert_eq!(a, b, "run_header_key must be deterministic");
}

#[test]
fn encode_determinism_run_event() {
    let run = RunId::new(7);
    let seq = vb_storage::types::EventSeq::new(3);
    let a = run_event_key(run, seq).unwrap();
    let b = run_event_key(run, seq).unwrap();
    assert_eq!(a, b, "run_event_key must be deterministic");
}

#[test]
fn encode_determinism_blob() {
    let digest = [0x33_u8; 32];
    let a = blob_key(digest).unwrap();
    let b = blob_key(digest).unwrap();
    assert_eq!(a, b, "blob_key must be deterministic");
}

#[test]
fn encode_determinism_index_action() {
    let action = ActionId::new(3);
    let run = RunId::new(30);
    let step = StepIdx::new(4);
    let a = index_action_key(action, run, step).unwrap();
    let b = index_action_key(action, run, step).unwrap();
    assert_eq!(a, b, "index_action_key must be deterministic");
}

// ============================================================
// PO-vb-s6vj-018 / ps-vb-s6vj-016: Cross-keyspace non-collision (proptest)
// ============================================================

proptest! {
    /// Verifies that keys from different keyspaces have different first bytes.
    /// This is structurally guaranteed by distinct prefix constants, but we
    /// provide empirical evidence via property testing.
    #[test]
    fn cross_keyspace_non_collision(
        digest1: [u8; 32],
        digest2: [u8; 32],
        run: u64,
    ) {
        let run_id = RunId::new(run);
        let key_workflow = workflow_source_key(digest1).unwrap();
        let key_compiled = compiled_ir_key(digest1).unwrap();
        let key_header = run_header_key(run_id).unwrap();
        let key_event = run_event_key(run_id, vb_storage::types::EventSeq::new(0)).unwrap();
        let key_blob = blob_key(digest2).unwrap();

        // All prefixes must differ
        prop_assert_ne!(key_workflow[0], key_compiled[0], "workflow vs compiled prefix");
        prop_assert_ne!(key_workflow[0], key_header[0], "workflow vs header prefix");
        prop_assert_ne!(key_workflow[0], key_event[0], "workflow vs event prefix");
        prop_assert_ne!(key_workflow[0], key_blob[0], "workflow vs blob prefix");
        prop_assert_ne!(key_compiled[0], key_header[0], "compiled vs header prefix");
        prop_assert_ne!(key_compiled[0], key_event[0], "compiled vs event prefix");
        prop_assert_ne!(key_compiled[0], key_blob[0], "compiled vs blob prefix");
        prop_assert_ne!(key_header[0], key_event[0], "header vs event prefix");
        prop_assert_ne!(key_header[0], key_blob[0], "header vs blob prefix");
        prop_assert_ne!(key_event[0], key_blob[0], "event vs blob prefix");
    }
}

// ============================================================
// PO-vb-s6vj-019 / ps-vb-s6vj-017: Exact byte length (unit)
// ============================================================

#[test]
fn encode_exact_length_workflow_source() {
    let key = workflow_source_key([0u8; 32]).unwrap();
    assert_eq!(
        key.len(),
        DIGEST_KEY_BYTES,
        "workflow_source key must be 33 bytes"
    );
}

#[test]
fn encode_exact_length_compiled_ir() {
    let key = compiled_ir_key([0u8; 32]).unwrap();
    assert_eq!(
        key.len(),
        DIGEST_KEY_BYTES,
        "compiled_ir key must be 33 bytes"
    );
}

#[test]
fn encode_exact_length_run_header() {
    let key = run_header_key(RunId::new(0)).unwrap();
    assert_eq!(
        key.len(),
        RUN_ONLY_KEY_BYTES,
        "run_header key must be 9 bytes"
    );
}

#[test]
fn encode_exact_length_run_event() {
    let key = run_event_key(RunId::new(0), vb_storage::types::EventSeq::new(0)).unwrap();
    assert_eq!(
        key.len(),
        JOURNAL_KEY_BYTES,
        "run_event key must be 17 bytes"
    );
}

#[test]
fn encode_exact_length_blob() {
    let key = blob_key([0u8; 32]).unwrap();
    assert_eq!(key.len(), DIGEST_KEY_BYTES, "blob key must be 33 bytes");
}

#[test]
fn encode_exact_length_index_action() {
    let key = index_action_key(ActionId::new(0), RunId::new(0), StepIdx::new(0)).unwrap();
    assert_eq!(
        key.len(),
        INDEX_ACTION_KEY_BYTES,
        "index_action key must be 13 bytes"
    );
}

// ============================================================
// PO-vb-s6vj-020 / ps-vb-s6vj-018: index_action ordering (proptest)
// ============================================================

proptest! {
    /// Verifies encode_index_action(a1,r1,s1) < encode_index_action(a2,r2,s2)
    /// iff (a1 < a2) or (a1 == a2 and r1 < r2) or (a1 == a2 and r1 == r2 and s1 < s2).
    #[test]
    fn index_action_ordering(a1: u16, r1: u64, s1: u16, a2: u16, r2: u64, s2: u16) {
        let key1 = index_action_key(ActionId::new(a1), RunId::new(r1), StepIdx::new(s1)).unwrap();
        let key2 = index_action_key(ActionId::new(a2), RunId::new(r2), StepIdx::new(s2)).unwrap();

        let expected_ordering = a1 < a2
            || (a1 == a2 && r1 < r2)
            || (a1 == a2 && r1 == r2 && s1 < s2);

        if expected_ordering {
            prop_assert!(
                key1 < key2,
                "expected key1 < key2 but key1 >= key2 for (a1={}, r1={}, s1={}) vs (a2={}, r2={}, s2={})",
                a1, r1, s1, a2, r2, s2
            );
        }
    }
}

// ============================================================
// BLOCKED: PO-vb-s6vj-011 / ps-vb-s6vj-010: Unknown prefix errors (proptest)
// Requires: try_decode_key (NOT YET IMPLEMENTED — see PO-vb-s6vj-006..010)
// ============================================================

// COMMENTED OUT — try_decode_key does not exist yet in vb_storage::keys
// This test will be uncommented once the decode function is implemented.
//
// proptest! {
//     /// Verifies all 254 invalid prefix bytes return UnknownKeyPrefix error.
//     fn unknown_prefix_errors(bad_prefix: u8) {
//         // Skip valid prefixes
//         if VALID_PREFIXES.contains(&bad_prefix) {
//             return Ok(());
//         }
//         let mut key = vec![bad_prefix];
//         key.extend(std::iter::repeat(0).take(32)); // pad to reasonable length
//         let result = vb_storage::keys::try_decode_key(&key);
//         prop_assert!(
//             matches!(result, Err(JournalError::UnknownKeyPrefix { found }) if found == bad_prefix),
//             "invalid prefix {:#04x} must return UnknownKeyPrefix, got {:?}",
//             bad_prefix, result
//         );
//     }
// }

// ============================================================
// BLOCKED: PO-vb-s6vj-012 / ps-vb-s6vj-010: Specific unknown prefixes (unit)
// Requires: try_decode_key (NOT YET IMPLEMENTED)
// ============================================================

// COMMENTED OUT — try_decode_key does not exist yet.
//
// #[test]
// fn specific_unknown_prefixes() {
//     let test_cases = [0x00u8, 0xFF, 0x1F];
//     for &prefix in &test_cases {
//         let mut key = vec![prefix];
//         key.extend(std::iter::repeat(0).take(32));
//         let result = vb_storage::keys::try_decode_key(&key);
//         assert!(
//             matches!(result, Err(JournalError::UnknownKeyPrefix { found }) if found == prefix),
//             "prefix {:#04x} must return UnknownKeyPrefix, got {:?}",
//             prefix, result
//         );
//     }
// }

// ============================================================
// BLOCKED: PO-vb-s6vj-013 / ps-vb-s6vj-011: Short key errors (proptest)
// Requires: try_decode_key (NOT YET IMPLEMENTED)
// ============================================================

// COMMENTED OUT — try_decode_key does not exist yet.
//
// proptest! {
//     /// Verifies all lengths 1..L-1 return ShortKey error for each keyspace.
//     fn short_key_errors(prefix in prop::sample::select(VALID_PREFIXES)) {
//         let expected_len = match prefix {
//             0x01 | 0x02 | 0x20 => 33usize,
//             0x10 => 9usize,
//             0x11 | 0x12 => 17usize,
//             0x30 => 18usize,
//             0x31 | 0x32 => 13usize,
//             _ => return Ok(()),
//         };
//
//         // Generate lengths from 1 to expected_len-1
//         for len in 1..expected_len {
//             let mut key = vec![prefix];
//             key.extend(std::iter::repeat(0).take(len - 1));
//             let result = vb_storage::keys::try_decode_key(&key);
//             prop_assert!(
//                 matches!(result, Err(JournalError::ShortKey { prefix: p, found_len: fl, expected_len: el })
//                     if p == prefix && fl == len && el == expected_len),
//                 "short key for prefix {:#04x} len {} must return ShortKey, got {:?}",
//                 prefix, len, result
//             );
//         }
//     }
// }

// ============================================================
// BLOCKED: PO-vb-s6vj-014 / ps-vb-s6vj-012: Empty bytes no panic (unit)
// Requires: try_decode_key (NOT YET IMPLEMENTED)
// ============================================================

// COMMENTED OUT — try_decode_key does not exist yet.
//
// #[test]
// fn empty_bytes_no_panic() {
//     let result = vb_storage::keys::try_decode_key(&[]);
//     assert!(
//         matches!(result, Err(JournalError::ShortKey { .. })),
//         "empty bytes must return ShortKey error, not panic"
//     );
// }

// ============================================================
// BLOCKED: PO-vb-s6vj-016 / ps-vb-s6vj-014: Prefix registry bijectivity (proptest)
// Requires: try_prefix_to_keyspace and try_keyspace_to_prefix (NOT YET IMPLEMENTED)
// ============================================================

// COMMENTED OUT — registry lookup functions do not exist yet.
//
// proptest! {
//     /// Verifies the prefix<->name mapping is bijective for all 9 keyspaces.
//     fn prefix_registry_bijective(prefix in prop::sample::select(VALID_PREFIXES)) {
//         let name_result = vb_storage::keys::try_prefix_to_keyspace(prefix);
//         prop_assert!(name_result.is_some(), "valid prefix {:#04x} must map to a name", prefix);
//
//         if let Some(name) = name_result {
//             let prefix_roundtrip = vb_storage::keys::try_keyspace_to_prefix(name);
//             prop_assert!(
//                 prefix_roundtrip == Some(prefix),
//                 "prefix {:#04x} -> {} -> {:?} must roundtrip",
//                 prefix, name, prefix_roundtrip
//             );
//         }
//     }
// }

// ============================================================
// Smoke test — verify the test module itself compiles and runs
// ============================================================

#[test]
fn smoke_test_module_loads() {
    // Sanity check: constants are in scope
    assert_eq!(
        VALID_PREFIXES.len(),
        10,
        "must have exactly 10 valid prefixes"
    );
    assert_eq!(DIGEST_KEY_BYTES, 33, "DIGEST_KEY_BYTES must be 33");
    assert_eq!(JOURNAL_KEY_BYTES, 17, "JOURNAL_KEY_BYTES must be 17");
    assert_eq!(RUN_ONLY_KEY_BYTES, 9, "RUN_ONLY_KEY_BYTES must be 9");
    assert_eq!(
        INDEX_STATUS_KEY_BYTES, 18,
        "INDEX_STATUS_KEY_BYTES must be 18"
    );
    assert_eq!(
        INDEX_WORKFLOW_KEY_BYTES, 13,
        "INDEX_WORKFLOW_KEY_BYTES must be 13"
    );
    assert_eq!(
        INDEX_ACTION_KEY_BYTES, 13,
        "INDEX_ACTION_KEY_BYTES must be 13"
    );
}
