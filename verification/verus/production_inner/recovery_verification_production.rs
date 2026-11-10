// SPDX-License-Identifier: MIT
//
// ============================================================================
// Drift-detection stub for recovery_verification
// ============================================================================
//
// This file is a minimal drift-detection stub for the
// `recovery_verification` Verus spec. It exists so the companion
// `verification/verus/extern_recovery_verification.rs` can include
// this file via
// `#[path = "production_inner/recovery_verification_production.rs"]`
// to satisfy the production-binding gate.
//
// The actual production mirror content (UnsupportedRecoveryState,
// RecoveredStepState, RecoveryRuntimeSummary, DigestCheck, all the
// pure decision fns, etc.) lives in
// `verification/verus/extern_recovery_verification.rs`, which carries
// verbatim copies of the production source at:
//
//   - `UnsupportedRecoveryState`         <- crates/vb_storage/src/recovery/types/unsupported.rs:5-14
//   - `RecoveryCannotResumeState`        <- crates/vb_storage/src/recovery/types/cannot_resume.rs:14-41
//   - `RecoveredStepState`               <- crates/vb_storage/src/recovery/types/recovered.rs:8-19
//   - `RecoveredStepEntry`               <- crates/vb_storage/src/recovery/types/recovered.rs:23-28
//   - `RecoveredSlotEntry`               <- crates/vb_storage/src/recovery/types/recovered.rs:32-39
//   - `RecoveredPendingAction`           <- crates/vb_storage/src/recovery/types/recovered.rs:43-48
//   - `RecoveryTerminalState`            <- crates/vb_storage/src/recovery/types/runtime.rs:6-21
//   - `RecoveryRuntimeSummary`           <- crates/vb_storage/src/recovery/types/runtime.rs:23-48
//   - `RecoveryHydration`                <- crates/vb_storage/src/recovery/types/hydration.rs:9-40
//   - `RecoveryFrameSeedProduct`         <- crates/vb_storage/src/recovery/types/hydration.rs:55-60
// Production `crates/vb_storage/src/recovery/types/hydration.rs:55-60`
//   - `DigestPair` / ActionAbiDigest..   <- crates/vb_storage/src/recovery/types/digests.rs:7-147
//   - `DigestCheck`                      <- crates/vb_storage/src/recovery/types/digest_check.rs:6-13
//   - Decision fns                       <- crates/vb_storage/src/recovery/recover.rs:32-187
//                                            + crates/vb_storage/src/recovery/hydrate/mod.rs:86-108
//                                            + crates/vb_runtime/src/recovery/product.rs:36-41
//                                            + crates/vb_runtime/src/recovery.rs:159-161
//
// This stub mirrors the production `UnsupportedRecoveryState` field
// set as the smallest drift-detection surface.
//
// DRIFT POLICY: `crates/vb_storage/src/recovery/types/hydration.rs:42-135`
// Production source coverage:
//   - `UnsupportedRecoveryState`         <- crates/vb_storage/src/recovery/types/unsupported.rs:5-14
//   - `RecoveryCannotResumeState`        <- crates/vb_storage/src/recovery/types/cannot_resume.rs:14-41
//   - `RecoveredStepState`               <- crates/vb_storage/src/recovery/types/recovered.rs:8-19
//   - `RecoveredStepEntry`               <- crates/vb_storage/src/recovery/types/recovered.rs:23-28
//   - `RecoveredSlotEntry`               <- crates/vb_storage/src/recovery/types/recovered.rs:32-39
//   - `RecoveredPendingAction`           <- crates/vb_storage/src/recovery/types/recovered.rs:43-48
//   - `RecoveryTerminalState`            <- crates/vb_storage/src/recovery/types/runtime.rs:6-21
//   - `RecoveryRuntimeSummary`           <- crates/vb_storage/src/recovery/types/runtime.rs:23-48
//   - `RecoveryHydration`                <- crates/vb_storage/src/recovery/types/hydration.rs:9-40
//   - `RecoveryFrameSeedProduct`         <- crates/vb_storage/src/recovery/types/hydration.rs:42-135
//   - `DigestPair` / ActionAbiDigest..   <- crates/vb_storage/src/recovery/types/digests.rs:7-147
//   - `DigestCheck`                      <- crates/vb_storage/src/recovery/types/digest_check.rs:6-13
//   - Decision fns                       <- crates/vb_storage/src/recovery/recover.rs:32-187
//                                            + crates/vb_storage/src/recovery/hydrate/mod.rs:86-108
//                                            + crates/vb_runtime/src/recovery/product.rs:36-41
//                                            + crates/vb_runtime/src/recovery.rs:159-161
// Regenerate this file whenever production changes. Any rename of
// `slot_values`, `slot_taint`, `action_payloads`, or `pending_actions`
// breaks the `extern_recovery_verification` Verus build.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Drift-detection stubs
// ---------------------------------------------------------------------------
/// Mirror of production `UnsupportedRecoveryState` field shape at
/// `crates/vb_storage/src/recovery/types/unsupported.rs:5-14`. The stub is a
/// struct with the SAME field names and types as production so any
/// rename breaks the build.
#[derive(Clone, Copy)]
pub struct UnsupportedRecoveryStateStub {
    /// Mirror of production `slot_values: bool` at types/unsupported.rs:7.
    pub slot_values: bool,
    /// Mirror of production `slot_taint: bool` at types/unsupported.rs:9.
    pub slot_taint: bool,
    /// Mirror of production `action_payloads: bool` at types/unsupported.rs:11.
    pub action_payloads: bool,
    /// Mirror of production `pending_actions: bool` at types/unsupported.rs:13.
    pub pending_actions: bool,
}

