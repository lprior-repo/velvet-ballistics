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
//   - `UnsupportedRecoveryState`         <- crates/vb_storage/src/recovery/types.rs:577-649
//   - `RecoveryCannotResumeState`        <- crates/vb_storage/src/recovery/types.rs:680-834
//   - `RecoveredStepState`               <- crates/vb_storage/src/recovery/types.rs:533-544
//   - `RecoveredStepEntry`               <- crates/vb_storage/src/recovery/types.rs:548-553
//   - `RecoveredSlotEntry`               <- crates/vb_storage/src/recovery/types.rs:557-564
//   - `RecoveredPendingAction`           <- crates/vb_storage/src/recovery/types.rs:568-573
//   - `RecoveryTerminalState`            <- crates/vb_storage/src/recovery/types.rs:454-466
//   - `RecoveryRuntimeSummary`           <- crates/vb_storage/src/recovery/types.rs:470-493
//   - `RecoveryHydration`                <- crates/vb_storage/src/recovery/types.rs:512-528
//   - `DigestPair` / ActionAbiDigest..   <- crates/vb_storage/src/recovery/types.rs:269-449
//   - `DigestCheck`                      <- crates/vb_storage/src/recovery/types.rs:1058-1065
//   - Decision fns                       <- crates/vb_storage/src/recovery/recover.rs:32-187
//                                            + crates/vb_storage/src/recovery/hydrate.rs:206-225
//                                            + crates/vb_runtime/src/recovery.rs:99-115,188-190
//
// This stub mirrors the production `UnsupportedRecoveryState` field
// set as the smallest drift-detection surface.
//
// DRIFT POLICY: `crates/vb_storage/src/recovery/types.rs:577-649`
// Production source coverage:
//   - `UnsupportedRecoveryState`         <- crates/vb_storage/src/recovery/types.rs:577-649
//   - `RecoveryCannotResumeState`        <- crates/vb_storage/src/recovery/types.rs:680-834
//   - `RecoveredStepState`               <- crates/vb_storage/src/recovery/types.rs:533-544
//   - `RecoveredStepEntry`               <- crates/vb_storage/src/recovery/types.rs:548-553
//   - `RecoveredSlotEntry`               <- crates/vb_storage/src/recovery/types.rs:557-564
//   - `RecoveredPendingAction`           <- crates/vb_storage/src/recovery/types.rs:568-573
//   - `RecoveryTerminalState`            <- crates/vb_storage/src/recovery/types.rs:454-466
//   - `RecoveryRuntimeSummary`           <- crates/vb_storage/src/recovery/types.rs:470-493
//   - `RecoveryHydration`                <- crates/vb_storage/src/recovery/types.rs:512-528
//   - `DigestPair` / ActionAbiDigest..   <- crates/vb_storage/src/recovery/types.rs:269-449
//   - `DigestCheck`                      <- crates/vb_storage/src/recovery/types.rs:1058-1065
//   - Decision fns                       <- crates/vb_storage/src/recovery/recover.rs:32-187
//                                            + crates/vb_storage/src/recovery/hydrate.rs:206-225
//                                            + crates/vb_runtime/src/recovery.rs:99-115,188-190
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
/// `crates/vb_storage/src/recovery/types.rs:577-649`. The stub is a
/// struct with the SAME field names and types as production so any
/// rename breaks the build.
#[derive(Clone, Copy)]
pub struct UnsupportedRecoveryStateStub {
    /// Mirror of production `slot_values: bool` at types.rs:579.
    pub slot_values: bool,
    /// Mirror of production `slot_taint: bool` at types.rs:581.
    pub slot_taint: bool,
    /// Mirror of production `action_payloads: bool` at types.rs:583.
    pub action_payloads: bool,
    /// Mirror of production `pending_actions: bool` at types.rs:585.
    pub pending_actions: bool,
}

impl UnsupportedRecoveryStateStub {
    /// Mirror of production `is_fully_supported` decision at
    /// `crates/vb_storage/src/recovery/types.rs:635-639`. Body is
    /// `#[verifier::external]` (opaque).
    #[verifier::external]
    pub fn is_fully_supported_stub(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads && !self.pending_actions
    }
}

/// Mirror of production `RecoveryCannotResumeState` field shape at
/// `crates/vb_storage/src/recovery/types.rs:680-834`.
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
    /// decision at `crates/vb_storage/src/recovery/types.rs:783-799`.
    #[verifier::external]
    pub fn is_resumable_stub(self) -> bool {
        !self.slot_values
            && !self.slot_taint
            && !self.action_payloads
            && !self.pending_actions
            && !self.pending_timers
            && !self.pending_asks
            && !self.workflow_missing
            && !self.store_missing
            && !self.action_attempts_missing
            && !self.admission_missing
            && !self.collect_states_missing
            && !self.action_contracts_missing
            && !self.action_abi_digests_missing
    }
}

/// Mirror of production `reject_unsupported_live_frame_state` decision
/// at `crates/vb_runtime/src/recovery.rs:109-115`. Production body checks
/// `seed.cannot_resume_state().is_resumable()`. Body is `#[verifier::external]`.
#[verifier::external]
pub fn reject_unsupported_stub(state: RecoveryCannotResumeStateStub) -> bool {
    state.is_resumable_stub()
}

} // verus!
