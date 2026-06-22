#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing
)]
#![forbid(unsafe_code)]
//! Section 38 property test: `yaml_profile_enforcement`.
//!
//! This file asserts the strict-profile enforcement invariants of
//! `vb_yaml`:
//! - `reject_duplicate_keys` returns `Err(DuplicateKey)` iff a
//!   duplicate key appears in the input slice.
//! - `reject_yaml_1_1_ambiguous_scalars` returns
//!   `Err(AmbiguousScalar)` iff any scalar matches the YAML 1.1
//!   ambiguous set.
//! - `validate_yaml_profile` is deterministic over the same input.
//! - `reject_anchors_aliases_merges`, `reject_multiple_documents`,
//!   and `reject_forbidden_features` return a typed `YamlError`
//!   variant on the corresponding input shape.

use proptest::prelude::*;

use crate::events::collect_events;
use crate::{
    YamlError, reject_duplicate_keys, reject_yaml_1_1_ambiguous_scalars, validate_yaml_profile,
};

fn arb_safe_key() -> impl Strategy<Value = String> {
    // Filter out YAML 1.1 ambiguous scalars so that mapping keys
    // and scalar inputs always pass the strict profile.
    "[a-zA-Z_][a-zA-Z0-9_]{0,8}".prop_filter("not a YAML 1.1 ambiguous scalar", |s| {
        !is_yaml11_ambiguous(s)
    })
}

fn is_yaml11_ambiguous(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "y" | "n" | "yes" | "no" | "on" | "off"
    )
}

fn arb_unique_keys(n: usize) -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arb_safe_key(), n).prop_map(|mut v| {
        v.sort();
        v.dedup();
        v
    })
}

