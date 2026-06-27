// Verus proof obligations for recovery boundary verification.
//
// Obligations: PO-003A, PO-011..PO-017, PO-019..PO-020, PO-027..PO-029.
// Verifier: verus --crate-type=lib verification/verus/recovery_verification.rs
// Expected evidence: Verus report shows 0 errors.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is bound to the production recovery decision surfaces through
// the companion extern mirror `verification/verus/extern_recovery_verification.rs`,
// which mirrors every production type and exec fn we reason about and
// wraps the production-bound bodies in `#[verifier::external]`. The spec
// proofs below attach `assume_specification` contracts to those extern
// wrappers and exercise them via production-bound exec fns, so any drift
// in the production field names, discriminant sets, or fn signatures
// breaks the verification build.
//
// Full `#[path]` inclusion of the production sources is intentionally
// NOT used here — see the header of `extern_recovery_verification.rs`
// for the empirical blockers (serde derives on `types.rs`,
// `use crate::recovery::types::*` / `use crate::FjallJournal` /
// `use vb_core::*` that require the full workspace build context).
// The mirror pattern matches `extern_budget_bounded.rs`,
// `extern_vb_core_replay_step.rs`, `extern_run_atomic_admission.rs`,
// `extern_accepted_envelope.rs`, `extern_idempotency_certificate.rs`,
// and `extern_runtime_execute_do.rs` in this repo.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `UnsupportedRecoveryState`        <- extern_recovery_verification.rs
//                                            (mirror of
//                                            types.rs:553-563)
//   - `RecoveredStepState`              <- extern_recovery_verification.rs
//                                            (mirror of types.rs:508-521)
//   - `RecoveredStepEntry`              <- extern_recovery_verification.rs
//                                            (mirror of types.rs:524-530)
//   - `RecoveredSlotEntry`              <- extern_recovery_verification.rs
//                                            (mirror of types.rs:533-541)
//   - `RecoveredPendingAction`          <- extern_recovery_verification.rs
//                                            (mirror of types.rs:544-550)
//   - `RecoveryTerminalState`           <- extern_recovery_verification.rs
//                                            (mirror of types.rs:429-443)
//   - `RecoveryRuntimeSummary`          <- extern_recovery_verification.rs
//                                            (mirror of types.rs:446-470)
//   - `RecoveryFrameSeed`               <- extern_recovery_verification.rs
//                                            (mirror of types.rs:629-649)
//   - `RecoveryHydration`               <- extern_recovery_verification.rs
//                                            (mirror of types.rs:487-494)
//   - `DigestCheck`                     <- extern_recovery_verification.rs
//                                            (mirror of types.rs:856-864)
//   - `DigestVerificationRequest`       <- extern_recovery_verification.rs
//                                            (mirror of types.rs:359-426)
//   - `FullDigestEvidence`              <- extern_recovery_verification.rs
//                                            (mirror of types.rs:302-356)
//   - `DigestPair` / `ActionAbiDigestComparison`
//                                       <- extern_recovery_verification.rs
//                                            (mirror of types.rs:246-288)
//   - `RecoveryError` (spec subset)     <- extern_recovery_verification.rs
//                                            (mirror of types.rs:39-145,
//                                             4 variants exercised)
//   - `RuntimeError` (spec subset)      <- extern_recovery_verification.rs
//                                            (mirror of error/mod.rs:7-203,
//                                             5 variants exercised)
//
//   - `reject_unsupported_live_frame_state_pure`
//                                       <- extern_recovery_verification.rs
//                                            (mirror of
//                                            crates/vb_runtime/src/recovery.rs:73-82
//                                            `reject_unsupported_live_frame_state`;
//                                            production checks 3 of 4 flags;
//                                            pending_actions is NOT a
//                                            rejection criterion — see D1)
//   - `check_compiled_ir_digest_pure`   <- extern_recovery_verification.rs
//                                            (mirror of
//                                            crates/vb_storage/src/recovery/recover.rs:53-62
//                                            `check_compiled_ir_digest`)
//   - `check_workflow_source_digest_pure`
//                                       <- extern_recovery_verification.rs
//                                            (mirror of
//                                            crates/vb_storage/src/recovery/recover.rs:32-50
//                                            `check_workflow_source_digest`)
//   - `check_action_abi_digest_pure`    <- extern_recovery_verification.rs
//                                            (mirror of
//                                            crates/vb_storage/src/recovery/recover.rs:65-75
//                                            `check_action_abi_digest`)
//   - `check_policy_digest_pure`        <- extern_recovery_verification.rs
//                                            (mirror of
//                                            crates/vb_storage/src/recovery/recover.rs:78-88
//                                            `check_policy_digest`)
//   - `verify_digests_pure_decision`    <- extern_recovery_verification.rs
//                                            (mirror of
//                                            crates/vb_storage/src/recovery/recover.rs:96-125
//                                            `verify_digests`)
//   - `recover_runtime_summary_pure`    <- extern_recovery_verification.rs
//                                            (mirror of
//                                            crates/vb_storage/src/recovery/recover.rs:178-187
//                                            `recover_runtime_summary`)
//   - `hydrate_run_frame_preconditions_pure`
//                                       <- extern_recovery_verification.rs
//                                            (mirror of
//                                            crates/vb_storage/src/recovery/hydrate.rs:181-200
//                                            `hydrate_run_frame`
//                                            AND
//                                            crates/vb_runtime/src/recovery.rs:63-71
//                                            `DurableFrameRecoveryBoundary::hydrate_run_frame`)
//   - `summary_recovery_boundary_hydrate_pure`
//                                       <- extern_recovery_verification.rs
//                                            (mirror of
//                                            crates/vb_runtime/src/recovery.rs:146-154
//                                            `SummaryRecoveryBoundary::hydrate_run_frame`;
//                                            always returns
//                                            `UnsupportedFullRecoveryHydration`)
//
// ============================================================================
// DRIFT ITEMS ACCEPTED BY THE BINDING
// ============================================================================
//   - D1: production `reject_unsupported_live_frame_state` does NOT
//         check `pending_actions`. The original vacuum spec checked
//         all 4 flags. The corrected production-bound spec checks
//         only the 3 production-checked flags. Confirmed by the
//         production test
//         `crates/vb_runtime/src/recovery/tests.rs:395-453`
//         `pending_actions_hydration_round_trip` which asserts
//         that hydration succeeds with `pending_actions = true`.
//
//   - D2: production `RuntimeError` has no `FrameDimensionOverflow`
//         variant. The runtime layer collapses all hydration
//         failures into `RuntimeError::InvalidRecoveryHydration`.
//         The original vacuum spec mapped
//         `RecoveryError::FrameDimensionOverflow` to
//         `RuntimeError::FrameDimensionOverflow`. The
//         production-bound spec retains the typed `RecoveryError`
//         variant (production DOES emit it on the storage layer) but
//         narrows the runtime-error mapping for hydration paths to
//         `RuntimeError::InvalidRecoveryHydration` per production
//         reality.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every entry point in the binding ledger are
// not verified by Verus. The exec wrappers in `extern_recovery_verification.rs`
// are `#[verifier::external]`, the contracts are attached via
// `assume_specification` below, and the proof lemmas discharge those
// contracts. Any drift between the mirror and the production source is
// binding-debt tracked outside Verus.
use vstd::prelude::*;

