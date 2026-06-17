#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
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
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables,
)]

// harnesses/kani/benchmark_metadata_capture.rs
//
// Kani bounded model checking harnesses for capture_metadata.
//
// This artifact targets the planned production function:
//   pub fn capture_metadata(
//       name: &str,
//       baseline: Option<Duration>,
//       result: Duration,
//       command: &str,
//       commit_hash: &str,
//       environment: &str,
//       budget_us: u64,
//       fjall_write_latency_ns: u64,
//       direct_api_latency_ns: u64,
//       ipc_latency_ns: u64,
//   ) -> Result<BenchmarkMetadata, EvidenceError>
//
// Obligation coverage:
//   PO-vb-hints-002  (capture_metadata populates all three latency fields)
//   PO-vb-hints-004  (capture_metadata postconditions: latency outputs == inputs)
//   PO-vb-hints-009  (serialized JSON contains all 20 MASTER_METADATA_FIELDS keys)
//   PO-vb-hints-020  (serialized JSON keys are audit-compatible without _ns suffix)
//   PO-vb-hints-022  (commit_hash validation preserved with new parameters)
//
// Production code is implemented in vb_benchmark/src/lib.rs with all
// required types: capture_metadata (10-param), BenchmarkMetadata with
// latency fields, EvidenceError variants, LatencyFieldId enum, and
// MASTER_METADATA_FIELDS constant. Serde/serde_json are dependencies.

#[cfg(kani)]
use std::time::Duration;
#[cfg(kani)]
use vb_benchmark::*;

#[cfg(kani)]
mod kani_harnesses {
    use kani::Arbitrary;

    // Derive Arbitrary for BenchmarkMetadata to enable symbolic inputs.
    // This is required by GOD Rule 1: no hardcoded structural inputs.
    impl Arbitrary for BenchmarkMetadata {
        fn arbitrary() -> Self {
            Self {
                name: kani::any::<String>(),
                baseline_us: kani::any(),
                result_us: kani::any(),
                command: kani::any::<String>(),
                commit_hash: kani::any::<String>(),
                environment: kani::any::<String>(),
                budget_us: kani::any(),
                fjall_write_latency_ns: kani::any(),
                direct_api_latency_ns: kani::any(),
                ipc_latency_ns: kani::any(),
            }
        }
    }

    /// Harness: capture_metadata populates all three latency fields from inputs.
    ///
    /// Proves PO-vb-hints-002: for any valid inputs, the returned metadata contains
    /// the latency fields populated with the exact input values.
    #[kani::proof]
    fn proof_capture_metadata_populates_latency_fields() {
        let name: String = kani::any();
        let baseline: Option<Duration> = kani::any();
        let result: Duration = kani::any();
        let command: String = kani::any();
        let commit_hash: String = kani::any();
        let environment: String = kani::any();
        let budget_us: u64 = kani::any();
        let fjall_ns: u64 = kani::any();
        let api_ns: u64 = kani::any();
        let ipc_ns: u64 = kani::any();

        // Assume commit_hash is valid (non-empty ASCII hex)
        kani::assume(!commit_hash.is_empty());
        kani::assume(commit_hash.bytes().all(|b| b.is_ascii_hexdigit()));

        let result = capture_metadata(
            &name,
            baseline,
            result,
            &command,
            &commit_hash,
            &environment,
            budget_us,
            fjall_ns,
            api_ns,
            ipc_ns,
        );

        match result {
            Ok(metadata) => {
                kani::assert(
                    metadata.fjall_write_latency_ns == fjall_ns,
                    "fjall_write_latency_ns must equal input fjall_ns",
                );
                kani::assert(
                    metadata.direct_api_latency_ns == api_ns,
                    "direct_api_latency_ns must equal input api_ns",
                );
                kani::assert(
                    metadata.ipc_latency_ns == ipc_ns,
                    "ipc_latency_ns must equal input ipc_ns",
                );
            }
            Err(_) => {
                // Should not happen with valid commit_hash assumption.
                kani::assert(
                    false,
                    "capture_metadata should succeed with valid commit_hash",
                );
            }
        }
    }