impl UnsupportedRecoveryStateStub {
    /// Mirror of production `is_fully_supported` decision at
    /// `crates/vb_storage/src/recovery/types/unsupported.rs:63-65`. Body is
    /// `#[verifier::external]` (opaque).
    #[verifier::external]
    pub fn is_fully_supported_stub(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads && !self.pending_actions
    }
}

/// Mirror of production `RecoveryCannotResumeState` field shape at
/// `crates/vb_storage/src/recovery/types/cannot_resume.rs:14-41`.
#[derive(Clone, Copy)]
pub struct RecoveryCannotResumeStateStub {
    pub slot_values: bool,
    pub slot_taint: bool,
    pub action_payloads: bool,
    pub pending_actions: bool,
    pub pending_timers: bool,
    pub pending_asks: bool,
    pub workflow_missing: bool,
    pub store_missing: bool,
    pub action_attempts_missing: bool,
    pub admission_missing: bool,
    pub collect_states_missing: bool,
    pub action_contracts_missing: bool,
    pub action_abi_digests_missing: bool,
}

impl RecoveryCannotResumeStateStub {
    /// Mirror of production `RecoveryCannotResumeState::is_resumable`
    /// decision at `crates/vb_storage/src/recovery/types/cannot_resume.rs:167-181`.
    #[verifier::external]
    pub fn is_resumable_stub(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads && !self.pending_actions
            && !self.pending_timers && !self.pending_asks && !self.workflow_missing
            && !self.store_missing && !self.action_attempts_missing && !self.admission_missing
            && !self.collect_states_missing && !self.action_contracts_missing
            && !self.action_abi_digests_missing
    }
}

/// Minimal mirror of production `RecoveryFrameSeed` identity used by
/// the product split at `crates/vb_storage/src/recovery/types/hydration.rs:42-135`.
#[derive(Clone, Copy)]
pub struct RecoveryFrameSeed;

/// Mirror of production `NonResumableRecoveryFrameSeedProduct`.
#[derive(Clone, Copy)]
pub struct NonResumableRecoveryFrameSeedProduct {
    pub seed: RecoveryFrameSeed,
    pub cannot_resume: RecoveryCannotResumeStateStub,
}

/// Mirror of production `ResumableRecoveryFrameSeedProduct`.
#[derive(Clone, Copy)]
pub struct ResumableRecoveryFrameSeedProduct {
    pub seed: RecoveryFrameSeed,
}

/// Mirror of the storage typestate split on `RecoveryFrameSeedProduct`.
#[derive(Clone, Copy)]
pub enum RecoveryFrameSeedProduct {
    CannotResume(NonResumableRecoveryFrameSeedProduct),
    Resumable(ResumableRecoveryFrameSeedProduct),
}

/// Mirror of production `reject_unsupported_live_frame_state` decision
/// at `crates/vb_runtime/src/recovery.rs:109-115`. Production body checks
/// `seed.cannot_resume_state().is_resumable()`. Body is `#[verifier::external]`.
#[verifier::external]
pub fn reject_unsupported_stub(state: RecoveryCannotResumeStateStub) -> bool {
    state.is_resumable_stub()
}

} // verus!
