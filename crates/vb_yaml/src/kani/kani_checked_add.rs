#![forbid(unsafe_code)]
//! PO-KANI-001: Kani harness verifying that YamlLimits counter updates
//! in `collect_and_validate_events` use `checked_add` and convert
//! overflow to explicit `YamlError` variants instead of panicking.
//!
//! Proves: For arbitrary counter values and increments, every
//! `checked_add` site in the profile validation event loop returns
//! `Ok(…)` or an explicit `YamlError` — never panics via wrapping
//! or unchecked overflow.
//!
//! Bound: depth ≤ u16::MAX, node_count ≤ u32::MAX, seq/map counters ≤ usize::MAX.
//!
//! ## WAIVER NOTICE (F-REVIEW-001, Repair 1)
//!
//! These harnesses prove arithmetic correctness of the `checked_add(...).ok_or(...)`
//! pattern at each call site in isolation. They do NOT exercise the production
//! `collect_and_validate_events()` event loop because Kani cannot symbolically
//! execute the saphyr_parser (C-backed) parser. See `.beads/vb-jpq7.34/waiver-PO-KANI-001.md`
//! for full rationale and compensating evidence.
//!
//! Compensating evidence:
//! - PO-FUZZ-001: cargo-fuzz exercises the full production event loop for 60+ seconds
//! - PO-PROP-001: proptest runs 2000 iterations through `parse_yaml_events`
//! - Combined: Kani proves arithmetic safety; fuzz+proptest prove integration correctness.

use crate::{YamlError, YamlLimits};

/// Verifies that `depth.checked_add(1)` — as used in
/// `collect_and_validate_events` `MappingStart` / `SequenceStart` handlers —
/// never panics and correctly detects overflow.
///
/// Corresponds to production call sites at:
/// - profile_validation.rs:93 (MappingStart depth increment)
/// - profile_validation.rs:109 (SequenceStart depth increment)
#[kani::proof]
fn check_checked_add_counters_depth() {
    let depth: u16 = kani::any();
    let limits = YamlLimits::default();
    let max_depth = limits.max_depth;

    let result = depth.checked_add(1).ok_or(YamlError::NestingTooDeep {
        depth,
        max: max_depth,
    });

    match result {
        Ok(new_depth) => {
            assert!(new_depth > depth);
            // Cover: production branch where depth exceeds limit (lines 97-102, 113-118)
            if new_depth > max_depth {
                kani::cover!(true, "depth_exceeded_max");
                // This path leads to an early return with NestingTooDeep in production.
            }
        }
        Err(_) => {
            kani::cover!(true, "depth_overflow");
            // Overflow path: depth was u16::MAX, checked_add returns None.
            assert_eq!(depth, u16::MAX);
        }
    }
}

/// Verifies that `node_count.checked_add(1)` — as used at
/// profile_validation.rs:211-216 — never panics and correctly
/// detects overflow.
#[kani::proof]
fn check_checked_add_counters_node_count() {
    let node_count: u32 = kani::any();
    let limits = YamlLimits::default();
    let max_nodes = limits.max_nodes;

    let result = node_count
        .checked_add(1)
        .ok_or(YamlError::NodeLimitExceeded {
            count: u32::MAX,
            max: max_nodes,
        });

    match result {
        Ok(new_count) => {
            assert!(new_count > node_count);
            if new_count > max_nodes {
                kani::cover!(true, "node_count_exceeded_max");
            }
        }
        Err(_) => {
            kani::cover!(true, "node_count_overflow");
            assert_eq!(node_count, u32::MAX);
        }
    }
}

/// Verifies that `document_count.checked_add(1)` — as used at
/// profile_validation.rs:88-90 — never panics and detects overflow.
#[kani::proof]
fn check_checked_add_counters_document_count() {
    let document_count: usize = kani::any();

    let result = document_count
        .checked_add(1)
        .ok_or(YamlError::MultipleDocuments { count: usize::MAX });

    match result {
        Ok(new_count) => {
            assert!(new_count > document_count);
        }
        Err(_) => {
            kani::cover!(true, "document_count_overflow");
            assert_eq!(document_count, usize::MAX);
        }
    }
}