proptest! {
    /// `reject_duplicate_keys` returns `Ok(())` for any input with no
    /// duplicates (modulo the algorithm's iteration order).
    #[test]
    fn ype_no_duplicates_passes(keys in arb_unique_keys(8)) {
        // Cap length to keep the property tractable.
        let cap: usize = 6;
        let slice: Vec<&str> = keys[..keys.len().min(cap)]
            .iter()
            .map(String::as_str)
            .collect();
        let result = reject_duplicate_keys(&slice);
        prop_assert_eq!(result, Ok(()));
    }

    /// `reject_duplicate_keys` returns `Err(DuplicateKey)` for any
    /// input where the last element duplicates an earlier element.
    #[test]
    fn ype_duplicate_at_end_fails(
        prefix in arb_unique_keys(4),
        dup in arb_safe_key(),
    ) {
        let mut keys = prefix;
        keys.push(dup.clone());
        keys.push(dup);
        // Skip if the chosen dup collided with a unique prefix
        // element — we want a *guaranteed* duplicate.
        let last_two: Vec<&String> = keys.iter().rev().take(2).collect();
        if last_two[0] != last_two[1] {
            return Ok(());
        }
        let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
        let result = reject_duplicate_keys(&refs);
        match result {
            Err(YamlError::DuplicateKey { key }) => {
                prop_assert_eq!(key.as_ref(), last_two[0].as_str());
            }
            other => prop_assert!(
                matches!(other, Err(YamlError::DuplicateKey { .. })),
                "expected DuplicateKey, got {other:?}"
            ),
        }
    }

    /// `reject_yaml_1_1_ambiguous_scalars` returns `Ok(())` for
    /// non-ambiguous safe keys.
    #[test]
    fn ype_non_ambiguous_keys_pass(s in arb_safe_key()) {
        let result = reject_yaml_1_1_ambiguous_scalars(&[s.as_str()]);
        prop_assert_eq!(result, Ok(()));
    }

    /// `reject_yaml_1_1_ambiguous_scalars` returns
    /// `Err(AmbiguousScalar)` for any YAML 1.1 ambiguous scalar.
    /// The error stores the original (case-preserving) scalar.
    #[test]
    fn ype_yaml11_ambiguous_rejected(
        word in prop_oneof![
            Just("y"), Just("n"),
            Just("yes"), Just("no"),
            Just("on"), Just("off"),
            Just("Y"), Just("N"),
            Just("Yes"), Just("No"),
            Just("On"), Just("Off"),
            Just("YES"), Just("NO"),
            Just("ON"), Just("OFF"),
        ],
    ) {
        let result = reject_yaml_1_1_ambiguous_scalars(&[word]);
        match result {
            Err(YamlError::AmbiguousScalar { scalar }) => {
                prop_assert_eq!(scalar.as_ref(), word);
            }
            other => prop_assert!(
                matches!(other, Err(YamlError::AmbiguousScalar { .. })),
                "expected AmbiguousScalar({word}), got {other:?}"
            ),
        }
    }

    /// `validate_yaml_profile` is deterministic: two calls on the
    /// same input return identical results.
    #[test]
    fn ype_validate_is_deterministic(keys in arb_unique_keys(4)) {
        let mut yaml = String::new();
        for k in &keys {
            yaml.push_str(&format!("{k}: v\n"));
        }
        let r1 = validate_yaml_profile(&yaml);
        let r2 = validate_yaml_profile(&yaml);
        let r3 = validate_yaml_profile(&yaml);
        prop_assert_eq!(r1.clone(), r2.clone());
        prop_assert_eq!(r2, r3);
    }

    /// A clean mapping validates successfully.
    #[test]
    fn ype_clean_mapping_passes(keys in arb_unique_keys(4)) {
        let mut yaml = String::new();
        for k in &keys {
            yaml.push_str(&format!("{k}: v\n"));
        }
        let result = validate_yaml_profile(&yaml);
        prop_assert_eq!(result, Ok(()));
    }

    /// Empty source is rejected as `EmptySource` (no content).
    #[test]
    fn ype_empty_source_rejected(_unit in 0u8..1u8) {
        let result = validate_yaml_profile("");
        prop_assert!(matches!(result, Err(YamlError::EmptySource)));
    }

    /// Whitespace-only source is rejected (no document parsed).
    #[test]
    fn ype_whitespace_only_rejected(_unit in 0u8..1u8) {
        // Use spaces, not newlines, to avoid potential "found content" paths.
        let result = validate_yaml_profile("   ");
        prop_assert!(matches!(result, Err(YamlError::EmptySource)));
    }

    /// A source with a duplicate top-level key is rejected as
    /// `DuplicateKey`.
    #[test]
    fn ype_duplicate_top_level_key_rejected(
        key in arb_safe_key(),
    ) {
        let yaml = format!("{key}: 1\n{key}: 2\n");
        let result = validate_yaml_profile(&yaml);
        match result {
            Err(YamlError::DuplicateKey { key: k }) => {
                prop_assert_eq!(k.as_ref(), key);
            }
            other => prop_assert!(
                matches!(other, Err(YamlError::DuplicateKey { .. })),
                "expected DuplicateKey, got {other:?}"
            ),
        }
    }

    /// A source with an explicit YAML 1.1 ambiguous scalar
    /// (`flag: yes`) is rejected as `AmbiguousScalar`.
    #[test]
    fn ype_ambiguous_value_rejected(
        key in arb_safe_key(),
        word in prop_oneof![Just("yes"), Just("no"), Just("on"), Just("off")],
    ) {
        let yaml = format!("{key}: {word}\n");
        let result = validate_yaml_profile(&yaml);
        match result {
            Err(YamlError::AmbiguousScalar { .. }) => {}
            other => prop_assert!(
                matches!(other, Err(YamlError::AmbiguousScalar { .. })),
                "expected AmbiguousScalar, got {other:?}"
            ),
        }
    }

    /// A source with multiple documents (`--- ... ---`) is rejected.
    #[test]
    fn ype_multiple_documents_rejected(keys in arb_unique_keys(2)) {
        let mut yaml = String::from("---\n");
        for k in &keys {
            yaml.push_str(&format!("{k}: 1\n"));
        }
        yaml.push_str("---\n");
        for k in &keys {
            yaml.push_str(&format!("{k}: 2\n"));
        }
        let result = validate_yaml_profile(&yaml);
        prop_assert!(
            matches!(result, Err(YamlError::MultipleDocuments { .. })),
            "expected MultipleDocuments, got {result:?}"
        );
    }

    /// A source with an anchor is rejected as `AnchorAliasMerge`.
    #[test]
    fn ype_anchor_rejected(
        key in arb_safe_key(),
        value in arb_safe_key(),
    ) {
        let yaml = format!("{key}: &anchor {value}\n");
        let result = validate_yaml_profile(&yaml);
        prop_assert!(matches!(result, Err(YamlError::AnchorAliasMerge)));
    }

    /// A source with an alias is rejected as `AnchorAliasMerge`.
    #[test]
    fn ype_alias_rejected(
        key in arb_safe_key(),
        value in arb_safe_key(),
    ) {
        let yaml = format!("{key}: &anchor {value}\nother: *anchor\n");
        let result = validate_yaml_profile(&yaml);
        prop_assert!(matches!(result, Err(YamlError::AnchorAliasMerge)));
    }

    /// A source with a quoted (non-plain) `yes` is accepted.
    #[test]
    fn ype_quoted_ambiguous_value_accepted(
        key in arb_safe_key(),
        word in prop_oneof![Just("yes"), Just("no"), Just("on"), Just("off")],
    ) {
        let yaml = format!("{key}: '{word}'\n");
        let result = validate_yaml_profile(&yaml);
        prop_assert_eq!(result, Ok(()));
    }

    /// `validate_yaml_profile` with custom limits honors the
    /// `max_source_bytes` field. The limit is enforced only through
    /// the public `validate_yaml_profile` (default limits), so we
    /// construct a payload that exceeds the default 1 MiB
    /// `max_source_bytes`.
    #[test]
    fn ype_max_source_bytes_enforced(
        key in arb_safe_key(),
        extra_bytes in prop_oneof![Just(0usize), Just(1usize)],
    ) {
        // Build a payload of `default_max + extra_bytes + 1`. Use a
        // 16 KiB chunk to avoid pathological allocations, then
        // repeat.
        const DEFAULT_MAX: usize = 1_048_576;
        let chunk: String = "x".repeat(16_384);
        let target = DEFAULT_MAX + extra_bytes + 1;
        let repeats = target / chunk.len();
        let mut payload = String::with_capacity(repeats * chunk.len() + key.len() + 4);
        payload.push_str(&key);
        payload.push_str(": ");
        for _ in 0..repeats {
            payload.push_str(&chunk);
        }
        // Ensure we actually exceed the limit (last-mile pad).
        while payload.len() <= DEFAULT_MAX {
            payload.push('x');
        }
        let result = validate_yaml_profile(&payload);
        prop_assert!(
            matches!(result, Err(YamlError::SourceTooLarge { .. })),
            "expected SourceTooLarge for {} bytes, got {result:?}",
            payload.len()
        );
    }

    /// `collect_events` is deterministic for the same input.
    #[test]
    fn ype_collect_events_is_deterministic(keys in arb_unique_keys(3)) {
        let mut yaml = String::new();
        for k in &keys {
            yaml.push_str(&format!("{k}: v\n"));
        }
        let a = collect_events(&yaml);
        let b = collect_events(&yaml);
        match (a, b) {
            (Ok(ea), Ok(eb)) => {
                prop_assert_eq!(ea.len(), eb.len());
                for (xa, xb) in ea.iter().zip(eb.iter()) {
                    prop_assert_eq!(xa.anchor_id(), xb.anchor_id());
                    prop_assert_eq!(xa.tag(), xb.tag());
                }
            }
            (Err(ea), Err(eb)) => {
                prop_assert_eq!(ea, eb);
            }
            (oa, ob) => prop_assert!(
                matches!((&oa, &ob), (Err(_), Err(_))),
                "divergent outcomes for same input: a={oa:?}, b={ob:?}"
            ),
        }
    }
}
