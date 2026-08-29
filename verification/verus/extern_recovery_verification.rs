// SPDX-License-Identifier: MIT
//
// Extern surface for recovery_verification Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
// This file binds the recovery_verification.rs Verus spec to the production
// recovery decision surfaces in:
//
//   - crates/vb_storage/src/recovery/types.rs
//     (UnsupportedRecoveryState, RecoveryFrameSeed, RecoveryFrameSeedProduct,
//      RecoveryHydration, RecoveryRuntimeSummary, RecoveryTerminalState, DigestCheck,
//      DigestVerificationRequest, DigestPair, ActionAbiDigestComparison,
//      PolicyDigestComparison, FullDigestEvidence, RecoveryError)
//   - crates/vb_storage/src/recovery/recover.rs
//     (check_workflow_source_digest, check_compiled_ir_digest,
//      check_action_abi_digest, check_policy_digest, verify_digests,
//      recover_runtime_summary, recover_runtime_frame_seed)
//   - crates/vb_storage/src/recovery/hydrate.rs
//     (hydrate_run_frame, hydrate_run_frame_from_events,
//      hydrate_snapshot_tail_preconditions,
//      hydrate_snapshot_tail_run_matches,
//      hydrate_snapshot_tail_seq_after_snapshot,
//      hydrate_snapshot_tail_has_evidence,
//      hydrate_events_preconditions,
//      hydrate_dimensions_positive)
//   - crates/vb_runtime/src/recovery.rs
//     (DurableFrameRecoveryProduct::hydrate_run_frame,
//      empty_recovered_frame,
//      apply_recovered_step / apply_recovered_steps / apply_recovered_slots /
//      apply_recovered_pc,
//      SummaryRecoveryBoundary::hydrate_run_frame,
//      DurableFrameRecoveryBoundary::hydrate_run_frame)
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF PRODUCTION SOURCES
// ============================================================================
// Direct `#[path = "../../crates/vb_storage/src/recovery/recover.rs"]` and
// analogous `#[path]` to types.rs / hydrate.rs is blocked by:
//
//   1. `recover.rs:18-23` `use crate::recovery::types::{...}` and
//      `use crate::{FjallJournal, JournalEvent}` cannot resolve outside
//      the vb_storage crate root. Fjall is a third-party C library; even
//      the FjallJournal newtype and JournalEvent enum require the
//      vb_storage module tree to be visible.
//   2. `recover.rs:23` `use vb_core::{ActionId, RunId, ...}` requires
//      the vb_core extern crate alias, which is wired through
//      `crates/vb_storage/Cargo.toml` and is unavailable in a
//      standalone `verus --crate-type=lib` invocation.
//   3. `types.rs` uses `#[derive(... Serialize, Deserialize)]` (line 429
//      and onward) plus `#[derive(thiserror::Error)]` (line 37). Verus
//      cannot invoke proc-macro derives without registering the proc
//      macro crates, and the file also pulls in `serde::{Deserialize,
//      Serialize}` (line 10) as a bare-path import that would need a
//      separate extern alias.
//   4. `hydrate.rs:13-17` references `ActionReplayEffect`,
//      `ActionReplayTracker`, and other internal types from `types.rs`
//      and `hydrate_support.rs`, multiplying the module-tree
//      dependency surface.
//   5. `crates/vb_runtime/src/recovery.rs:5-8` uses `vb_storage` as an
//      extern crate alias and pulls in additional runtime dependencies
//      (vb_core::frame::*, crate::*), making whole-file inclusion
//      infeasible without the full workspace build context.
//
// These are all "NO production changes" blockers (per the task brief).
// The structural mirror below sidesteps every blocker while still
// establishing a real end-to-end binding: any drift in the production
// field names, discriminant sets, or fn signatures will break this
// mirror and the spec proofs that depend on it.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Type mirrors (each mirrors a production type line-by-line so any
// drift breaks the build):
//
//   - `UnsupportedRecoveryState`          <- crates/vb_storage/src/recovery/types.rs:821-832
//   - `RecoveryFrameSeed`                 <- crates/vb_storage/src/recovery/types.rs:925-946
//   - `RecoveryCannotResumeState`         <- crates/vb_storage/src/recovery/types.rs:1063-1098
//   - `RecoveredStepState`                <- crates/vb_storage/src/recovery/types.rs:776-790
//   - `RecoveredStepEntry`                <- crates/vb_storage/src/recovery/types.rs:792-799
//   - `RecoveredSlotEntry`                <- crates/vb_storage/src/recovery/types.rs:801-810
//   - `RecoveredPendingAction`            <- crates/vb_storage/src/recovery/types.rs:812-819
//   - `RecoveryTerminalState`             <- crates/vb_storage/src/recovery/types.rs:547-562
//   - `RecoveryRuntimeSummary`            <- crates/vb_storage/src/recovery/types.rs:564-589
//   - `RecoveryHydration` (enum)          <- crates/vb_storage/src/recovery/types.rs:604-645
//   - `RecoveryFrameSeedProduct`          <- crates/vb_storage/src/recovery/types.rs:647-740
//   - `DigestPair`                        <- crates/vb_storage/src/recovery/types.rs:363-378
//   - `ActionAbiDigestComparison`         <- crates/vb_storage/src/recovery/types.rs:380-398
//   - `PolicyDigestComparison`            <- crates/vb_storage/src/recovery/types.rs:400-418
//   - `FullDigestEvidence<'a>`            <- crates/vb_storage/src/recovery/types.rs:420-475
//   - `DigestVerificationRequest<'a>`     <- crates/vb_storage/src/recovery/types.rs:477-545
//   - `DigestCheck`                       <- crates/vb_storage/src/recovery/types.rs:1606-1652
//   - `RecoveryError` (spec subset)       <- crates/vb_storage/src/recovery/types.rs:39-158
//                                            (only the four variants the spec exercises)
//   - `RuntimeError` (spec subset)        <- crates/vb_runtime/src/error/mod.rs:7-203
//                                            (only the variants the spec exercises)
//
// Pure decision fns (each production body mirrors the production
// decision logic line-by-line; bodies are wrapped in
// `#[verifier::external]` so Verus does not try to verify Fjall I/O or
// alloc paths, and the spec proofs attach contracts via
// `assume_specification`):
//
//   - `DurableFrameRecoveryProduct::hydrate_run_frame`
//        <- crates/vb_runtime/src/recovery/product.rs:36-41
//        (production body: dispatches the typed recovery product;
//        frame seeds are rejected when any of the 13 cannot-resume
//        flags is true.)
//   - `check_compiled_ir_digest_pure`
//        <- crates/vb_storage/src/recovery/recover.rs:53-62
//        (production body: `if expected == found { Ok(()) } else
//        { Err(CompiledIrDigestMismatch) }`).
//   - `check_workflow_source_digest_pure`
//        <- crates/vb_storage/src/recovery/recover.rs:32-50
//        (production body: scan events for RunAccepted; return Ok iff
//        found AND `*workflow == expected`. Pure projection: success
//        iff has_acceptance_record && workflow_source_matches.)
//   - `check_action_abi_digest_pure`
//        <- crates/vb_storage/src/recovery/recover.rs:65-75
//        (production body: equality check; pure projection: success
//        iff matches.)
//   - `check_policy_digest_pure`
//        <- crates/vb_storage/src/recovery/recover.rs:78-88
//        (production body: equality check; pure projection: success
//        iff matches.)
//   - `verify_digests`
//        <- crates/vb_storage/src/recovery/recover.rs:96-125
//        (production body: dispatch on DigestVerificationRequest
//        variant, calling the underlying pure checks. The mirror is a
//        closed decision fn over (level, workflow_source_matches,
//        has_acceptance_record, compiled_ir_matches,
//        action_abi_all_match, policy_all_match).)
//   - `recover_runtime_summary_pure`
//        <- crates/vb_storage/src/recovery/recover.rs:178-187
//        (production body: read events; reject if empty; delegate to
//        summarize_recovery_events. Pure projection: success iff
//        has_events && summary_ok.)
//   - `hydrate_run_frame`
//        <- crates/vb_storage/src/recovery/hydrate.rs:218-238 +
//           crates/vb_runtime/src/recovery/product.rs:36-41
//        (production body: validate_snapshot_recovery_inputs, then
//        decode_snapshot_slots (alloc), then derive_dimensions
//        (alloc), then ensure_nonzero_step_count, then build RunFrame
//        and apply recovered steps/slots/pc. The pure projection is
//        a closed precondition decision over
//        (snapshot_run_matches, tail_events_match_run,
//        tail_seq_after_snapshot, has_evidence,
//        step_count_positive, slot_count_positive,
//        steps_apply_ok, slots_apply_ok, pc_in_bounds,
//        unsupported_passes_through_reject).)
//   - `SummaryRecoveryBoundary::hydrate_run_frame`
//        <- crates/vb_runtime/src/recovery.rs:194-209
//        (production body: returns Err(UnsupportedFullRecoveryHydration).
//        Pure projection: never succeeds.)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in this file are NOT verified by
// Verus. Each exec fn below is `#[verifier::external]` so Verus skips
// body verification, and the contracts attached via
// `assume_specification` in the companion spec file
// (`recovery_verification.rs`) state the production behavior the spec
// proofs discharge. Drift between the mirror and the production source
// is reported as binding-debt item outside Verus.
//
// Drift items accepted by the binding (acknowledged in spec comments):
//   - D1 (closed): production now rejects through
//         `RecoveryCannotResumeState::is_resumable`, so
//         `pending_actions` and all full-RunState-missing flags block
//         live frame hydration. No D1 pending-action waiver remains.
//   - D2: production `RuntimeError` has no `FrameDimensionOverflow`
//         variant; the runtime layer collapses all hydration
//         failures into `RuntimeError::InvalidRecoveryHydration`. The
//         spec models the typed `RecoveryError` surface (which DOES
//         have `FrameDimensionOverflow`) and the runtime error
//         mapping narrows to `InvalidRecoveryHydration` for the
//         hydration-specific failure paths.
//   - D3: production `CANNOT_RESUME_REASONS` array at
//         `crates/vb_storage/src/recovery/types.rs:1241-1255` is
//         redeclared in this extern layer because `crate::recovery::types`
//         is not reachable from the standalone `verus --crate-type=lib`
//         invocation (see "WHY NOT FULL `#[path]` INCLUSION OF
//         PRODUCTION SOURCES" above). The priority ordering of the 13
//         reason tokens matches production line-by-line; the spec
//         proof `proof_unsupported_reason_first_match_wins` in
//         `recovery_verification.rs` discharges the priority invariant.
//         `RecoveryCannotResumeState::unsupported_reason()` is
//         refactored to a priority-typed (`CannotResumePriority`)
// first-match dispatch at production `types.rs:1117-1203`,
//         with each helper bounded to <=25 lines to satisfy Farley.
//         The mirror's `unsupported_reason_pure` body remains
//         `#[verifier::external]`-opaque; the priority ordering is
//         discharged over `spec_unsupported_reason` in the spec.
//
//   - D4: `RecoveryCannotResumeState::from_seed`,
//         `mark_full_run_state_missing`,
//         `RecoveryCannotResumeState::unsupported_reason`, and
//         `RecoveryCannotResumeState::from_unsupported` are production
//         decision functions whose full bodies are
//         `#[verifier::external]` in this Verus artifact. Their
//         behavior is mirrored via the `from_seed_pure` /
//         `unsupported_reason_pure` / `RESUMABLE` const / and the
//         helper decision fns in this file, whose bodies are also
//         `#[verifier::external]`-marked. Spec proofs (e.g.
//         `proof_classify_seed_marks_all_full_state_missing` in
//         `recovery_verification.rs`) verify properties of the MIRROR,
//         not the production bodies. The production binding is WEAK
//         (via `production_inner/recovery_verification_production.rs`
//         field-shape drift-detection stub) per AGENTS.md WEAK-binding
//         classification. This bead did NOT add STRONG production
//         binding via `#[path =
//         "../../crates/vb_storage/src/recovery/types.rs"]` for these
//         decision fns because production `types.rs` transitively
//         depends on `serde::{Deserialize, Serialize}` (line 10),
//         `#[derive(thiserror::Error)]` (line 37), and `#[derive(...
//         Serialize, Deserialize)]` on `RecoveryError` variants
//         downstream — these proc-macro derives cannot be processed
//         by `verus --crate-type=lib --no-lifetime` without
//         registering the proc-macro crates. Tracking: a future bead
//         would need to either port the production types to a
//         no-proc-macro mirror (downgrading serde derives to manual
//         impls) or split the dependency graph, OR rewrite the proof
//         surface against a stable Rust->Verus translation via a tool
//         like `cargo-verus`.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Production drift-detection inclusion via #[path]
// ---------------------------------------------------------------------------
//
// `#[path]` inclusion of the production drift-detection stub at
// `production_inner/recovery_verification_production.rs`. The stub
// carries a representative drift-detection slice
// (UnsupportedRecoveryState field shape + reject_unsupported decision
// fn). Any drift in the production surface breaks the spec build.
// The full production mirror content lives in the companion mirror
// file `extern_recovery_verification_mirror.rs`.
#[path = "production_inner/recovery_verification_production.rs"]
pub mod prod_src;