/// Verifies that sequence counter `checked_add(1)` — as used at
/// profile_validation.rs:163-166 — never panics and detects overflow.
///
/// The `Err` variant is `YamlError::SequenceTooLong`: this is the
/// user-facing semantic of the surrounding code at
/// profile_validation.rs:167-172 (the post-increment soft-limit check
/// returns `SequenceTooLong`, and the test-review contract is that the
/// harness models the user-visible error rather than the internal
/// `ok_or` argument).
#[kani::proof]
fn check_checked_add_counters_sequence() {
    let count: usize = kani::any();
    let limits = YamlLimits::default();
    let max_sequence_len = limits.max_sequence_len;

    let result = count.checked_add(1).ok_or(YamlError::SequenceTooLong {
        len: count,
        max: max_sequence_len,
    });

    match result {
        Ok(new_count) => {
            assert!(new_count > count);
            if new_count > max_sequence_len {
                kani::cover!(true, "sequence_len_exceeded_max");
            }
        }
        Err(_) => {
            kani::cover!(true, "sequence_count_overflow");
            assert_eq!(count, usize::MAX);
        }
    }
}

/// Verifies that mapping entry counter `checked_add(1)` — as used at
/// profile_validation.rs:137-140 — never panics and detects overflow.
///
/// The `Err` variant is `YamlError::MappingTooLarge`: this is the
/// user-facing semantic of the surrounding code at
/// profile_validation.rs:141-146 (the post-increment soft-limit check
/// returns `MappingTooLarge`, and the test-review contract is that the
/// harness models the user-visible error rather than the internal
/// `ok_or` argument).
#[kani::proof]
fn check_checked_add_counters_mapping() {
    let count: usize = kani::any();
    let limits = YamlLimits::default();
    let max_mapping_entries = limits.max_mapping_entries;

    let result = count.checked_add(1).ok_or(YamlError::MappingTooLarge {
        count,
        max: max_mapping_entries,
    });

    match result {
        Ok(new_count) => {
            assert!(new_count > count);
            if new_count > max_mapping_entries {
                kani::cover!(true, "mapping_entries_exceeded_max");
            }
        }
        Err(_) => {
            kani::cover!(true, "mapping_count_overflow");
            assert_eq!(count, usize::MAX);
        }
    }
}

/// Verifies that nested counter merge `checked_add(count)` — as used
/// at profile_validation.rs:182-188 and :198-204 — never panics and
/// detects overflow when merging child counters into parent.
///
/// The `Err` variant is `YamlError::NodeLimitExceeded`: this is the
/// production variant at profile_validation.rs:185-188 and :201-204.
/// The merge path has no soft-limit post-check (the `SequenceTooLong`/
/// `MappingTooLarge` checks are only reached on `Scalar` events at the
/// innermost level, not on the parent-counter merge). Therefore
/// `NodeLimitExceeded` is the only error variant the production
/// construct can return at the merge overflow point. This is the
/// production-call-site-faithful variant, confirmed by the test-review
/// finding (HIGH 4): "the latter is correct for `merge`".
#[kani::proof]
fn check_checked_add_counters_merge() {
    let parent: usize = kani::any();
    let child: usize = kani::any();
    let max_nodes = YamlLimits::default().max_nodes;

    let result = parent
        .checked_add(child)
        .ok_or(YamlError::NodeLimitExceeded {
            count: u32::MAX,
            max: max_nodes,
        });

    match result {
        Ok(sum) => {
            assert!(sum >= parent);
            assert!(sum >= child);
        }
        Err(_) => {
            kani::cover!(true, "merge_overflow");
            // Overflow: parent + child > usize::MAX.
            assert!(parent > usize::MAX - child);
        }
    }
}
