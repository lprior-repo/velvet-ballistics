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
//                                            types.rs:577-649)
//   - `RecoveryCannotResumeState`        <- extern_recovery_verification.rs
//                                            (mirror of types.rs:680-834)
//   - `RecoveredStepState`              <- extern_recovery_verification.rs
//                                            (mirror of types.rs:533-544)
//   - `RecoveredStepEntry`              <- extern_recovery_verification.rs
//                                            (mirror of types.rs:548-553)
//   - `RecoveredSlotEntry`              <- extern_recovery_verification.rs
//                                            (mirror of types.rs:557-564)
//   - `RecoveredPendingAction`          <- extern_recovery_verification.rs
//                                            (mirror of types.rs:568-573)
//   - `RecoveryTerminalState`           <- extern_recovery_verification.rs
//                                            (mirror of types.rs:454-466)
//   - `RecoveryRuntimeSummary`          <- extern_recovery_verification.rs
//                                            (mirror of types.rs:470-493)
//   - `RecoveryFrameSeed`               <- extern_recovery_verification.rs
//                                            (mirror of types.rs:653-672)
//   - `RecoveryHydration`               <- extern_recovery_verification.rs
//                                            (mirror of types.rs:512-528)
//   - `DigestCheck`                     <- extern_recovery_verification.rs
//                                            (mirror of types.rs:1058-1065)
//   - `DigestVerificationRequest`       <- extern_recovery_verification.rs
//                                            (mirror of types.rs:383-449)
//   - `FullDigestEvidence`              <- extern_recovery_verification.rs
//                                            (mirror of types.rs:326-379)
//   - `DigestPair` / `ActionAbiDigestComparison`
//                                       <- extern_recovery_verification.rs
//                                            (mirror of types.rs:269-322)
//   - `RecoveryError` (spec subset)     <- extern_recovery_verification.rs
//                                            (mirror of types.rs:39-158,
//                                             4 variants exercised)
//   - `RuntimeError` (spec subset)      <- extern_recovery_verification.rs
//                                            (mirror of error/mod.rs:7-203,
//                                             5 variants exercised)
//
//   - `reject_unsupported_live_frame_state_pure`
//                                       <- extern_recovery_verification.rs
//                                            (mirror of
//                                            crates/vb_runtime/src/recovery.rs:109-115
//                                            `reject_unsupported_live_frame_state`;
//                                            production checks the 13-flag
//                                            cannot-resume witness)
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
//                                            crates/vb_storage/src/recovery/hydrate.rs:206-225
//                                            `hydrate_run_frame`
//                                            AND
//                                            crates/vb_runtime/src/recovery.rs:99-105
//                                            `DurableFrameRecoveryBoundary::hydrate_run_frame`)
//   - `summary_recovery_boundary_hydrate_pure`
//                                       <- extern_recovery_verification.rs
//                                            (mirror of
//                                            crates/vb_runtime/src/recovery.rs:188-190
//                                            `SummaryRecoveryBoundary::hydrate_run_frame`;
//                                            always returns
//                                            `UnsupportedFullRecoveryHydration`)
//
// ============================================================================
// DRIFT ITEMS ACCEPTED BY THE BINDING
// ============================================================================
//   - D1 (closed): production `reject_unsupported_live_frame_state`
//         now checks the 13-flag `RecoveryCannotResumeState` witness.
//         `pending_actions`, pending timers/asks, and every
//         full-RunState-missing flag block live frame hydration.
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
//   - D3: production `CANNOT_RESUME_REASONS` array at
//         `crates/vb_storage/src/recovery/types.rs:801-818` is
//         redeclared in the extern mirror
//         (`verification/verus/extern_recovery_verification.rs`).
//         The priority ordering of the 13 reason tokens matches
//         production line-by-line. `RecoveryCannotResumeState::unsupported_reason()`
//         is refactored to a priority-typed (`CannotResumePriority`)
//         first-match dispatch (helper `CannotResumePriority::first_match`
//         + `CannotResumePriority::reason`) at production
//         `types.rs:801-887`, with each helper bounded to <=25 lines
//         to satisfy Farley. The spec proof
//         `proof_unsupported_reason_first_match_wins` below
//         discharges the priority invariant: when a higher-priority
//         flag is true, the returned reason is the highest-priority
//         matching token, never a later-priority one.
//
//   - D4: `RecoveryCannotResumeState::from_seed`,
//         `mark_full_run_state_missing`,
//         `RecoveryCannotResumeState::unsupported_reason`, and
//         `RecoveryCannotResumeState::from_unsupported` are production
//         decision functions whose full bodies are
//         `#[verifier::external]` in this Verus artifact. Their
//         behavior is mirrored via the `from_seed_pure` /
//         `unsupported_reason_pure` / `from_unsupported_pure` exec
//         wrappers in
//         `verification/verus/extern_recovery_verification.rs`, whose
//         bodies are also `#[verifier::external]`-marked. Spec proofs
//         (e.g. `proof_classify_seed_marks_all_full_state_missing`)
//         verify properties of the MIRROR, not the production bodies.
//         The production binding is WEAK (via
//         `production_inner/recovery_verification_production.rs`
//         field-shape drift-detection stub) per AGENTS.md
//         WEAK-binding classification. This bead did NOT add STRONG
//         production binding via `#[path =
//         "../../crates/vb_storage/src/recovery/types.rs"]` for these
//         decision fns because production `types.rs` transitively
//         depends on `serde::{Deserialize, Serialize}` (line 10),
//         `#[derive(thiserror::Error)]` (line 37), and
//         `#[derive(... Serialize, Deserialize)]` on `RecoveryError`
//         variants downstream of `types.rs` — these proc-macro derives
//         cannot be processed by `verus --crate-type=lib
//         --no-lifetime` without registering the proc-macro crates.
//         Tracking: a future bead would need to either port the
//         production types to a no-proc-macro mirror (downgrading
//         serde derives to manual impls) or split the dependency
//         graph, OR rewrite the proof surface against a stable
//         Rust->Verus translation via a tool like `cargo-verus`.
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
    CANNOT_RESUME_REASONS,
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
    RecoveryCannotResumeState,
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
/// the production body returns Ok(()) after evaluating the typed
/// 13-flag cannot-resume witness.
pub open spec fn spec_reject_unsupported_passes(state: RecoveryCannotResumeState) -> bool {
    spec_cannot_resume_is_resumable(state)
}

