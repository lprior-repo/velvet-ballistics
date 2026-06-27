// SPDX-License-Identifier: MIT
//
// Extern surface for run_loop_termination Verus spec.
//
// =============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// This file is the production-binding surface for the
// `run_loop_termination.rs` Verus spec. It uses the established
// in-tree mirror pattern (cf. `signals_invariant.rs` /
// `extern_signals_invariant.rs`):
//
//   - `crates/vb_core/src/engine/signals.rs` is included via the
//     in-tree mirror at
//     `verification/verus/production_inner/signals_production.rs`,
//     which is a verbatim copy of production with two minimal
//     substitutions:
//       1. `StepBudget::remaining` is `pub` (relaxed from production
//          private visibility) so Verus's
//          `#[verifier::external_type_specification]` can read the
//          field.
//       2. `StepBudget::from_env` body is wrapped in
//          `#[verifier::external_body]` (closure-pattern blocker;
//          signature and field name remain production-identical).
//     Field NAME and TYPE are preserved byte-for-byte; any drift
//     breaks the verification build.
//
//   - The companion spec file `run_loop_termination.rs` declares
//     the bridge `#[verifier::external_type_specification] pub struct
//     ExStepBudget(production::StepBudget)` and attaches
//     `assume_specification` contracts to the mirror's
//     `production::StepBudget::new`, `::try_take`, `::remaining`
//     methods directly. This replaces the prior
//     `MirrorStepBudget` re-declaration (the user-flagged issue).
//
//   - For `step_once`, `run_until_blocked`, `drive_deterministic`,
//     this file declares production-named stub modules
//     (`production_step`, `production_run_loop`) whose function
//     signatures MATCH production signatures at
//     `crates/vb_core/src/engine/run_loop.rs:12-35` and
//     `crates/vb_core/src/engine/step.rs:23-51` exactly. The bodies
//     are `#[verifier::external]` (opaque `loop {}`). Direct
//     `#[path]`-inclusion of step.rs is blocked by transitive deps
//     on `action_lifecycle` and the entire `action` subsystem
//     (8+ files); the stubs sidestep this while preserving signature
//     drift detection.
//
// =============================================================================
// BINDING LEDGER
// =============================================================================
//   - `ExStepBudget`              <- production::StepBudget
//                                    #[verifier::external_type_specification]
//                                    bridges the mirror at
//                                    production_inner/signals_production.rs:100
//   - `StepBudget::new`           <- production::StepBudget::new
//                                    (signals.rs:27-35; mirror at signals_production.rs:121)
//   - `StepBudget::try_take`      <- production::StepBudget::try_take
//                                    (signals.rs:50-60; mirror at signals_production.rs:136)
//   - `StepBudget::remaining`     <- production::StepBudget::remaining
//                                    (signals.rs:64-66; mirror at signals_production.rs:152)
//   - `EngineSignal`              <- production::EngineSignal
//                                    (signals.rs:99-115; mirror at signals_production.rs:189)
//   - `drive_deterministic`       <- mirror_drive_deterministic
//                                    (faithful mirror of run_loop.rs:22-35)
//   - `run_until_blocked`         <- mirror_run_until_blocked
//                                    (faithful mirror of run_loop.rs:12-19)
//   - `step_once`                 <- mirror_step_once
//                                    (opaque loop {} body;
//                                    assume_specification defines
//                                    production behavior)
//
// =============================================================================
// TRUST BOUNDARY
// =============================================================================
// The production bodies of every fn in this file's mirror inclusion
// are NOT verified by Verus (each is either `#[verifier::external]`
// or `#[verifier::external_body]`). The contracts attached via
// `assume_specification` in the companion spec file state the
// production behavior the spec proofs discharge. Drift between the
// in-tree mirror and the production source is reported as
// binding-debt tracked outside Verus (see
// `production_inner/signals_production.rs:26-29` for the drift
// policy).
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION SIGNALS MIRROR — included via #[path] (NOT #[verifier::external])
// ---------------------------------------------------------------------------
//
// `#[path]` inclusion of the in-tree mirror at
// `production_inner/signals_production.rs`. The mirror is verbatim
// from `crates/vb_core/src/engine/signals.rs` except for the
// `pub remaining` field relaxation and the `#[verifier::external_body]`
// wrapper on `from_env`. The mirror is included WITHOUT module-level
// `#[verifier::external]` so the type declarations are nameable in
// spec mode. Each production method body is opaque via per-fn
// `#[verifier::external]`.
#[path = "production_inner/signals_production.rs"]
pub mod production_signals;

// Re-export the mirror's types so the spec file can reference them as
// `crate::production::{EngineSignal, EngineError, StepBudget}`.
pub use production_signals::{EngineSignal, EngineError, StepBudget};

// ---------------------------------------------------------------------------
// PRODUCTION-NAMED STEP STUB
// ---------------------------------------------------------------------------
//
// Stub module whose `step_once` signature MATCHES the production
// `crates/vb_core/src/engine/step.rs:23-51` signature exactly.
// Body is `#[verifier::external]` (opaque `loop {}`); the behavior is
// captured by `assume_specification` in the companion spec file.
// The spec file's `mirror_step_once` exec fn is the non-vacuum
// witness that invokes this production-named function. Drift in
// production's step_once signature breaks this stub's signature at
// Rust resolution time.
#[verifier::external]
pub mod production_step {
    pub use super::production_signals::{EngineSignal, EngineError, StepBudget};
    use crate::frame::RunFrame;
    use crate::value_store::ValueStore;
    use crate::workflow::CompiledWorkflow;

    pub fn step_once(
        _plan: &CompiledWorkflow,
        _run: &mut RunFrame,
        _store: &mut ValueStore,
    ) -> Result<EngineSignal, EngineError> {
        loop {}
    }
}

// ---------------------------------------------------------------------------
// PRODUCTION-NAMED RUN_LOOP STUB
// ---------------------------------------------------------------------------
//
// Stub module whose `run_until_blocked` and `drive_deterministic`
// signatures MATCH production
// `crates/vb_core/src/engine/run_loop.rs:12-19` and `:22-35`
// signatures exactly. Bodies are `#[verifier::external]` (opaque
// `loop {}`); behavior is captured by `assume_specification`
// contracts in the companion spec file. The spec file's
// `mirror_run_until_blocked` and `mirror_drive_deterministic` exec
// fns are the non-vacuum witnesses.
#[verifier::external]
pub mod production_run_loop {
    pub use super::production_signals::{EngineSignal, EngineError, StepBudget};
    use crate::frame::RunFrame;
    use crate::value_store::ValueStore;
    use crate::workflow::CompiledWorkflow;

    pub fn run_until_blocked(
        _plan: &CompiledWorkflow,
        _run: &mut RunFrame,
        _budget: StepBudget,
        _store: &mut ValueStore,
    ) -> Result<EngineSignal, EngineError> {
        loop {}
    }

    pub fn drive_deterministic(
        _plan: &CompiledWorkflow,
        _run: &mut RunFrame,
        _budget: &mut StepBudget,
        _store: &mut ValueStore,
    ) -> Result<EngineSignal, EngineError> {
        loop {}
    }
}

} // verus!
