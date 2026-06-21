#![forbid(unsafe_code)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::let_underscore_must_use,
    clippy::needless_range_loop,
    clippy::redundant_clone,
    clippy::too_many_lines,
    reason = "test-only lint overrides; production code in preview.rs is unaffected"
)]
//! Adversarial property tests for key validation and the `truncated`
//! invariant of `preview_keyspace`.
//!
//! These tests cover:
//!   (d) invalid keys are silently skipped (kept entries all have valid
//!       `StorageKey::RunHeader` variants with non-zero `RunId`)
//!   (e) `truncated` is true iff a cap (records OR bytes) was hit
//!
//! Each test uses a deterministic seed (Numerical Recipes LCG) where
//! randomness applies, so failures can be reproduced. If `preview_keyspace`
//! regresses on key validation or the truncation invariant, these tests
//! fail loudly with a seed-tagged diagnostic message.

use crate::preview::{DecodedPreview, PreviewPayload, preview_keyspace};
use crate::types::{PreviewConfig, StorageKey};

/// Build a valid 9-byte RunHeader key for the given run id (1..=u64::MAX).
///
/// RunHeader keys have the shape `[0x10, run_id(8 bytes big-endian)]`.
fn make_valid_run_header_key(run_id: u64) -> Vec<u8> {
    let mut key = vec![0x10u8]; // PREFIX_RUN_HEADER
    key.extend_from_slice(&run_id.to_be_bytes());
    key
}

/// Numerical Recipes linear congruential generator — deterministic, seedable.
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005_u64)
            .wrapping_add(1_442_695_040_888_963_407_u64);
        self.state
    }
    fn next_in_range(&mut self, lo: u64, hi_excl: u64) -> u64 {
        if hi_excl <= lo {
            return lo;
        }
        lo + self.next_u64() % (hi_excl - lo)
    }
}

/// Property: mixed valid and invalid keys — invalid keys MUST be silently
/// skipped, valid keys MUST be kept. All kept entries must have valid
/// `StorageKey::RunHeader` variants with non-zero `RunId`.
#[test]
fn property_mixed_valid_invalid_keys_silently_skips_invalid() {
    for seed in 0u64..16 {
        let mut rng = Lcg::new(seed);
        let config = PreviewConfig::new(100, 4096).unwrap();
        let entries: Vec<(Vec<u8>, Vec<u8>)> = (0..8)
            .map(|idx| {
                let key: Vec<u8> = match idx {
                    0 => vec![],                               // EmptyKey
                    1 => vec![0xFFu8],                         // UnknownPrefix
                    2 => vec![0x10u8, 0, 0, 0, 0],             // Wrong length for RunHeader
                    3 => vec![0x10u8, 0, 0, 0, 0, 0, 0, 0, 0], // InvalidRunId (zero)
                    4 => make_valid_run_header_key(1),
                    5 => make_valid_run_header_key(2),
                    6 => make_valid_run_header_key(3),
                    _ => make_valid_run_header_key(4),
                };
                let payload = vec![rng.next_u64() as u8; 8];
                (key, payload)
            })
            .collect();
        let result = preview_keyspace(config, &entries).unwrap();
        assert_eq!(
            result.entries.len(),
            4,
            "seed={seed}: must keep exactly 4 valid entries, got {}",
            result.entries.len()
        );
        for (kept_key, _kept_value, _kept_payload) in &result.entries {
            match kept_key {
                StorageKey::RunHeader { run } => {
                    assert_ne!(
                        run.get(),
                        0,
                        "seed={seed}: RunHeader kept entry must have non-zero run id"
                    );
                }
                other => panic!("seed={seed}: unexpected key variant kept: {other:?}"),
            }
        }
    }
}

/// Property: a single valid entry with valid key and value MUST be
/// admitted (the simplest non-empty case).
#[test]
fn property_single_valid_entry_admitted() {
    let config = PreviewConfig::new(10, 1024).unwrap();
    let entries: Vec<(Vec<u8>, Vec<u8>)> =
        vec![(make_valid_run_header_key(42), vec![0xDE, 0xAD, 0xBE, 0xEF])];
    let result = preview_keyspace(config, &entries).unwrap();
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.total_keyspace_records, 1);
    assert!(!result.truncated);
}

/// Property: `truncated` MUST be true iff a cap (records OR bytes) was
/// hit; otherwise it MUST be false.
#[test]
fn property_truncated_iff_a_cap_was_hit() {
    // Case 1: records cap hit, byte cap not hit → truncated=true
    let config = PreviewConfig::new(2, u32::MAX).unwrap();
    let entries: Vec<(Vec<u8>, Vec<u8>)> = (1..=5u64)
        .map(|id| (make_valid_run_header_key(id), vec![0u8; 4]))
        .collect();
    let result = preview_keyspace(config, &entries).unwrap();
    assert!(result.truncated, "cap=2 of 5 entries must truncate");
    assert_eq!(result.entries.len(), 2);

    // Case 2: no cap hit (1 of 1 fits) → truncated=false
    let config = PreviewConfig::new(10, u32::MAX).unwrap();
    let entries: Vec<(Vec<u8>, Vec<u8>)> = vec![(make_valid_run_header_key(1), vec![0u8; 4])];
    let result = preview_keyspace(config, &entries).unwrap();
    assert!(!result.truncated, "1 of 1 must not truncate");
    assert_eq!(result.entries.len(), 1);

    // Case 3: byte cap hit → truncated=true
    let config = PreviewConfig::new(100, 8).unwrap();
    let entries: Vec<(Vec<u8>, Vec<u8>)> = (1..=5u64)
        .map(|id| (make_valid_run_header_key(id), vec![0u8; 16]))
        .collect();
    let result = preview_keyspace(config, &entries).unwrap();
    assert!(result.truncated, "byte cap hit must truncate");
    assert_eq!(result.entries.len(), 0);

    // Case 4: empty entries, no cap → truncated=false, total=0
    let config = PreviewConfig::new(10, 1024).unwrap();
    let result = preview_keyspace(config, &[]).unwrap();
    assert!(!result.truncated);
    assert_eq!(result.total_keyspace_records, 0);
    assert!(result.entries.is_empty());

    // Suppress dead-code warning for the imported alias.
    let _: fn() = || {
        let _ = PreviewPayload::Raw;
        let _ = std::mem::needs_drop::<DecodedPreview>();
    };
}

/// Property: a single INVALID key byte array (corrupt) MUST be silently
/// skipped per production behavior. We verify the resulting DecodedPreview
/// is valid and doesn't include the corrupt entry.
#[test]
fn property_single_invalid_key_skipped() {
    let config = PreviewConfig::new(10, 1024).unwrap();
    let entries: Vec<(Vec<u8>, Vec<u8>)> = vec![(
        vec![
            0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8,
        ],
        vec![0u8; 8],
    )];
    let result = preview_keyspace(config, &entries).unwrap();
    assert_eq!(
        result.entries.len(),
        0,
        "single invalid key must be silently skipped"
    );
    assert!(
        !result.truncated,
        "skipping invalid key is not a cap hit — truncated must be false"
    );
    assert_eq!(result.total_keyspace_records, 1);
}
