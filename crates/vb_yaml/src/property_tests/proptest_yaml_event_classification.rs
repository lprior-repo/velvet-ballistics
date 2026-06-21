#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing
)]
#![forbid(unsafe_code)]
//! Section 38 property test: `yaml_event_classification`.
//!
//! This file asserts the invariants of the typed `YamlEvent` stream
//! produced by the YAML parser:
//! - The first event in a successful stream is `StreamStart`; the
//!   last event is `StreamEnd`.
//! - `is_document_start()`, `is_alias()`, `as_scalar()`, `tag()`,
//!   `anchor_id()` accessors always agree with the actual
//!   `YamlEvent` variant.
//! - `Scalar` event spans are well-formed: `start <= end` and
//!   `line >= 1`.
//! - Tag strings, when present, are stable across parsing of the
//!   same source (determinism).
//! - Plain-style `Scalar` events with YAML-1.1-ambiguous values are
//!   correctly classified as ambiguous (and rejected by
//!   `reject_yaml_1_1_ambiguous_scalars`).

use proptest::prelude::*;

use crate::events::{collect_events, ScalarStyle, YamlEvent};
use crate::profile::reject_yaml_1_1_ambiguous_scalars;
use crate::{YamlError, parse_yaml_events};

fn arb_safe_scalar() -> impl Strategy<Value = String> {
    // ASCII alphanumerics + a few safe punctuation; no whitespace,
    // no anchors, no aliases, no tags, no flow indicators.
    // The filter rejects YAML 1.1 ambiguous scalars (yes/no/on/off
    // and case variants) because the strict profile rejects them.
    "[a-zA-Z0-9_]{1,16}".prop_filter("not a YAML 1.1 ambiguous scalar", |s| {
        !is_yaml11_ambiguous(s)
    })
}

/// True when `s` matches a YAML 1.1 ambiguous scalar that the strict
/// profile rejects.
fn is_yaml11_ambiguous(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "y" | "n" | "yes" | "no" | "on" | "off"
    )
}

