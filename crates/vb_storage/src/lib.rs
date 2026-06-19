#![forbid(unsafe_code)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::comparison_chain)]
//! Fjall append-only journal boundary with full recovery support.
//!
//! Provides digest-mismatch detection, full primitive replay (all node kinds),
//! non-idempotent action blocking during recovery, replay divergence detection,
//! snapshot-plus-tail journal recovery, and full journal recovery when no
//! snapshot is available.

// ============================================================================
// Public surface: thin re-export delegations (implementation in exports.rs)
// ============================================================================

pub use crate::convenience::*;
pub use crate::exports::*;

// ============================================================================
// Submodules
// ============================================================================

pub mod admission;
pub mod artifacts;
pub mod batch;
pub mod binary;
pub mod blobs;
pub mod codec;
#[cfg(test)]
pub mod codec_miri_tests;
pub mod constants;
pub mod error;
pub mod events;
pub mod headers;
pub mod indexes;
pub mod journal;
#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_codec;
pub mod mrwe5_contract;
pub mod mrwe6_seams;

// HVR-PO-STORAGE-002/HVR-PO-STORAGE-005: legacy Kani modules stay out of the vb-god2f feature lane.

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_magic;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_schema;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_kind;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_payload_len;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_record_crc;

#[cfg(all(
    kani,
    any(feature = "legacy-kani", feature = "kani-digest-checks-vb-2bzz")
))]
pub mod kani_digest_checks_vb_2bzz;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_hydrate_proofs;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_admission;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_postcard_envelope_wire;

#[cfg(all(kani, feature = "kani-storage-trailing-bytes"))]
pub mod kani_postcard_envelope_wire_trailing_bytes;

#[cfg(all(kani, feature = "kani-typed-partitioned-ids"))]
pub mod kani_typed_partitioned_ids;

#[cfg(all(kani, feature = "kani-vb-u8gi-decode-taxonomy"))]
pub mod kani_vb_u8gi_storage_decode_order;

#[cfg(all(kani, feature = "kani-vb-u8gi-decode-taxonomy"))]
pub mod kani_vb_u8gi_storage_numeric_fields;

#[cfg(all(kani, feature = "kani-vb-u8gi-decode-taxonomy"))]
pub mod kani_vb_u8gi_storage_payload_bounds;

// --- vb-vzcuf Kani harnesses (PS-001 through PS-009) ---
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps001;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps002;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps003;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps004;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps005;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps006;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps007;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps008;
#[cfg(all(kani, feature = "kani-vb-vzcuf"))]
pub mod kani_vb_vzcuf_ps009;

#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_vbjpq733_proofs;

// HVR-PO-STORAGE-002/HVR-PO-STORAGE-005: feature-isolated vb-god2f storage Kani harnesses.
#[cfg(all(kani, feature = "kani-vb-god2f-hard-verus"))]
pub mod kani_vb_god2f_classification_recovery;

// vb-mrwe.5: StepSucceeded record kind parity proofs
#[cfg(all(kani, feature = "kani-vb-mrwe5"))]
pub mod kani_vb_mrwe5_record_kind;
#[cfg(all(kani, feature = "kani-vb-mrwe5"))]
pub mod kani_vb_mrwe5_step_succeeded_id;
#[cfg(all(kani, feature = "kani-vb-fn4vt"))]
#[path = "verification/vb-fn4vt/kani/policy_digest_binding.rs"]
pub mod vb_fn4vt_policy_digest_binding;
#[cfg(all(kani, feature = "kani-vb-mrwe5"))]
#[path = "verification/kani/vb_mrwe5_compat_kind_family.rs"]
pub mod vb_mrwe5_compat_kind_family;
#[cfg(all(kani, feature = "kani-vb-mrwe5"))]
#[path = "verification/kani/vb_mrwe5_decode_reject.rs"]
pub mod vb_mrwe5_decode_reject;
#[cfg(all(kani, feature = "kani-vb-mrwe5"))]
#[path = "verification/kani/vb_mrwe5_kind_parity.rs"]
pub mod vb_mrwe5_kind_parity;
#[cfg(all(kani, feature = "kani-vb-mrwe5"))]
#[path = "verification/kani/vb_mrwe5_roundtrip.rs"]
pub mod vb_mrwe5_roundtrip;

// vb-mrwe.4: pending_actions recovery proofs
#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_vb_mrwe4_reject_unsupported_state;
#[cfg(all(kani, feature = "legacy-kani"))]
pub mod kani_vb_mrwe4_seed_unsupported_state;

