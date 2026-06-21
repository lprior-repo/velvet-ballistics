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
//! Adversarial property tests for the `max_records` cap of `preview_keyspace`.
//!
//! These tests exercise the documented records-cap guarantees:
//!   (a) `max_records = 0` is rejected at the config layer
//!   (b) `entries.len() <= max_records`
//!   (c) when records cap is hit, `truncated == true`
//!
//! Each test uses a deterministic seed (Numerical Recipes LCG) so failures
//! can be reproduced. If `preview_keyspace` regresses on the records cap,
//! these tests fail loudly with a seed-tagged diagnostic message.

use crate::preview::preview_keyspace;
use crate::types::PreviewConfig;

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