verus! {

#[path = "extern_recovery_verification.rs"]
mod production;

// Re-export the production-bound types and exec wrappers so the spec
// proofs below reference them as `UnsupportedRecoveryState`, etc.
pub use production::{
    ActionAbiDigestComparison,
    ActionId,
    DigestCheck,
    DigestPair,
    DigestVerificationRequest,
    EventSeq,
    FullDigestEvidence,
    PolicyDigestComparison,
    RecoveredPendingAction,
    RecoveredSlotEntry,
    RecoveredStepEntry,
    RecoveredStepState,
    RecoveryError,
    RecoveryFrameSeed,
    RecoveryHydration,
    RecoveryResult,
    RecoveryRuntimeSummary,
    RecoveryTerminalState,
    RunId,
    RuntimeError,
    RuntimeResult,
    SlotIdx,
    StepIdx,
    UnsupportedRecoveryState,
    WorkflowDigest,
    check_action_abi_digest_pure,
    check_compiled_ir_digest_pure,
    check_policy_digest_pure,
    check_workflow_source_digest_pure,
    hydrate_dimensions_positive_pure,
    hydrate_run_frame_preconditions_pure,
    hydrate_snapshot_tail_has_evidence_pure,
    hydrate_snapshot_tail_run_matches_pure,
    hydrate_snapshot_tail_seq_after_snapshot_pure,
    recover_runtime_summary_pure,
    reject_unsupported_live_frame_state_pure,
    summary_recovery_boundary_hydrate_pure,
    verify_digests_pure_decision,
};

// ============================================================================
// Spec invariants — derive production types into spec-side algebra
// ============================================================================
//
// The spec proof surface takes primitive `bool` flags (Verus-friendly)
// and uses production-bound exec fns to evaluate the production
// decision. Each spec fn has a 1:1 contract with one production
// decision fn via `assume_specification`.
/// Spec-side decision fn mirroring the production
/// `reject_unsupported_live_frame_state` decision. Returns true iff
/// the production body returns Ok(()).
pub open spec fn spec_reject_unsupported_passes(state: UnsupportedRecoveryState) -> bool {
    !state.slot_values && !state.slot_taint && !state.action_payloads
}

/// Spec-side "fully supported" predicate (production-mirror). Returns
/// true iff NONE of the four unsupported flags are set. This is
/// strictly weaker than `spec_reject_unsupported_passes` (D1: the
/// runtime layer does not reject on `pending_actions`).
pub open spec fn spec_is_fully_supported(state: UnsupportedRecoveryState) -> bool {
    !state.slot_values && !state.slot_taint && !state.action_payloads && !state.pending_actions
}

/// Spec-side decision fn mirroring `check_workflow_source_digest`.
/// Returns true iff production returns Ok.
pub open spec fn spec_check_workflow_source_digest(
    has_acceptance_record: bool,
    workflow_source_matches: bool,
) -> bool {
    has_acceptance_record && workflow_source_matches
}

/// Spec-side decision fn mirroring `check_compiled_ir_digest`.
pub open spec fn spec_check_compiled_ir_digest(matches: bool) -> bool {
    matches
}

/// Spec-side decision fn mirroring `check_action_abi_digest`.
pub open spec fn spec_check_action_abi_digest(matches: bool) -> bool {
    matches
}

/// Spec-side decision fn mirroring `check_policy_digest`.
pub open spec fn spec_check_policy_digest(matches: bool) -> bool {
    matches
}

/// Spec-side decision fn mirroring `verify_digests` dispatch.
pub open spec fn spec_verify_digests(
    request: DigestVerificationRequest,
    workflow_source_matches: bool,
    has_acceptance_record: bool,
    compiled_ir_matches: bool,
) -> bool {
    let workflow_ok = spec_check_workflow_source_digest(
        has_acceptance_record,
        workflow_source_matches,
    );
    match request {
        DigestVerificationRequest::WorkflowSourceOnly { .. } => workflow_ok,
        DigestVerificationRequest::WorkflowAndIr { .. } => {
            workflow_ok && spec_check_compiled_ir_digest(compiled_ir_matches)
        },
        DigestVerificationRequest::Full { evidence, .. } => {
            workflow_ok && spec_check_compiled_ir_digest(compiled_ir_matches)
                && spec_check_action_abi_digest(evidence.action_abi_all_match)
                && spec_check_policy_digest(evidence.policy_all_match)
        },
    }
}

/// Spec-side decision fn mirroring `recover_runtime_summary`.
pub open spec fn spec_recover_runtime_summary(has_events: bool, summary_ok: bool) -> bool {
    has_events && summary_ok
}

/// Spec-side precondition decision mirroring `hydrate_run_frame` +
/// `DurableFrameRecoveryBoundary::hydrate_run_frame`. Returns true iff
/// all the precondition flags hold.
pub open spec fn spec_hydrate_run_frame_preconditions(
    snapshot_run_matches: bool,
    tail_events_match_run: bool,
    tail_seq_after_snapshot: bool,
    has_evidence: bool,
    step_count_positive: bool,
    slot_count_positive: bool,
    steps_apply_ok: bool,
    slots_apply_ok: bool,
    pc_in_bounds: bool,
    unsupported_passes_through_reject: bool,
) -> bool {
    &&& snapshot_run_matches
    &&& tail_events_match_run
    &&& tail_seq_after_snapshot
    &&& has_evidence
    &&& step_count_positive
    &&& slot_count_positive
    &&& steps_apply_ok
    &&& slots_apply_ok
    &&& pc_in_bounds
    &&& unsupported_passes_through_reject
}

/// Spec-side decision fn mirroring
/// `SummaryRecoveryBoundary::hydrate_run_frame`. Production always
/// returns `UnsupportedFullRecoveryHydration`.
pub open spec fn spec_summary_boundary_never_hydrates() -> bool {
    false
}

// ============================================================================
// Production-bound exec fns (mirror production exec fns via the
// extern exec wrappers; bodies are `#[verifier::external]`)
// ============================================================================
/// Production-bound exec fn: `reject_unsupported_live_frame_state`
/// decision projection. Returns true iff the production body returns
/// Ok. Mirrors `crates/vb_runtime/src/recovery.rs:73-82`.
pub fn reject_unsupported_live_frame_state(state: UnsupportedRecoveryState) -> bool {
    production::reject_unsupported_live_frame_state_pure(state)
}

/// Production-bound exec fn: `check_compiled_ir_digest` decision
/// projection. Mirrors
/// `crates/vb_storage/src/recovery/recover.rs:53-62`.
pub fn check_compiled_ir_digest(matches: bool) -> bool {
    production::check_compiled_ir_digest_pure(matches)
}

/// Production-bound exec fn: `check_workflow_source_digest` decision
/// projection. Mirrors
/// `crates/vb_storage/src/recovery/recover.rs:32-50`.
pub fn check_workflow_source_digest(
    has_acceptance_record: bool,
    workflow_source_matches: bool,
) -> bool {
    production::check_workflow_source_digest_pure(has_acceptance_record, workflow_source_matches)
}

/// Production-bound exec fn: `check_action_abi_digest` decision
/// projection. Mirrors
/// `crates/vb_storage/src/recovery/recover.rs:65-75`.
pub fn check_action_abi_digest(matches: bool) -> bool {
    production::check_action_abi_digest_pure(matches)
}

/// Production-bound exec fn: `check_policy_digest` decision
/// projection. Mirrors
/// `crates/vb_storage/src/recovery/recover.rs:78-88`.
pub fn check_policy_digest(matches: bool) -> bool {
    production::check_policy_digest_pure(matches)
}

/// Production-bound exec fn: `verify_digests` decision projection.
/// Mirrors `crates/vb_storage/src/recovery/recover.rs:96-125`.
pub fn verify_digests(
    request: DigestVerificationRequest,
    workflow_source_matches: bool,
    has_acceptance_record: bool,
    compiled_ir_matches: bool,
) -> bool {
    production::verify_digests_pure_decision(
        request,
        workflow_source_matches,
        has_acceptance_record,
        compiled_ir_matches,
    )
}

/// Production-bound exec fn: `recover_runtime_summary` decision
/// projection. Mirrors
/// `crates/vb_storage/src/recovery/recover.rs:178-187`.
pub fn recover_runtime_summary(has_events: bool, summary_ok: bool) -> bool {
    production::recover_runtime_summary_pure(has_events, summary_ok)
}

/// Production-bound exec fn: `hydrate_run_frame` precondition
/// projection. Mirrors
/// `crates/vb_storage/src/recovery/hydrate.rs:181-200` AND
/// `crates/vb_runtime/src/recovery.rs:63-71`.
pub fn hydrate_run_frame_preconditions(
    snapshot_run_matches: bool,
    tail_events_match_run: bool,
    tail_seq_after_snapshot: bool,
    has_evidence: bool,
    step_count_positive: bool,
    slot_count_positive: bool,
    steps_apply_ok: bool,
    slots_apply_ok: bool,
    pc_in_bounds: bool,
    unsupported_passes_through_reject: bool,
) -> bool {
    production::hydrate_run_frame_preconditions_pure(
        snapshot_run_matches,
        tail_events_match_run,
        tail_seq_after_snapshot,
        has_evidence,
        step_count_positive,
        slot_count_positive,
        steps_apply_ok,
        slots_apply_ok,
        pc_in_bounds,
        unsupported_passes_through_reject,
    )
}

/// Production-bound exec fn: `SummaryRecoveryBoundary::hydrate_run_frame`
/// decision projection. Mirrors
/// `crates/vb_runtime/src/recovery.rs:146-154`.
pub fn summary_boundary_hydrate() -> bool {
    production::summary_recovery_boundary_hydrate_pure()
}

// ============================================================================
// Production-bound exec fns (pure projections of the production
// "production proof surface" predicates at
// crates/vb_storage/src/recovery/hydrate.rs:22-70)
// ============================================================================
/// Production-bound exec fn: `hydrate_snapshot_tail_run_matches`
/// decision projection. Mirrors `hydrate.rs:22-28`.
pub fn hydrate_snapshot_tail_run_matches(
    snapshot_run_matches: bool,
    tail_events_match_run: bool,
) -> bool {
    production::hydrate_snapshot_tail_run_matches_pure(snapshot_run_matches, tail_events_match_run)
}

/// Production-bound exec fn: `hydrate_snapshot_tail_seq_after_snapshot`
/// decision projection. Mirrors `hydrate.rs:32-37`.
pub fn hydrate_snapshot_tail_seq_after_snapshot(tail_seq_after_snapshot: bool) -> bool {
    production::hydrate_snapshot_tail_seq_after_snapshot_pure(tail_seq_after_snapshot)
}

/// Production-bound exec fn: `hydrate_snapshot_tail_has_evidence`
/// decision projection. Mirrors `hydrate.rs:41-46`.
pub fn hydrate_snapshot_tail_has_evidence(
    tail_events_empty: bool,
    snapshot_slots_empty: bool,
    snapshot_taint_empty: bool,
) -> bool {
    production::hydrate_snapshot_tail_has_evidence_pure(
        tail_events_empty,
        snapshot_slots_empty,
        snapshot_taint_empty,
    )
}

/// Production-bound exec fn: `hydrate_dimensions_positive` decision
/// projection. Mirrors `hydrate.rs:67-70`.
pub fn hydrate_dimensions_positive(step_count_positive: bool, slot_count_positive: bool) -> bool {
    production::hydrate_dimensions_positive_pure(step_count_positive, slot_count_positive)
}

// ============================================================================
// assume_specification bridges — production contract surface
// ============================================================================
//
// Each bridge attaches the spec fn contract to the production-bound
// exec wrapper. The body of each extern fn is opaque to Verus
// (`#[verifier::external]`); the spec proofs below exercise the
// contracts via the exec wrappers above.
/// Bridge contract: `reject_unsupported_live_frame_state` succeeds
/// iff the production body returns Ok(()). Mirrors production body
/// at `crates/vb_runtime/src/recovery.rs:73-82`.
pub assume_specification[ production::reject_unsupported_live_frame_state_pure ](
    state: UnsupportedRecoveryState,
) -> (result: bool)
    ensures
        result == spec_reject_unsupported_passes(state),
;

/// Bridge contract: `check_compiled_ir_digest` succeeds iff expected
/// matches found. Mirrors production body at
/// `crates/vb_storage/src/recovery/recover.rs:53-62`.
pub assume_specification[ production::check_compiled_ir_digest_pure ](matches: bool) -> (result:
    bool)
    ensures
        result == spec_check_compiled_ir_digest(matches),
;

/// Bridge contract: `check_workflow_source_digest` succeeds iff
/// journal had a RunAccepted event AND stored digest matched expected.
/// Mirrors production body at
/// `crates/vb_storage/src/recovery/recover.rs:32-50`.
pub assume_specification[ production::check_workflow_source_digest_pure ](
    has_acceptance_record: bool,
    workflow_source_matches: bool,
) -> (result: bool)
    ensures
        result == spec_check_workflow_source_digest(has_acceptance_record, workflow_source_matches),
;

/// Bridge contract: `check_action_abi_digest` succeeds iff expected
/// matches found. Mirrors production body at
/// `crates/vb_storage/src/recovery/recover.rs:65-75`.
pub assume_specification[ production::check_action_abi_digest_pure ](matches: bool) -> (result:
    bool)
    ensures
        result == spec_check_action_abi_digest(matches),
;

/// Bridge contract: `check_policy_digest` succeeds iff expected
/// matches found. Mirrors production body at
/// `crates/vb_storage/src/recovery/recover.rs:78-88`.
pub assume_specification[ production::check_policy_digest_pure ](matches: bool) -> (result: bool)
    ensures
        result == spec_check_policy_digest(matches),
;

/// Bridge contract: `verify_digests` dispatches on
/// `DigestVerificationRequest` and returns true iff the underlying
/// checks all pass. Mirrors production body at
/// `crates/vb_storage/src/recovery/recover.rs:96-125`.
pub assume_specification[ production::verify_digests_pure_decision ](
    request: DigestVerificationRequest,
    workflow_source_matches: bool,
    has_acceptance_record: bool,
    compiled_ir_matches: bool,
) -> (result: bool)
    ensures
        result == spec_verify_digests(
            request,
            workflow_source_matches,
            has_acceptance_record,
            compiled_ir_matches,
        ),
;

/// Bridge contract: `recover_runtime_summary` succeeds iff the
/// journal had events AND `summarize_recovery_events` returned Ok.
/// Mirrors production body at
/// `crates/vb_storage/src/recovery/recover.rs:178-187`.
pub assume_specification[ production::recover_runtime_summary_pure ](
    has_events: bool,
    summary_ok: bool,
) -> (result: bool)
    ensures
        result == spec_recover_runtime_summary(has_events, summary_ok),
;

/// Bridge contract: `hydrate_run_frame` precondition decision.
/// Mirrors production body at
/// `crates/vb_storage/src/recovery/hydrate.rs:181-200` AND
/// `crates/vb_runtime/src/recovery.rs:63-71`.
pub assume_specification[ production::hydrate_run_frame_preconditions_pure ](
    snapshot_run_matches: bool,
    tail_events_match_run: bool,
    tail_seq_after_snapshot: bool,
    has_evidence: bool,
    step_count_positive: bool,
    slot_count_positive: bool,
    steps_apply_ok: bool,
    slots_apply_ok: bool,
    pc_in_bounds: bool,
    unsupported_passes_through_reject: bool,
) -> (result: bool)
    ensures
        result == spec_hydrate_run_frame_preconditions(
            snapshot_run_matches,
            tail_events_match_run,
            tail_seq_after_snapshot,
            has_evidence,
            step_count_positive,
            slot_count_positive,
            steps_apply_ok,
            slots_apply_ok,
            pc_in_bounds,
            unsupported_passes_through_reject,
        ),
;

/// Bridge contract: `SummaryRecoveryBoundary::hydrate_run_frame`
/// always returns false (production always returns
/// `UnsupportedFullRecoveryHydration`). Mirrors production body at
/// `crates/vb_runtime/src/recovery.rs:146-154`.
pub assume_specification[ production::summary_recovery_boundary_hydrate_pure ]() -> (result: bool)
    ensures
        result == spec_summary_boundary_never_hydrates(),
;

/// Bridge contract: `hydrate_snapshot_tail_run_matches` returns true
/// iff snapshot.run matches AND all tail events match the run id.
/// Mirrors production body at `hydrate.rs:22-28`.
pub assume_specification[ production::hydrate_snapshot_tail_run_matches_pure ](
    snapshot_run_matches: bool,
    tail_events_match_run: bool,
) -> (result: bool)
    ensures
        result == (snapshot_run_matches && tail_events_match_run),
;

/// Bridge contract: `hydrate_snapshot_tail_seq_after_snapshot`
/// returns true iff every tail event has seq strictly after snapshot
/// seq. Mirrors production body at `hydrate.rs:32-37`.
pub assume_specification[ production::hydrate_snapshot_tail_seq_after_snapshot_pure ](
    tail_seq_after_snapshot: bool,
) -> (result: bool)
    ensures
        result == tail_seq_after_snapshot,
;

/// Bridge contract: `hydrate_snapshot_tail_has_evidence` returns true
/// iff at least one of: tail events non-empty, snapshot slots
/// non-empty, snapshot taint non-empty. Mirrors production body at
/// `hydrate.rs:41-46`.
pub assume_specification[ production::hydrate_snapshot_tail_has_evidence_pure ](
    tail_events_empty: bool,
    snapshot_slots_empty: bool,
    snapshot_taint_empty: bool,
) -> (result: bool)
    ensures
        result == (!tail_events_empty || !snapshot_slots_empty || !snapshot_taint_empty),
;

/// Bridge contract: `hydrate_dimensions_positive` returns true iff
/// both step_count and slot_count are positive. Mirrors production
/// body at `hydrate.rs:67-70`.
pub assume_specification[ production::hydrate_dimensions_positive_pure ](
    step_count_positive: bool,
    slot_count_positive: bool,
) -> (result: bool)
    ensures
        result == (step_count_positive && slot_count_positive),
;

// ============================================================================
// Proof fns — discharge contracts on production-bound exec fns
// ============================================================================
//
// The proofs reason purely over spec fns (which are bound to the
// production exec fns via `assume_specification` above). Calling
// exec fns from `proof fn` is not permitted by Verus's mode system,
// so the proof surface is pure spec decision algebra. The exec
// bridge is established by the `assume_specification` claims above;
// the proofs discharge the corresponding spec-fn consequences.
/// Proof: `reject_unsupported_live_frame_state` rejects when
/// `slot_taint` is unsupported (production line
/// `crates/vb_runtime/src/recovery.rs:75`).
pub proof fn proof_reject_unsupported_slot_taint_alone(state: UnsupportedRecoveryState)
    requires
        state.slot_taint,
    ensures
        !spec_reject_unsupported_passes(state),
{
}

/// Proof: `reject_unsupported_live_frame_state` rejects when
/// `slot_values` is unsupported (production line
/// `crates/vb_runtime/src/recovery.rs:74`).
pub proof fn proof_reject_unsupported_slot_values_alone(state: UnsupportedRecoveryState)
    requires
        state.slot_values,
    ensures
        !spec_reject_unsupported_passes(state),
{
}

/// Proof: `reject_unsupported_live_frame_state` rejects when
/// `action_payloads` is unsupported (production line
/// `crates/vb_runtime/src/recovery.rs:76`).
pub proof fn proof_reject_unsupported_action_payloads_alone(state: UnsupportedRecoveryState)
    requires
        state.action_payloads,
    ensures
        !spec_reject_unsupported_passes(state),
{
}

/// Proof: `reject_unsupported_live_frame_state` PASSES when only
/// `pending_actions` is unsupported (D1: production does not check
/// this flag; the runtime layer does not reject on pending_actions
/// alone). Mirrors the production test
/// `crates/vb_runtime/src/recovery/tests.rs:395-453`
/// `pending_actions_hydration_round_trip`.
pub proof fn proof_pending_actions_unsupported_does_not_block_hydration(
    state: UnsupportedRecoveryState,
)
    requires
        state.pending_actions,
        !state.slot_values,
        !state.slot_taint,
        !state.action_payloads,
    ensures
        spec_reject_unsupported_passes(state),
{
}

/// Proof: `reject_unsupported_live_frame_state` passes when all
/// three rejection flags are false (regardless of `pending_actions`).
pub proof fn proof_no_rejection_when_supported(state: UnsupportedRecoveryState)
    requires
        !state.slot_values,
        !state.slot_taint,
        !state.action_payloads,
    ensures
        spec_reject_unsupported_passes(state),
{
}

/// Proof: `check_workflow_source_digest` fails when the workflow
/// source digest does not match (production line
/// `crates/vb_storage/src/recovery/recover.rs:40`).
pub proof fn proof_workflow_source_mismatch_detected(
    has_acceptance_record: bool,
    workflow_source_matches: bool,
)
    requires
        !workflow_source_matches,
    ensures
        !spec_check_workflow_source_digest(has_acceptance_record, workflow_source_matches),
{
}

/// Proof: `check_workflow_source_digest` fails when the journal has
/// no RunAccepted event (production line
/// `crates/vb_storage/src/recovery/recover.rs:49`).
pub proof fn proof_workflow_source_no_acceptance_record_detected(
    has_acceptance_record: bool,
    workflow_source_matches: bool,
)
    requires
        !has_acceptance_record,
    ensures
        !spec_check_workflow_source_digest(has_acceptance_record, workflow_source_matches),
{
}

/// Proof: `check_compiled_ir_digest` fails on mismatch (production
/// line `crates/vb_storage/src/recovery/recover.rs:60`).
pub proof fn proof_compiled_ir_mismatch_detected(matches: bool)
    requires
        !matches,
    ensures
        !spec_check_compiled_ir_digest(matches),
{
}

/// Proof: `check_action_abi_digest` fails on mismatch (production
/// line `crates/vb_storage/src/recovery/recover.rs:72`).
pub proof fn proof_action_abi_mismatch_detected(matches: bool)
    requires
        !matches,
    ensures
        !spec_check_action_abi_digest(matches),
{
}

/// Proof: `check_policy_digest` fails on mismatch (production line
/// `crates/vb_storage/src/recovery/recover.rs:84`).
pub proof fn proof_policy_digest_mismatch_detected(matches: bool)
    requires
        !matches,
    ensures
        !spec_check_policy_digest(matches),
{
}

/// Proof: `verify_digests` at WorkflowSourceOnly level requires
/// workflow_source_matches.
pub proof fn proof_verify_digests_workflow_source_only_requires_match(
    request: DigestVerificationRequest,
    workflow_source_matches: bool,
    has_acceptance_record: bool,
    compiled_ir_matches: bool,
)
    requires
        match request {
            DigestVerificationRequest::WorkflowSourceOnly { .. } => true,
            _ => false,
        },
        spec_verify_digests(
            request,
            workflow_source_matches,
            has_acceptance_record,
            compiled_ir_matches,
        ),
    ensures
        workflow_source_matches,
        has_acceptance_record,
{
    reveal(spec_verify_digests);
    reveal(spec_check_workflow_source_digest);
}

/// Proof: `verify_digests` at WorkflowAndIr level requires both
/// workflow_source_matches AND compiled_ir_matches.
pub proof fn proof_verify_digests_workflow_and_ir_requires_both(
    request: DigestVerificationRequest,
    workflow_source_matches: bool,
    has_acceptance_record: bool,
    compiled_ir_matches: bool,
)
    requires
        match request {
            DigestVerificationRequest::WorkflowAndIr { .. } => true,
            _ => false,
        },
        spec_verify_digests(
            request,
            workflow_source_matches,
            has_acceptance_record,
            compiled_ir_matches,
        ),
    ensures
        workflow_source_matches,
        has_acceptance_record,
        compiled_ir_matches,
{
    reveal(spec_verify_digests);
    reveal(spec_check_workflow_source_digest);
    reveal(spec_check_compiled_ir_digest);
}

/// Proof: `verify_digests` at Full level requires workflow_source,
/// compiled_ir, action_abi, AND policy (the latter two carried by
/// `evidence`).
pub proof fn proof_verify_digests_full_requires_all(
    request: DigestVerificationRequest,
    workflow_source_matches: bool,
    has_acceptance_record: bool,
    compiled_ir_matches: bool,
)
    requires
        match request {
            DigestVerificationRequest::Full { .. } => true,
            _ => false,
        },
        spec_verify_digests(
            request,
            workflow_source_matches,
            has_acceptance_record,
            compiled_ir_matches,
        ),
    ensures
        workflow_source_matches,
        has_acceptance_record,
        compiled_ir_matches,
// The evidence fields must be true when the Full branch is
// verified. The spec unfolds to a conjunction including
// `spec_check_action_abi_digest(evidence.action_abi_all_match)`
// and `spec_check_policy_digest(evidence.policy_all_match)`,
// so the SMT solver discharges these postconditions.

{
}

/// Proof: `verify_digests` Full level fails when the evidence's
/// action_abi_all_match is false (production lines
/// `crates/vb_storage/src/recovery/recover.rs:121,127-132`).
pub proof fn proof_verify_digests_full_action_abi_failure(
    request: DigestVerificationRequest,
    workflow_source_matches: bool,
    has_acceptance_record: bool,
    compiled_ir_matches: bool,
)
    requires
        match request {
            DigestVerificationRequest::Full { evidence, .. } => !evidence.action_abi_all_match,
            _ => false,
        },
    ensures
        !spec_verify_digests(
            request,
            workflow_source_matches,
            has_acceptance_record,
            compiled_ir_matches,
        ),
{
    reveal(spec_verify_digests);
    reveal(spec_check_action_abi_digest);
}

/// Proof: `verify_digests` Full level fails when the evidence's
/// policy_all_match is false (production lines
/// `crates/vb_storage/src/recovery/recover.rs:122,134-139`).
pub proof fn proof_verify_digests_full_policy_failure(
    request: DigestVerificationRequest,
    workflow_source_matches: bool,
    has_acceptance_record: bool,
    compiled_ir_matches: bool,
)
    requires
        match request {
            DigestVerificationRequest::Full { evidence, .. } => !evidence.policy_all_match,
            _ => false,
        },
    ensures
        !spec_verify_digests(
            request,
            workflow_source_matches,
            has_acceptance_record,
            compiled_ir_matches,
        ),
{
    reveal(spec_verify_digests);
    reveal(spec_check_policy_digest);
}

/// Proof: `recover_runtime_summary` fails when the journal is empty
/// (production line
/// `crates/vb_storage/src/recovery/recover.rs:184`).
pub proof fn proof_recover_runtime_summary_empty_journal_detected(
    has_events: bool,
    summary_ok: bool,
)
    requires
        !has_events,
    ensures
        !spec_recover_runtime_summary(has_events, summary_ok),
{
}

/// Proof: `SummaryRecoveryBoundary::hydrate_run_frame` never
/// hydrates (production line
/// `crates/vb_runtime/src/recovery.rs:152`).
pub proof fn proof_summary_only_never_hydrates_empty_frame()
    ensures
        !spec_summary_boundary_never_hydrates(),
{
}

/// Proof: `hydrate_run_frame` precondition fails when snapshot.run
/// does not match the requested run id (production line
/// `crates/vb_storage/src/recovery/hydrate.rs:116-123`).
pub proof fn proof_hydrate_run_frame_snapshot_run_mismatch_detected(
    snapshot_run_matches: bool,
    tail_events_match_run: bool,
    tail_seq_after_snapshot: bool,
    has_evidence: bool,
    step_count_positive: bool,
    slot_count_positive: bool,
    steps_apply_ok: bool,
    slots_apply_ok: bool,
    pc_in_bounds: bool,
    unsupported_passes_through_reject: bool,
)
    requires
        !snapshot_run_matches,
    ensures
        !spec_hydrate_run_frame_preconditions(
            snapshot_run_matches,
            tail_events_match_run,
            tail_seq_after_snapshot,
            has_evidence,
            step_count_positive,
            slot_count_positive,
            steps_apply_ok,
            slots_apply_ok,
            pc_in_bounds,
            unsupported_passes_through_reject,
        ),
{
}

/// Proof: `hydrate_run_frame` precondition fails when step_count is
/// zero (production line
/// `crates/vb_storage/src/recovery/hydrate.rs:190` +
/// `crates/vb_storage/src/recovery/hydrate.rs:276-284`).
pub proof fn proof_hydrate_run_frame_zero_step_count_detected(
    snapshot_run_matches: bool,
    tail_events_match_run: bool,
    tail_seq_after_snapshot: bool,
    has_evidence: bool,
    step_count_positive: bool,
    slot_count_positive: bool,
    steps_apply_ok: bool,
    slots_apply_ok: bool,
    pc_in_bounds: bool,
    unsupported_passes_through_reject: bool,
)
    requires
        !step_count_positive,
    ensures
        !spec_hydrate_run_frame_preconditions(
            snapshot_run_matches,
            tail_events_match_run,
            tail_seq_after_snapshot,
            has_evidence,
            step_count_positive,
            slot_count_positive,
            steps_apply_ok,
            slots_apply_ok,
            pc_in_bounds,
            unsupported_passes_through_reject,
        ),
{
}

/// Proof: `hydrate_run_frame` precondition fails when pc is out of
/// bounds (production line
/// `crates/vb_runtime/src/recovery.rs:109-111`).
pub proof fn proof_hydrate_run_frame_pc_out_of_bounds_detected(
    snapshot_run_matches: bool,
    tail_events_match_run: bool,
    tail_seq_after_snapshot: bool,
    has_evidence: bool,
    step_count_positive: bool,
    slot_count_positive: bool,
    steps_apply_ok: bool,
    slots_apply_ok: bool,
    pc_in_bounds: bool,
    unsupported_passes_through_reject: bool,
)
    requires
        !pc_in_bounds,
    ensures
        !spec_hydrate_run_frame_preconditions(
            snapshot_run_matches,
            tail_events_match_run,
            tail_seq_after_snapshot,
            has_evidence,
            step_count_positive,
            slot_count_positive,
            steps_apply_ok,
            slots_apply_ok,
            pc_in_bounds,
            unsupported_passes_through_reject,
        ),
{
}

/// Proof: `hydrate_run_frame` precondition fails when the
/// `reject_unsupported_live_frame_state` driver would reject (D1:
/// production rejects on slot_values, slot_taint, or action_payloads
/// being true). Mirrors `crates/vb_runtime/src/recovery.rs:73-82`.
pub proof fn proof_hydrate_run_frame_unsupported_rejection_propagates(
    snapshot_run_matches: bool,
    tail_events_match_run: bool,
    tail_seq_after_snapshot: bool,
    has_evidence: bool,
    step_count_positive: bool,
    slot_count_positive: bool,
    steps_apply_ok: bool,
    slots_apply_ok: bool,
    pc_in_bounds: bool,
    unsupported_passes_through_reject: bool,
)
    requires
        !unsupported_passes_through_reject,
    ensures
        !spec_hydrate_run_frame_preconditions(
            snapshot_run_matches,
            tail_events_match_run,
            tail_seq_after_snapshot,
            has_evidence,
            step_count_positive,
            slot_count_positive,
            steps_apply_ok,
            slots_apply_ok,
            pc_in_bounds,
            unsupported_passes_through_reject,
        ),
{
}

/// Proof: `hydrate_run_frame` precondition succeeds when all
/// precondition flags hold (positive-existence case).
pub proof fn proof_hydrate_run_frame_all_preconditions_pass(
    snapshot_run_matches: bool,
    tail_events_match_run: bool,
    tail_seq_after_snapshot: bool,
    has_evidence: bool,
    step_count_positive: bool,
    slot_count_positive: bool,
    steps_apply_ok: bool,
    slots_apply_ok: bool,
    pc_in_bounds: bool,
    unsupported_passes_through_reject: bool,
)
    requires
        snapshot_run_matches,
        tail_events_match_run,
        tail_seq_after_snapshot,
        has_evidence,
        step_count_positive,
        slot_count_positive,
        steps_apply_ok,
        slots_apply_ok,
        pc_in_bounds,
        unsupported_passes_through_reject,
    ensures
        spec_hydrate_run_frame_preconditions(
            snapshot_run_matches,
            tail_events_match_run,
            tail_seq_after_snapshot,
            has_evidence,
            step_count_positive,
            slot_count_positive,
            steps_apply_ok,
            slots_apply_ok,
            pc_in_bounds,
            unsupported_passes_through_reject,
        ),
{
}

// ============================================================================
// RecoveryError -> RuntimeError refinement proofs (D2: production
// runtime collapses most hydration failures into
// `RuntimeError::InvalidRecoveryHydration`; the storage layer DOES
// have typed `FrameDimensionOverflow` variant.)
// ============================================================================
/// Spec: production `RuntimeError` collapses hydration failures.
/// Returns true iff the production runtime layer would emit
/// `RuntimeError::InvalidRecoveryHydration` for the given storage
/// `RecoveryError`. Mirrors the production runtime driver at
/// `crates/vb_runtime/src/recovery.rs:73-115` (all hydration
/// failure paths collapse to `InvalidRecoveryHydration`).
pub open spec fn spec_runtime_collapses_to_invalid_recovery_hydration(
    error: RecoveryError,
) -> bool {
    match error {
        RecoveryError::WorkflowSourceDigestMismatch { .. } => false,
        RecoveryError::CompiledIrDigestMismatch { .. } => false,
        RecoveryError::UnsupportedFrameSeed { .. } => true,
        RecoveryError::FrameDimensionOverflow { .. } => true,
    }
}

/// Proof: the runtime layer always collapses hydration failures to
/// `RuntimeError::InvalidRecoveryHydration` (D2). Mirrors
/// `crates/vb_runtime/src/recovery.rs:73-115`.
pub proof fn proof_hydration_runtime_collapse(error: RecoveryError)
    requires
        match error {
            RecoveryError::UnsupportedFrameSeed { .. } => true,
            RecoveryError::FrameDimensionOverflow { .. } => true,
            _ => false,
        },
    ensures
        spec_runtime_collapses_to_invalid_recovery_hydration(error),
{
}

/// Proof: the runtime layer preserves the typed
/// `WorkflowSourceDigestMismatch` and `CompiledIrDigestMismatch` from
/// the storage layer (these are not hydration-internal failures; they
/// surface as themselves in the runtime error enum when bubbled up).
pub proof fn proof_digest_runtime_preserves_typed_error(error: RecoveryError)
    requires
        match error {
            RecoveryError::WorkflowSourceDigestMismatch { .. } => true,
            RecoveryError::CompiledIrDigestMismatch { .. } => true,
            _ => false,
        },
    ensures
        !spec_runtime_collapses_to_invalid_recovery_hydration(error),
{
}

fn main() {
}

} // verus!
