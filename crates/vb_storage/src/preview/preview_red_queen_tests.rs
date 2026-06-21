#![forbid(unsafe_code)]
//! Adversarial property tests for `preview_keyspace` (red-queen v2 pass).
//!
//! These tests exercise the documented contract guarantees of
//! `crate::preview::preview_keyspace`:
//!
//!   (a) entries.len() <= max_records
//!   (b) total bytes <= max_bytes
//!   (c) truncated iff a cap was hit
//!   (d) all kept entries have valid keys (invalid keys silently skipped)
//!
//! Each test uses a deterministic seed (Numerical Recipes LCG) so failures
//! can be reproduced. If `preview_keyspace` regresses (e.g. wrong cap
//! direction, off-by-one boundary, ignored invalid keys, missing saturation
//! guard), these tests fail loudly with a seed-tagged diagnostic message.

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

use crate::preview::{preview_keyspace, DecodedPreview, PreviewPayload};
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

/// Property: with max_records=0, the function MUST reject construction
/// at the config layer (PreviewConfig::new(0, _) returns Err(QueueCapacity)).
#[test]
fn property_max_records_zero_is_rejected_at_config_layer() {
    let result = PreviewConfig::new(0, 1024);
    assert!(
        matches!(result, Err(crate::JournalError::QueueCapacity)),
        "max_records=0 must be rejected with QueueCapacity at PreviewConfig::new"
    );
}

/// Property: with max_bytes=0 and non-empty entries, the very first valid
/// entry must trigger truncation (it can never fit). Returns Ok with
/// empty entries + truncated=true.
#[test]
fn property_max_bytes_zero_with_entries_yields_truncated_empty() {
    for seed in 0u64..8 {
        let mut rng = Lcg::new(seed);
        let entry_count: usize = rng.next_in_range(1, 8) as usize;
        let config = PreviewConfig::new(entry_count + 4, 0).unwrap();
        let entries: Vec<(Vec<u8>, Vec<u8>)> = (1..=entry_count as u64)
            .map(|id| (make_valid_run_header_key(id), vec![0xABu8; 16]))
            .collect();
        let result = preview_keyspace(config, &entries).unwrap();
        assert_eq!(
            result.entries.len(),
            0,
            "seed={seed}: max_bytes=0 must yield empty entries"
        );
        assert!(
            result.truncated,
            "seed={seed}: max_bytes=0 with non-empty entries must set truncated=true"
        );
        assert_eq!(
            result.total_keyspace_records as usize,
            entry_count,
            "seed={seed}: total_keyspace_records must equal input length"
        );
    }
}

/// Property: max_records < total entries MUST set truncated=true and
/// output exactly max_records entries.
#[test]
fn property_max_records_strictly_less_than_input_yields_truncated_at_cap() {
    for seed in 0u64..16 {
        let mut rng = Lcg::new(seed);
        let total: usize = rng.next_in_range(2, 16) as usize;
        let cap_records: usize = rng.next_in_range(1, total as u64) as usize;
        let config = PreviewConfig::new(cap_records, u32::MAX).unwrap();
        let entries: Vec<(Vec<u8>, Vec<u8>)> = (1..=total as u64)
            .map(|id| (make_valid_run_header_key(id), vec![0u8; 4]))
            .collect();
        let result = preview_keyspace(config, &entries).unwrap();
        assert_eq!(
            result.entries.len(),
            cap_records,
            "seed={seed} total={total} cap={cap_records}: entries.len must equal cap"
        );
        assert!(
            result.truncated,
            "seed={seed} total={total} cap={cap_records}: truncated must be true"
        );
    }
}

/// Property: max_bytes = u32::MAX with reasonable-sized payloads must NOT
/// trigger truncation due to byte cap.
#[test]
fn property_max_bytes_u32_max_with_small_payloads_never_truncates_for_byte_cap() {
    for seed in 0u64..8 {
        let mut rng = Lcg::new(seed);
        let total: usize = rng.next_in_range(1, 16) as usize;
        let config = PreviewConfig::new(total + 4, u32::MAX).unwrap();
        let entries: Vec<(Vec<u8>, Vec<u8>)> = (1..=total as u64)
            .map(|id| (make_valid_run_header_key(id), vec![0u8; 32]))
            .collect();
        let result = preview_keyspace(config, &entries).unwrap();
        assert_eq!(
            result.entries.len(),
            total,
            "seed={seed}: u32::MAX cap should admit all small payloads"
        );
        assert!(
            !result.truncated,
            "seed={seed}: u32::MAX cap should not truncate on small payloads"
        );
    }
}