// --- vb-h09wf Kani harnesses (PS-001 through PS-012) ---
// Verification wiring only — no production behavior changes.
#[cfg(all(kani, feature = "kani-vb-h09wf"))]
pub mod kani_vb_h09wf_ps001;
#[cfg(all(kani, feature = "kani-vb-h09wf"))]
pub mod kani_vb_h09wf_ps002;
#[cfg(all(kani, feature = "kani-vb-h09wf"))]
pub mod kani_vb_h09wf_ps003;
#[cfg(all(kani, feature = "kani-vb-h09wf"))]
pub mod kani_vb_h09wf_ps004;
#[cfg(all(kani, feature = "kani-vb-h09wf"))]
pub mod kani_vb_h09wf_ps005;
#[cfg(all(kani, feature = "kani-vb-h09wf"))]
pub mod kani_vb_h09wf_ps006;
#[cfg(all(kani, feature = "kani-vb-h09wf"))]
pub mod kani_vb_h09wf_ps007;
#[cfg(all(kani, feature = "kani-vb-h09wf"))]
pub mod kani_vb_h09wf_ps008;
#[cfg(all(kani, feature = "kani-vb-h09wf"))]
pub mod kani_vb_h09wf_ps009;
#[cfg(all(kani, feature = "kani-vb-h09wf"))]
pub mod kani_vb_h09wf_ps010;
#[cfg(all(kani, feature = "kani-vb-h09wf"))]
pub mod kani_vb_h09wf_ps011;
#[cfg(all(kani, feature = "kani-vb-h09wf"))]
pub mod kani_vb_h09wf_ps012;

#[cfg(all(kani, feature = "kani-vb-mrwe6"))]
#[path = "verification/kani/vb_mrwe6_atomic_index.rs"]
pub mod vb_mrwe6_atomic_index;

#[cfg(all(kani, feature = "kani-vb-mrwe6"))]
#[path = "verification/kani/vb_mrwe6_queue_intent.rs"]
pub mod vb_mrwe6_queue_intent;

#[cfg(all(kani, feature = "kani-vb-mrwe6"))]
#[path = "verification/kani/vb_mrwe6_duplicate_schedule.rs"]
pub mod vb_mrwe6_duplicate_schedule;

#[cfg(all(kani, feature = "kani-vb-mrwe6"))]
#[path = "verification/kani/vb_mrwe6_completion_policy.rs"]
pub mod vb_mrwe6_completion_policy;

#[cfg(all(kani, feature = "kani-vb-mrwe6"))]
#[path = "verification/kani/vb_mrwe6_recovery_reliance.rs"]
pub mod vb_mrwe6_recovery_reliance;

#[cfg(all(kani, feature = "kani-vb-mrwe-7"))]
pub mod kani_vb_mrwe_7_bounds;
#[cfg(all(kani, feature = "kani-vb-mrwe-7"))]
pub mod kani_vb_mrwe_7_commit_before_drain;
#[cfg(all(kani, feature = "kani-vb-mrwe-7"))]
pub mod kani_vb_mrwe_7_concurrency;
#[cfg(all(kani, feature = "kani-vb-mrwe-7"))]
pub mod kani_vb_mrwe_7_drain_all;
#[cfg(all(kani, feature = "kani-vb-mrwe-7"))]
pub mod kani_vb_mrwe_7_duplicates;
#[cfg(all(kani, feature = "kani-vb-mrwe-7"))]
pub mod kani_vb_mrwe_7_durability;
#[cfg(all(kani, feature = "kani-vb-mrwe-7"))]
pub mod kani_vb_mrwe_7_fjall_seam;
#[cfg(all(kani, feature = "kani-vb-mrwe-7"))]
pub mod kani_vb_mrwe_7_recovery;
#[cfg(all(kani, feature = "kani-vb-mrwe-7"))]
pub mod kani_vb_mrwe_7_report;
#[cfg(all(kani, feature = "kani-vb-mrwe-7"))]
#[path = "queue/kani_vb_mrwe_7_atomic.rs"]
pub mod queue_kani_vb_mrwe_7_atomic;

#[cfg(all(flux, feature = "vb-mrwe6-flux-refinements"))]
pub mod mrwe6_flux_storage {
    #[path = "../verification/flux/vb_mrwe6_duplicate_refinements.rs"]
    pub mod vb_mrwe6_duplicate_refinements;
    #[path = "../verification/flux/vb_mrwe6_recovery_refinements.rs"]
    pub mod vb_mrwe6_recovery_refinements;
}

// --- Re-export helpers (implementation in exports.rs) ---
mod exports;

// --- Convenience wrappers + test helpers (implementation in convenience.rs) ---
mod convenience;

pub mod keys;
pub mod preview;
pub mod process_lock;

// PO-010: register the deterministic replay proptest module for `cargo test --lib`
// evidence collection. This is test-only verification wiring and does not alter
// production runtime behavior.
#[cfg(test)]
#[path = "po010_proptests.rs"]
mod proptests;

// vb-b8i8f: proptest_storage.rs disabled — proptest 1.11.0 block-form
// incompatibility. File requires rewrite to single-test form.
// Will be fixed in follow-up bead. See LANDING-NOTE-001.
// #[cfg(test)]
// #[path = "proptest_storage.rs"]
// mod proptest_storage;

#[cfg(test)]
#[path = "proptests.rs"]
mod proptest_integration;

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;

#[cfg(test)]
#[path = "recover_tests.rs"]
mod recover_tests;

pub mod queue;
pub mod records;
pub mod recovery;
pub mod recovery_stamps;
pub mod security_tests;
pub mod slot_extra;
pub mod snapshots;
pub mod tests;
pub mod trimming;
pub mod types;
pub mod vb_2bok_durability_gate_tests;