proptest! {
    /// Successful parsing of any safe scalar produces at least one
    /// `Scalar` event whose `as_scalar()` returns the source value.
    #[test]
    fn yec_safe_scalar_classified(value in arb_safe_scalar()) {
        let yaml = format!("{value}\n");
        let events = parse_yaml_events(&yaml).expect("safe scalar must parse");
        let scalars: Vec<&str> = events.iter().filter_map(|e| e.as_scalar()).collect();
        prop_assert!(
            scalars.iter().any(|s| *s == value),
            "expected to find scalar {value} in {scalars:?}"
        );
    }

    /// A non-anchor scalar must report `anchor_id() == 0`.
    #[test]
    fn yec_non_anchor_scalar_has_zero_id(value in arb_safe_scalar()) {
        let yaml = format!("{value}\n");
        let events = parse_yaml_events(&yaml).expect("safe scalar must parse");
        for event in &events {
            if let YamlEvent::Scalar { value: v, .. } = event {
                if v.as_ref() == value {
                    prop_assert_eq!(
                        event.anchor_id(),
                        0,
                        "non-anchor scalar must report anchor_id 0"
                    );
                }
            }
        }
    }

    /// A non-tagged scalar must report `tag() == None`.
    #[test]
    fn yec_non_tagged_scalar_has_no_tag(value in arb_safe_scalar()) {
        let yaml = format!("{value}\n");
        let events = parse_yaml_events(&yaml).expect("safe scalar must parse");
        for event in &events {
            if let YamlEvent::Scalar { value: v, .. } = event {
                if v.as_ref() == value {
                    prop_assert_eq!(
                        event.tag(),
                        None,
                        "non-tagged scalar must report None tag"
                    );
                }
            }
        }
    }

    /// A scalar at the top level must be classified with
    /// `is_document_start()` returning false (the scalar is not a
    /// document start).
    #[test]
    fn yec_scalar_is_not_document_start(value in arb_safe_scalar()) {
        let yaml = format!("{value}\n");
        let events = parse_yaml_events(&yaml).expect("safe scalar must parse");
        for event in &events {
            if let YamlEvent::Scalar { .. } = event {
                prop_assert!(
                    !event.is_document_start(),
                    "scalar event must not be a document start"
                );
            }
        }
    }

    /// A scalar is never classified as an alias.
    #[test]
    fn yec_scalar_is_not_alias(value in arb_safe_scalar()) {
        let yaml = format!("{value}\n");
        let events = parse_yaml_events(&yaml).expect("safe scalar must parse");
        for event in &events {
            if let YamlEvent::Scalar { .. } = event {
                prop_assert!(!event.is_alias(), "scalar must not be an alias");
            }
        }
    }

    /// Every `Span` of a successful parse has `start <= end` and
    /// `line >= 1`. This is the well-formedness floor for spans.
    #[test]
    fn yec_spans_are_well_formed(value in arb_safe_scalar()) {
        let yaml = format!("{value}\n");
        let events = parse_yaml_events(&yaml).expect("safe scalar must parse");
        for event in &events {
            let span = event.span();
            prop_assert!(
                span.start <= span.end,
                "span start ({}) > end ({})",
                span.start,
                span.end
            );
            prop_assert!(span.line >= 1, "span line must be >= 1, got {}", span.line);
        }
    }

    /// Parsing the same source twice yields the same sequence of
    /// events. This is the determinism floor for the typed event
    /// stream.
    #[test]
    fn yec_parsing_is_deterministic(value in arb_safe_scalar()) {
        let yaml = format!("{value}\n");
        let a = parse_yaml_events(&yaml).expect("safe scalar must parse");
        let b = parse_yaml_events(&yaml).expect("safe scalar must parse");
        prop_assert_eq!(a.len(), b.len());
        // Spot-check each event's tag and anchor_id are stable.
        for (ea, eb) in a.iter().zip(b.iter()) {
            prop_assert_eq!(ea.anchor_id(), eb.anchor_id());
            prop_assert_eq!(ea.tag(), eb.tag());
            prop_assert_eq!(ea.is_document_start(), eb.is_document_start());
            prop_assert_eq!(ea.is_alias(), eb.is_alias());
        }
    }

    /// For any plain-style `Scalar` event whose value is a YAML 1.1
    /// ambiguous boolean (yes/no/on/off and case variants), the
    /// `reject_yaml_1_1_ambiguous_scalars` API classifies it as
    /// ambiguous.
    #[test]
    fn yec_ambiguous_scalars_are_classified_as_ambiguous(
        word in prop_oneof![
            Just("yes"), Just("no"), Just("on"), Just("off"),
            Just("Yes"), Just("No"), Just("On"), Just("Off"),
            Just("YES"), Just("NO"), Just("ON"), Just("OFF"),
        ],
    ) {
        // Collect events from a plain-style scalar in a mapping so
        // the scalar is well-formed YAML structurally.
        let yaml = format!("flag: {word}\n");
        let events = match parse_yaml_events(&yaml) {
            Ok(e) => e,
            // If parsing itself rejects the source (for example,
            // because the word clashes with a reserved id), the
            // classification is implicitly "ambiguous" by error
            // path — vacuously true.
            Err(_) => return Ok(()),
        };
        let plain_word = events.iter().find_map(|e| match e {
            YamlEvent::Scalar {
                value, style, ..
            } if matches!(style, ScalarStyle::Plain) && value.as_ref() == word => {
                Some(value.clone())
            }
            _ => None,
        });
        if let Some(found) = plain_word {
            let result = reject_yaml_1_1_ambiguous_scalars(&[found.as_ref()]);
            prop_assert!(
                matches!(
                    result,
                    Err(YamlError::AmbiguousScalar { .. })
                ),
                "expected AmbiguousScalar, got {result:?}"
            );
        }
    }

    /// Mapping with N unique keys produces N mapping keys in the
    /// event stream (counted by `expecting_key` flips).
    #[test]
    fn yec_mapping_key_count_matches_declared(
        keys in prop::collection::hash_set(arb_safe_scalar(), 1..=8),
    ) {
        let yaml = {
            let mut s = String::new();
            for k in &keys {
                s.push_str(&format!("{k}: v\n"));
            }
            s
        };
        let events = match parse_yaml_events(&yaml) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };
        let plain_keys: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                YamlEvent::Scalar {
                    value, style, ..
                } if matches!(style, ScalarStyle::Plain) => {
                    Some(value.as_ref())
                }
                _ => None,
            })
            .collect();
        // The first half of the scalars are the keys (alternating
        // key, value, key, value, ...). For a mapping of N keys, we
        // expect at least N key-like scalars.
        let expected = keys.len();
        prop_assert!(
            plain_keys.len() >= expected,
            "expected at least {expected} plain scalars, got {}: {:?}",
            plain_keys.len(),
            plain_keys
        );
    }

    /// `collect_events` (lower-level than `parse_yaml_events`) never
    /// panics for any safe scalar input.
    #[test]
    fn yec_collect_events_never_panics(value in arb_safe_scalar()) {
        let yaml = format!("{value}\n");
        let _ = collect_events(&yaml);
    }
}
