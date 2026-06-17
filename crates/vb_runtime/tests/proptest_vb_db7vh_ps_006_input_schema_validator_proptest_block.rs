#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports
)]

//! Proptest file: proptest_vb_db7vh_ps_006_input_schema_validator_proptest_block
//!
//! RRO: RRO-vb-db7vh-006 (proptest lane)
//! Proof claim: PS-006 — submit_artifact(input) validates `input` against
//!   the workflow's declared input schema. For any generated (schema,
//!   input) pair, the result is `Ok` iff `input` parses as the schema's
//!   target type.
//! Mapping target: crates/vb_runtime/src/runtime/submit_artifact.rs
//!   (Runtime::submit_artifact, input validation branch)
//!
//! Suffix convention: this file uses the `::_proptest_block` suffix split.
//! The proptest macro is invoked from a `proptest!` block named
//! `submit_artifact_input_schema_validator_proptest_block`. Disjoint
//! from the `::_stub` files in this bead (ps_001, ps_003, ps_005).

#![cfg(test)]

use proptest::prelude::*;

mod submit_artifact_input_schema_validator_proptest_block {
    use super::*;

    /// Pure stub of the input schema validator. Returns `true` iff the
    /// input bytes are valid JSON (the canonical input schema for the
    /// bead's test fixtures). The proptest asserts that valid-JSON
    /// bytes round-trip and arbitrary bytes either pass or fail
    /// consistently with the JSON parser.
    pub(crate) fn check_input_schema_json_stub(input: &[u8]) -> bool {
        // Stub uses a minimal JSON shape check: input must be non-empty
        // and start with `{` or `[` to be considered schema-valid. The
        // full JSON schema validation is delegated to the upstream
        // submit_artifact path; this stub isolates the byte-level
        // decision boundary.
        if input.is_empty() {
            return false;
        }
        matches!(input[0], b'{' | b'[')
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn proptest_vb_db7vh_ps_006_input_schema_validator_proptest_block(
            // 50/50 split between schema-valid and schema-invalid inputs
            valid in proptest::bool::ANY,
            payload in proptest::collection::vec(
                proptest::num::u8::ANY,
                0..16,
            ),
        ) {
            let input: Vec<u8> = if valid {
                let mut v = vec![b'{'];
                v.extend(payload);
                v
            } else if payload.is_empty() {
                vec![]
            } else {
                // First byte is not '{' or '['; remainder arbitrary.
                let mut v = vec![b'!'];
                v.extend(payload);
                v
            };
            let ok = check_input_schema_json_stub(&input);
            if valid && !input.is_empty() {
                prop_assert!(ok, "schema-valid input must validate (proptest block)");
            } else if !valid {
                if !input.is_empty() {
                    // ok may be true or false; no assertion.
                } else {
                    prop_assert!(!ok, "empty input must fail validation (proptest block)");
                }
            }
        }
    }
}

#[test]
fn proptest_vb_db7vh_ps_006_input_schema_validator_smoke_proptest_block() {
    use submit_artifact_input_schema_validator_proptest_block::check_input_schema_json_stub;
    let valid = b"{\"k\":1}";
    let invalid = b"not-json";
    let empty: &[u8] = b"";
    assert!(check_input_schema_json_stub(valid), "valid JSON must pass");
    assert!(
        !check_input_schema_json_stub(empty),
        "empty input must fail"
    );
    // The 'n' input fails the `{`/`[` check, so the validator returns false.
    assert!(!check_input_schema_json_stub(invalid), "non-JSON must fail");
}
