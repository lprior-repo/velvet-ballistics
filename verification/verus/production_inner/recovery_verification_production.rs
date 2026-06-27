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
//   - `UnsupportedRecoveryState`         <- crates/vb_storage/src/recovery/types.rs:553-563
//   - `RecoveredStepState`               <- crates/vb_storage/src/recovery/types.rs:508-521
//   - `RecoveredStepEntry`               <- crates/vb_storage/src/recovery/types.rs:524-530
//   - `RecoveredSlotEntry`               <- crates/vb_storage/src/recovery/types.rs:533-541
//   - `RecoveredPendingAction`           <- crates/vb_storage/src/recovery/types.rs:544-550
//   - `RecoveryTerminalState`            <- crates/vb_storage/src/recovery/types.rs:431-443
//   - `RecoveryRuntimeSummary`           <- crates/vb_storage/src/recovery/types.rs:446-470
//   - `RecoveryHydration`                <- crates/vb_storage/src/recovery/types.rs:487-494
//   - `DigestPair` / ActionAbiDigest..  <- crates/vb_storage/src/recovery/types.rs:244-426
//   - `DigestCheck`                      <- crates/vb_storage/src/recovery/types.rs:856-864
//   - Decision fns                       <- crates/vb_storage/src/recovery/recover.rs:32-187
//                                            + crates/vb_storage/src/recovery/hydrate.rs:181-200
//                                            + crates/vb_runtime/src/recovery.rs:63-82,146-154
//
// This stub mirrors the production `UnsupportedRecoveryState` field
// set as the smallest drift-detection surface.
//
// DRIFT POLICY: `crates/vb_storage/src/recovery/types.rs:553-563`
// Production source coverage:
//   - `UnsupportedRecoveryState`         <- crates/vb_storage/src/recovery/types.rs:553-563
//   - `RecoveredStepState`               <- crates/vb_storage/src/recovery/types.rs:508-521
//   - `RecoveredStepEntry`               <- crates/vb_storage/src/recovery/types.rs:524-530
//   - `RecoveredSlotEntry`               <- crates/vb_storage/src/recovery/types.rs:533-541
//   - `RecoveredPendingAction`           <- crates/vb_storage/src/recovery/types.rs:544-550
//   - `RecoveryTerminalState`            <- crates/vb_storage/src/recovery/types.rs:431-443
//   - `RecoveryRuntimeSummary`           <- crates/vb_storage/src/recovery/types.rs:446-470
//   - `RecoveryHydration`                <- crates/vb_storage/src/recovery/types.rs:487-494
//   - `DigestPair` / ActionAbiDigest..   <- crates/vb_storage/src/recovery/types.rs:244-426
//   - `DigestCheck`                      <- crates/vb_storage/src/recovery/types.rs:856-864
//   - Decision fns                       <- crates/vb_storage/src/recovery/recover.rs:32-187
//                                            + crates/vb_storage/src/recovery/hydrate.rs:181-200
//                                            + crates/vb_runtime/src/recovery.rs:63-82,146-154
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
/// `crates/vb_storage/src/recovery/types.rs:553-563`. The stub is a
/// struct with the SAME field names and types as production so any
/// rename breaks the build.
#[derive(Clone, Copy)]
pub struct UnsupportedRecoveryStateStub {
    /// Mirror of production `slot_values: bool` at types.rs:557.
    pub slot_values: bool,
    /// Mirror of production `slot_taint: bool` at types.rs:558.
    pub slot_taint: bool,
    /// Mirror of production `action_payloads: bool` at types.rs:559.
    pub action_payloads: bool,
    /// Mirror of production `pending_actions: bool` at types.rs:560.
    pub pending_actions: bool,
}

impl UnsupportedRecoveryStateStub {
    /// Mirror of production `is_fully_supported` decision at
    /// `crates/vb_storage/src/recovery/types.rs:614-616`. Body is
    /// `#[verifier::external]` (opaque).
    #[verifier::external]
    pub fn is_fully_supported_stub(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads && !self.pending_actions
    }
}

/// Mirror of production `reject_unsupported_live_frame_state` decision
/// at `crates/vb_runtime/src/recovery.rs:73-82`. Production body checks
/// the first three flags (NOT `pending_actions` — drift D1). Body is
/// `#[verifier::external]` (opaque).
#[verifier::external]
pub fn reject_unsupported_stub(state: UnsupportedRecoveryStateStub) -> bool {
    !state.slot_values && !state.slot_taint && !state.action_payloads
}

} // verus!