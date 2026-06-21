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
//! Adversarial property tests for the `max_bytes` cap of `preview_keyspace`.
//!
//! These tests exercise the documented byte-cap guarantees:
//!   (a) total bytes <= max_bytes
//!   (b) byte cap is INCLUSIVE at the boundary (== fits, == + 1 does not)
//!   (c) when byte cap is hit, `truncated == true`
//!
//! Each test uses a deterministic seed (Numerical Recipes LCG) so failures
//! can be reproduced. If `preview_keyspace` regresses on the byte cap
//! (off-by-one boundary, wrong cap direction, ignored payload size),
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
            result.total_keyspace_records as usize, entry_count,
            "seed={seed}: total_keyspace_records must equal input length"
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
        let entries_fit: Vec<(Vec<u8>, Vec<u8>)> =
            vec![(make_valid_run_header_key(1), vec![0u8; cap as usize])];
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
        let entries_over: Vec<(Vec<u8>, Vec<u8>)> =
            vec![(make_valid_run_header_key(1), vec![0u8; (cap as usize) + 1])];
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
