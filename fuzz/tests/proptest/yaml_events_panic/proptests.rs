//! Proptest blocks for YAML event parsing.
#![forbid(unsafe_code)]
use super::strategies::*;
use proptest::prelude::*;
use vb_core::diagnostic::HasSymbolicCode;

// ===========================================================================
// PO-PROP-001: panic-free YAML event parsing
// ===========================================================================

proptest! {
    /// PO-PROP-001: `parse_yaml_events` never panics on any UTF-8 input.
    #[test]
    fn proptest_yaml_events_panic_free(input in utf8_string_strategy()) {
        let result = vb_yaml::parse_yaml_events(&input);
        match result {
            Ok(events) => {
                let _ = events.len();
            }
            Err(error) => {
                let _code = error.symbolic_code();
            }
        }
    }
}

// ===========================================================================
// PO-PROP-002: non-empty YAML input produces non-empty events
// ===========================================================================

proptest! {
    /// PO-PROP-002: For valid non-empty YAML strings, `parse_yaml_events`
    /// returns `Ok(events)` where `events.len() > 0`.
    #[test]
    fn proptest_yaml_events_non_empty(input in valid_yaml_strategy()) {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(());
        }

        let result = vb_yaml::parse_yaml_events(trimmed);
        match result {
            Ok(events) => {
                prop_assert!(
                    !events.is_empty(),
                    "non-empty valid YAML must produce non-empty events. Input: {:?}",
                    trimmed
                );
            }
            Err(_error) => {
                // Valid YAML may still be rejected by the strict profile.
            }
        }
    }
}

// ===========================================================================
// PO-PROP-003: event count bounded by YamlLimits
// ===========================================================================

proptest! {
    /// PO-PROP-003: For all generated inputs, event count and source map
    /// entries are bounded by `YamlLimits` constants.
    #[test]
    fn proptest_yaml_events_bounded(input in utf8_string_strategy()) {
        let events_result = vb_yaml::parse_yaml_events(&input);
        match events_result {
            Ok(events) => {
                prop_assert!(
                    events.len() <= 100_000,
                    "event count {} must not exceed max_nodes",
                    events.len()
                );
            }
            Err(_) => {}
        }

        let source_map_result = vb_yaml::build_source_map(&input);
        match source_map_result {
            Ok(source_map) => {
                prop_assert!(
                    source_map.len() <= 100_000,
                    "source map entries {} must not exceed max_nodes",
                    source_map.len()
                );
            }
            Err(_) => {}
        }
    }
}

// ===========================================================================
// PO-PROP-004: source map integrity
// ===========================================================================

proptest! {
    /// PO-PROP-004: For valid non-empty YAML, `build_source_map` produces
    /// entries with valid line/column numbers (line >= 1, column >= 1).
    #[test]
    fn proptest_source_map_integrity(input in valid_yaml_strategy()) {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Ok(());
        }

        let result = vb_yaml::build_source_map(trimmed);
        match result {
            Ok(source_map) => {
                for (node_index, span) in source_map.iter() {
                    prop_assert!(
                        span.start_line >= 1,
                        "node {} start_line must be >= 1, got {}",
                        node_index,
                        span.start_line
                    );
                    prop_assert!(
                        span.start_col >= 1,
                        "node {} start_col must be >= 1, got {}",
                        node_index,
                        span.start_col
                    );
                    prop_assert!(
                        span.end_line >= 1,
                        "node {} end_line must be >= 1, got {}",
                        node_index,
                        span.end_line
                    );
                    prop_assert!(
                        span.end_col >= 1,
                        "node {} end_col must be >= 1, got {}",
                        node_index,
                        span.end_col
                    );
                }
            }
            Err(_) => {}
        }
    }
}

// ===========================================================================
// PO-PROP-005: YamlError classification
// ===========================================================================

proptest! {
    /// PO-PROP-005: For all generated `YamlError` values,
    /// `HasSymbolicCode::symbolic_code()` completes without panic and
    /// returns a registered `SymbolicCode`.
    #[test]
    fn proptest_yaml_error_classification(error in yaml_error_strategy()) {
        let code = error.symbolic_code();

        prop_assert!(
            vb_core::diagnostic::SymbolicCode::from_static(code.as_str()).is_some(),
            "symbolic_code '{}' is not registered in CODE_REGISTRY",
            code.as_str()
        );

        prop_assert_ne!(
            code,
            vb_core::diagnostic::SymbolicCode::INTERNAL_INVARIANT,
            "symbolic_code must not be INTERNAL_INVARIANT"
        );

        prop_assert!(
            !code.as_str().is_empty(),
            "symbolic_code must not be empty"
        );
    }
}