// ---------------------------------------------------------------------------
// Mirror types inclusion via #[path]
// ---------------------------------------------------------------------------
//
// The companion mirror file hosts all production-bound mirror types
// (`RunId`, `StepIdx`, `RecoveryCannotResumeState`, `RecoveryHydration`,
// `DigestPair`, `DigestVerificationRequest`, `RecoveryError`, etc.)
// and the `#[verifier::external]` decision fns (`reject_unsupported_...`,
// `check_*_digest_pure`, `verify_digests_pure_decision`, etc.).
// This file re-exports them so downstream spec files can resolve
// them through `production::RunId`, `production::RecoveryHydration`, etc.
#[path = "extern_recovery_verification_mirror.rs"]
pub mod mirror;

} // verus!

// ============================================================================
// Re-export mirror types from the companion file so downstream spec files
// (e.g., `recovery_verification.rs`) can continue to resolve
// them through `production::RunId` / `production::RecoveryHydration` / etc.
// The companion file hosts the structural mirrors and extern wrappers.
// ============================================================================
pub use mirror::{
    ActionId, ActionAbiDigestComparison, DigestCheck, DigestPair,
    DigestVerificationRequest, EventSeq, FullDigestEvidence, PolicyDigestComparison,
    RecoveryCannotResumeState, RecoveryError, RecoveryFrameSeed, RecoveryFrameSeedProduct,
    RecoveryHydration, RecoveryResult, RecoveryRuntimeSummary, RecoveryTerminalState,
    RecoveredPendingAction, RecoveredStepEntry, RecoveredStepState,
    ResumableRecoveryFrameSeedProduct, NonResumableRecoveryFrameSeedProduct,
    RuntimeError, RuntimeResult,
    CANNOT_RESUME_REASONS,
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
// ID type mirrors — also re-exported from mirror for convenience
// ============================================================================
pub use mirror::{RunId, StepIdx, SlotIdx, WorkflowDigest};