/// Spec-side storage-only "fully supported" predicate. Returns true
/// iff none of the four storage-level unsupported flags are set. This
/// is narrower than live runtime resumability: a storage-supported
/// frame seed still fails closed when full `RunState` evidence is
/// absent.
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

/// Spec-side predicate mirroring
/// [`RecoveryCannotResumeState::is_resumable`] at
/// `crates/vb_storage/src/recovery/types.rs:785-799`. Returns true iff
/// every cannot-resume flag is false.
pub open spec fn spec_cannot_resume_is_resumable(state: RecoveryCannotResumeState) -> bool {
    !state.slot_values && !state.slot_taint && !state.action_payloads && !state.pending_actions
        && !state.pending_timers && !state.pending_asks && !state.workflow_missing
        && !state.store_missing && !state.action_attempts_missing && !state.admission_missing
        && !state.collect_states_missing && !state.action_contracts_missing
        && !state.action_abi_digests_missing
}

/// Spec-side decision fn mirroring `RecoveryCannotResumeState::from_seed`
/// at `crates/vb_storage/src/recovery/types.rs:748-757`. Returns the
/// priority-ordered lower-level reason for a non-resumable seed (the
/// first true non-`RESUMABLE` flag wins, matching the priority order
/// of the canonical reason priority list).
///
/// `Resumable` is intentionally not a [`RecoveryResumeStatus`] variant
/// (see the production runtime's
/// `RecoveryResumeStatus::CannotResume`); this fn returns
/// `"resumable"` only when every flag is false.
pub open spec fn spec_unsupported_reason(state: RecoveryCannotResumeState) -> &'static str {
    if state.slot_values {
        "slot_values"
    } else if state.slot_taint {
        "slot_taint"
    } else if state.action_payloads {
        "action_payloads"
    } else if state.pending_actions {
        "pending_actions"
    } else if state.pending_timers {
        "pending_timers"
    } else if state.pending_asks {
        "pending_asks"
    } else if state.workflow_missing {
        "workflow_missing"
    } else if state.store_missing {
        "store_missing"
    } else if state.action_attempts_missing {
        "action_attempts_missing"
    } else if state.admission_missing {
        "admission_missing"
    } else if state.collect_states_missing {
        "collect_states_missing"
    } else if state.action_contracts_missing {
        "action_contracts_missing"
    } else if state.action_abi_digests_missing {
        "action_abi_digests_missing"
    } else {
        "resumable"
    }
}

