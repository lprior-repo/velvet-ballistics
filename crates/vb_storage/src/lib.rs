#![forbid(unsafe_code)]
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
// VERIF-002 (master §77.8 + RED-QUEEN-MASTER-ISSUE-REPORT.md):
// `codec_miri_tests` is a Miri-only harness module. The previous
// `#[cfg(test)]` gate caused the module to compile under plain
// `cargo test`, where its `panic_free_*` helpers rely on
// `std::panic::catch_unwind` semantics that do not actually prove
// the Miri-relevant invariants (raw pointer / uninitialized memory
// / out-of-bounds slices). The `#[cfg(any(test, miri))]` gate keeps
// the module available to `cargo miri test` while letting us assert
// compile-clean behavior under `cfg(miri)` from a regular `cargo
// test` run via the sentinel module below.
#[cfg(any(test, miri))]
pub mod codec_miri_tests;
#[cfg(test)]
mod codec_miri_tests_compile_check;
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
pub mod slot_extra;
pub mod snapshots;
pub mod trimming;
pub mod types;

#[cfg(test)]
mod hydrate_tests;
#[cfg(test)]
mod security_tests;
#[cfg(test)]
mod test_helpers;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod vb_2bok_durability_gate_tests;

// Section 38 property tests (master §38).
// Each submodule covers one named property:
//   - digest_stability
//   - layout_stability
//   - for_each_ordering
//   - bound_enforcement
//   - state_machine
//   - taint_safety
#[cfg(test)]
mod property_tests;