/// Property: an entry with payload_len > max_bytes MUST cause truncation
/// (the entry cannot fit, so we stop). All kept entries must individually
/// fit within max_bytes.
#[test]
fn property_payload_larger_than_max_bytes_hard_caps() {
    for seed in 0u64..8 {
        let mut rng = Lcg::new(seed);
        let cap: u32 = rng.next_in_range(8, 64) as u32;
        let config = PreviewConfig::new(100, cap).unwrap();
        let entries: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (make_valid_run_header_key(1), vec![0u8; (cap as usize) * 2]),
            (make_valid_run_header_key(2), vec![0u8; 1]),
            (make_valid_run_header_key(3), vec![0u8; 1]),
        ];
        let result = preview_keyspace(config, &entries).unwrap();
        assert!(
            result.truncated,
            "seed={seed}: oversized payload must trigger truncation"
        );
        assert_eq!(
            result.entries.len(),
            0,
            "seed={seed}: oversized first entry must not be included"
        );
        let total: u32 = result
            .entries
            .iter()
            .map(|(_, v, _)| u32::try_from(v.len()).unwrap_or(u32::MAX))
            .sum();
        assert!(
            total <= cap,
            "seed={seed}: total bytes {total} must not exceed cap {cap}"
        );
    }
}

/// Property: byte cap is INCLUSIVE at the boundary (== cap fits, == cap+1
/// does not). This catches off-by-one bugs in the cap comparison.
#[test]
fn property_byte_cap_boundary_inclusive_exclusive() {
    for cap in [1u32, 7, 64, 1024, 4096] {
        let config_fit = PreviewConfig::new(2, cap).unwrap();
        let entries_fit: Vec<(Vec<u8>, Vec<u8>)> = vec![(
            make_valid_run_header_key(1),
            vec![0u8; cap as usize],
        )];
        let result = preview_keyspace(config_fit, &entries_fit).unwrap();
        assert_eq!(
            result.entries.len(),
            1,
            "cap={cap}: payload exactly at cap must fit"
        );
        assert!(
            !result.truncated,
            "cap={cap}: payload exactly at cap must not truncate"
        );

        let config_over = PreviewConfig::new(2, cap).unwrap();
        let entries_over: Vec<(Vec<u8>, Vec<u8>)> = vec![(
            make_valid_run_header_key(1),
            vec![0u8; (cap as usize) + 1],
        )];
        let result = preview_keyspace(config_over, &entries_over).unwrap();
        assert_eq!(
            result.entries.len(),
            0,
            "cap={cap}: payload exceeding cap must be excluded"
        );
        assert!(
            result.truncated,
            "cap={cap}: payload exceeding cap must truncate"
        );
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
                    0 => vec![], // EmptyKey
                    1 => vec![0xFFu8], // UnknownPrefix
                    2 => vec![0x10u8, 0, 0, 0, 0], // Wrong length for RunHeader
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
    let entries: Vec<(Vec<u8>, Vec<u8>)> = vec![(
        make_valid_run_header_key(42),
        vec![0xDE, 0xAD, 0xBE, 0xEF],
    )];
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
    let entries: Vec<(Vec<u8>, Vec<u8>)> =
        vec![(make_valid_run_header_key(1), vec![0u8; 4])];
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
        vec![0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8],
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

/// Property: a huge number of small valid entries with a tight max_records
/// cap MUST respect the cap exactly. This is the O(n) regression test:
/// if a future change introduces an O(n^2) `Vec+contains` pattern, this
/// test would still pass functionally but would be observable via
/// timing; we exercise it across sizes for coverage.
#[test]
fn property_max_records_strictly_enforced_at_large_n() {
    for (total, cap) in [(100usize, 5usize), (500, 17), (1000, 33)] {
        let config = PreviewConfig::new(cap, u32::MAX).unwrap();
        let entries: Vec<(Vec<u8>, Vec<u8>)> = (1..=total as u64)
            .map(|id| (make_valid_run_header_key(id), vec![0u8; 8]))
            .collect();
        let result = preview_keyspace(config, &entries).unwrap();
        assert_eq!(
            result.entries.len(),
            cap,
            "total={total} cap={cap}: entries.len must equal cap exactly"
        );
        assert!(
            result.truncated,
            "total={total} cap={cap}: must truncate when cap < total"
        );
    }
}