/// Spec-side decision fn mirroring the production `from_seed`
/// classification. Production
/// `RecoveryCannotResumeState::from_seed` always invokes
/// `mark_full_run_state_missing` which sets the 7 `*_missing`
/// flags to true (FINDING-001: a frame seed alone never carries the
/// full RunState). The spec projection captures this invariant by
/// returning a state where every `*_missing` flag is true.
pub open spec fn spec_classify_seed_cannot_resume(
    seed_supported_flags: RecoveryCannotResumeState,
) -> RecoveryCannotResumeState {
    RecoveryCannotResumeState {
        slot_values: seed_supported_flags.slot_values,
        slot_taint: seed_supported_flags.slot_taint,
        action_payloads: seed_supported_flags.action_payloads,
        pending_actions: seed_supported_flags.pending_actions,
        pending_timers: seed_supported_flags.pending_timers,
        pending_asks: seed_supported_flags.pending_asks,
        workflow_missing: true,
        store_missing: true,
        action_attempts_missing: true,
        admission_missing: true,
        collect_states_missing: true,
        action_contracts_missing: true,
        action_abi_digests_missing: true,
    }
}

// ============================================================================
// Production-bound exec fns (mirror production exec fns via the
// extern exec wrappers; bodies are `#[verifier::external]`)
// ============================================================================
/// Production-bound exec fn: `reject_unsupported_live_frame_state`
/// decision projection. Returns true iff the production body returns
/// Ok. Mirrors `crates/vb_runtime/src/recovery.rs:109-115`.
pub fn reject_unsupported_live_frame_state(state: RecoveryCannotResumeState) -> bool {
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
/// `crates/vb_storage/src/recovery/hydrate.rs:206-225` AND
/// `crates/vb_runtime/src/recovery.rs:99-105`.
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
/// `crates/vb_runtime/src/recovery.rs:188-190`.
pub fn summary_boundary_hydrate() -> bool {
    production::summary_recovery_boundary_hydrate_pure()
}

/// Production-bound exec fn: `RecoveryFrameSeed::cannot_resume_state`
/// decision projection. Returns the production
/// `RecoveryCannotResumeState` for the supplied seed. Mirrors
/// `crates/vb_storage/src/recovery/types.rs:836-848`.
pub fn cannot_resume_state(seed: RecoveryFrameSeed) -> RecoveryCannotResumeState {
    // The mirror exposes `RecoveryCannotResumeState::from_seed` as a
    // free helper (not a method) so the spec-side proof surface can
    // attach a contract via `assume_specification`. Mirrors the
    // production `from_seed(&RecoveryFrameSeed) -> Self` at
    // types.rs:748-757.
    RecoveryCannotResumeState::from_seed_pure(seed)
}

/// Production-bound exec fn: `RecoveryCannotResumeState::is_resumable`
/// decision projection. Returns true iff every cannot-resume flag
/// is false. Mirrors `crates/vb_storage/src/recovery/types.rs:783-799`.
pub fn cannot_resume_is_resumable(state: RecoveryCannotResumeState) -> bool {
    state.is_resumable()
}

/// Production-bound exec fn: `RecoveryCannotResumeState::unsupported_reason`
/// decision projection. Returns the priority-ordered canonical reason
/// string. Mirrors `crates/vb_storage/src/recovery/types.rs:801-832`.
pub fn unsupported_reason(state: RecoveryCannotResumeState) -> &'static str {
    state.unsupported_reason_pure()
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
/// iff the typed cannot-resume witness is fully resumable. Mirrors
/// production body at `crates/vb_runtime/src/recovery.rs:109-115`.
pub assume_specification[ production::reject_unsupported_live_frame_state_pure ](
    state: RecoveryCannotResumeState,
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
/// `crates/vb_storage/src/recovery/hydrate.rs:206-225` AND
/// `crates/vb_runtime/src/recovery.rs:99-115`.
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
/// `crates/vb_runtime/src/recovery.rs:188-190`.
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

/// Bridge contract: `RecoveryCannotResumeState::unsupported_reason`
/// returns the priority-ordered canonical reason string. Mirrors
/// production body at
/// `crates/vb_storage/src/recovery/types.rs:801-832`.
pub assume_specification[ <RecoveryCannotResumeState>::unsupported_reason_pure ](
    state: RecoveryCannotResumeState,
) -> (result: &'static str)
    ensures
        result == spec_unsupported_reason(state),
;

/// Bridge contract: `RecoveryCannotResumeState::from_seed` always
/// produces a state where the seven `*_missing` full-RunState flags
/// are true. Mirrors production body at
/// `crates/vb_storage/src/recovery/types.rs:748-757`.
pub assume_specification[ <RecoveryCannotResumeState>::from_seed_pure ](
    seed: RecoveryFrameSeed,
) -> (result: RecoveryCannotResumeState)
    ensures
        result == spec_classify_seed_cannot_resume(RecoveryCannotResumeState::RESUMABLE),
        result.workflow_missing,
        result.store_missing,
        result.action_attempts_missing,
        result.admission_missing,
        result.collect_states_missing,
        result.action_contracts_missing,
        result.action_abi_digests_missing,
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
/// `slot_taint` is unsupported in the 13-flag cannot-resume witness.
pub proof fn proof_reject_unsupported_slot_taint_alone(state: RecoveryCannotResumeState)
    requires
        state.slot_taint,
    ensures
        !spec_reject_unsupported_passes(state),
{
}

/// Proof: `reject_unsupported_live_frame_state` rejects when
/// `slot_values` is unsupported in the 13-flag cannot-resume witness.
pub proof fn proof_reject_unsupported_slot_values_alone(state: RecoveryCannotResumeState)
    requires
        state.slot_values,
    ensures
        !spec_reject_unsupported_passes(state),
{
}

/// Proof: `reject_unsupported_live_frame_state` rejects when
/// `action_payloads` is unsupported in the 13-flag cannot-resume witness.
pub proof fn proof_reject_unsupported_action_payloads_alone(state: RecoveryCannotResumeState)
    requires
        state.action_payloads,
    ensures
        !spec_reject_unsupported_passes(state),
{
}

/// Proof: `reject_unsupported_live_frame_state` rejects when
/// `pending_actions` is true in the 13-flag cannot-resume witness.
pub proof fn proof_pending_actions_unsupported_blocks_hydration(state: RecoveryCannotResumeState)
    requires
        state.pending_actions,
    ensures
        !spec_reject_unsupported_passes(state),
{
}

/// Proof: `reject_unsupported_live_frame_state` passes when all 13
/// cannot-resume flags are false.
pub proof fn proof_no_rejection_when_supported(state: RecoveryCannotResumeState)
    requires
        !state.slot_values,
        !state.slot_taint,
        !state.action_payloads,
        !state.pending_actions,
        !state.pending_timers,
        !state.pending_asks,
        !state.workflow_missing,
        !state.store_missing,
        !state.action_attempts_missing,
        !state.admission_missing,
        !state.collect_states_missing,
        !state.action_contracts_missing,
        !state.action_abi_digests_missing,
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

/// Proof: the production `RecoveryCannotResumeState::from_seed`
/// always produces a state where every `*_missing` full-RunState
/// flag is true. This is the type-system invariant that makes
/// `RecoveryResumeStatus::Resumable` structurally unreachable from a
/// frame seed (FINDING-001, BLOCKER 3). Mirrors production body at
/// `crates/vb_storage/src/recovery/types.rs:748-757`
/// (`mark_full_run_state_missing` at types.rs:759-767).
pub proof fn proof_classify_seed_marks_all_full_state_missing()
    ensures
        ({
            let result = spec_classify_seed_cannot_resume(RecoveryCannotResumeState::RESUMABLE);
            &&& result.workflow_missing
            &&& result.store_missing
            &&& result.action_attempts_missing
            &&& result.admission_missing
            &&& result.collect_states_missing
            &&& result.action_contracts_missing
            &&& result.action_abi_digests_missing
        }),
{
}

/// Proof: priority-ordering invariant of `unsupported_reason`. When
/// `slot_values` is true on the input state, the returned reason is
/// `"slot_values"` (the highest-priority token in
/// the canonical reason priority list).
pub proof fn proof_unsupported_reason_priority_slot_values(state: RecoveryCannotResumeState)
    requires
        state.slot_values,
    ensures
        spec_unsupported_reason(state) == "slot_values",
{
}

/// Proof: priority-ordering invariant of `unsupported_reason`. When
/// both `slot_values` AND a lower-priority flag (e.g.
/// `action_abi_digests_missing`) are true, the first-match-wins rule
/// still picks `"slot_values"` — never the lower-priority token.
/// Demonstrates that `unsupported_reason`'s const-table walk does not
/// stop at the first lower-priority match when an earlier-priority
/// match exists. Mirrors production `types.rs:820-855` refactor.
pub proof fn proof_unsupported_reason_first_match_wins(state: RecoveryCannotResumeState)
    requires
        state.slot_values,
        state.action_abi_digests_missing,
    ensures
        spec_unsupported_reason(state) == "slot_values",
{
}

/// Proof: priority-ordering invariant of `unsupported_reason`. When
/// only `workflow_missing` is true (the case every frame seed lands
/// in after FINDING-001), the returned reason is `"workflow_missing"`.
pub proof fn proof_unsupported_reason_workflow_only(state: RecoveryCannotResumeState)
    requires
        !state.slot_values,
        !state.slot_taint,
        !state.action_payloads,
        !state.pending_actions,
        !state.pending_timers,
        !state.pending_asks,
        state.workflow_missing,
    ensures
        spec_unsupported_reason(state) == "workflow_missing",
{
}

/// Proof: `is_resumable` returns false for the post-`from_seed`
/// classification. This is the canonical frame-seed invariant that
/// excludes `RecoveryResumeStatus::Resumable` from the typed
/// boundary.
pub proof fn proof_classify_seed_is_never_resumable()
    ensures
        !spec_cannot_resume_is_resumable(
            spec_classify_seed_cannot_resume(RecoveryCannotResumeState::RESUMABLE),
        ),
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
/// `reject_unsupported_live_frame_state` driver would reject. The
/// runtime driver now rejects whenever the 13-flag cannot-resume
/// witness is not fully resumable. Mirrors
/// `crates/vb_runtime/src/recovery.rs:109-115`.
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