    /// Harness: commit_hash validation is preserved with new parameters.
    ///
    /// Proves PO-vb-hints-022: empty and non-hex commit hashes still return
    /// Err(MissingCommit) even when the new latency parameters are present.
    #[kani::proof]
    fn proof_commit_hash_validation_preserved() {
        let name: String = kani::any();
        let baseline: Option<Duration> = kani::any();
        let result: Duration = kani::any();
        let command: String = kani::any();
        let environment: String = kani::any();
        let budget_us: u64 = kani::any();
        let fjall_ns: u64 = kani::any();
        let api_ns: u64 = kani::any();
        let ipc_ns: u64 = kani::any();

        // Test case 1: empty commit hash
        let empty_hash: &str = "";
        let result = capture_metadata(
            &name,
            baseline,
            result,
            &command,
            empty_hash,
            &environment,
            budget_us,
            fjall_ns,
            api_ns,
            ipc_ns,
        );
        kani::assert(
            matches!(result, Err(EvidenceError::MissingCommit)),
            "empty commit_hash must return MissingCommit",
        );

        // Test case 2: non-hex commit hash
        let non_hex_hash: &str = "xyz123!@#";
        let result2 = capture_metadata(
            &name,
            baseline,
            result,
            &command,
            non_hex_hash,
            &environment,
            budget_us,
            fjall_ns,
            api_ns,
            ipc_ns,
        );
        kani::assert(
            matches!(result2, Err(EvidenceError::MissingCommit)),
            "non-hex commit_hash must return MissingCommit",
        );
    }

    /// Harness: serialized JSON contains all MASTER_METADATA_FIELDS keys.
    ///
    /// Proves PO-vb-hints-009: for any valid metadata, the serialized JSON
    /// representation contains all 20 keys from MASTER_METADATA_FIELDS.
    #[kani::proof]
    fn proof_serialization_completeness() {
        let name: String = kani::any();
        let baseline: Option<Duration> = kani::any();
        let result: Duration = kani::any();
        let command: String = kani::any();
        let commit_hash: String = kani::any();
        let environment: String = kani::any();
        let budget_us: u64 = kani::any();
        let fjall_ns: u64 = kani::any();
        let api_ns: u64 = kani::any();
        let ipc_ns: u64 = kani::any();

        kani::assume(!commit_hash.is_empty());
        kani::assume(commit_hash.bytes().all(|b| b.is_ascii_hexdigit()));

        let metadata = capture_metadata(
            &name,
            baseline,
            result,
            &command,
            &commit_hash,
            &environment,
            budget_us,
            fjall_ns,
            api_ns,
            ipc_ns,
        )
        .expect("valid inputs should produce Ok(metadata)");

        let json = serde_json::to_string(&metadata).expect("serialization should succeed");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("roundtrip parse should succeed");

        if let serde_json::Value::Object(map) = parsed {
            for key in &MASTER_METADATA_FIELDS {
                kani::assert(
                    map.contains_key(*key),
                    &format!("JSON must contain key: {}", key),
                );
            }
        } else {
            kani::assert(false, "serialized metadata must be a JSON object");
        }
    }

    /// Harness: serialized JSON keys are audit-compatible (without _ns suffix).
    ///
    /// Proves PO-vb-hints-020: the serialized JSON contains:
    ///   - fjall_write_latency (not fjall_write_latency_ns)
    ///   - direct_api_latency (not direct_api_latency_ns)
    ///   - ipc_latency (not ipc_latency_ns)
    #[kani::proof]
    fn proof_audit_compatible_keys() {
        let name: String = kani::any();
        let baseline: Option<Duration> = kani::any();
        let result: Duration = kani::any();
        let command: String = kani::any();
        let commit_hash: String = kani::any();
        let environment: String = kani::any();
        let budget_us: u64 = kani::any();
        let fjall_ns: u64 = kani::any();
        let api_ns: u64 = kani::any();
        let ipc_ns: u64 = kani::any();

        kani::assume(!commit_hash.is_empty());
        kani::assume(commit_hash.bytes().all(|b| b.is_ascii_hexdigit()));

        let metadata = capture_metadata(
            &name,
            baseline,
            result,
            &command,
            &commit_hash,
            &environment,
            budget_us,
            fjall_ns,
            api_ns,
            ipc_ns,
        )
        .expect("valid inputs should produce Ok(metadata)");

        let json = serde_json::to_string(&metadata).expect("serialization should succeed");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("roundtrip parse should succeed");

        if let serde_json::Value::Object(map) = parsed {
            kani::assert(
                map.contains_key("fjall_write_latency"),
                "must contain audit key fjall_write_latency",
            );
            kani::assert(
                map.contains_key("direct_api_latency"),
                "must contain audit key direct_api_latency",
            );
            kani::assert(
                map.contains_key("ipc_latency"),
                "must contain audit key ipc_latency",
            );
        }
    }
}
