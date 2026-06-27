// SPDX-License-Identifier: MIT
//
// Extern surface for step_budget Verus spec.
//
// =============================================================================
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// This file is the production-binding surface for the `step_budget.rs`
// Verus spec. It includes the in-tree production mirror at
// `verification/verus/production_inner/signals_production.rs` via
// `#[path]` so that:
//
//   * The companion gate `scripts/check-verus-production-binding.sh`
//     classifies the spec file as WEAK-bound (spec uses
//     `#[path = "extern_step_budget.rs"]`; this file uses
//     `#[path = "production_inner/signals_production.rs"]`).
//   * Any drift in the production field names, discriminant sets, or
//     fn signatures breaks the
//     `production_inner/signals_production.rs` mirror and the spec
//     proofs that depend on it.
//
// The mirror at `production_inner/signals_production.rs` is a
// hand-written structural copy of the production surface in
// `crates/vb_core/src/engine/signals.rs`. The substitution relative to
// direct production `#[path]` inclusion is documented in the mirror's
// header: production's private `remaining` field is relaxed to `pub`
// in the mirror so Verus's `#[verifier::external_type_specification]`
// bridge can establish a transparent binding for spec-mode field
// access. Field NAME and TYPE are preserved byte-for-byte; any drift
// breaks the verification build.
//
// =============================================================================
// BINDING LEDGER
// =============================================================================
//   - `StepBudget`                          <- crates/vb_core/src/engine/signals.rs:13-16
//   - `StepBudget::MAX`                     <- crates/vb_core/src/engine/signals.rs:20-22
//   - `StepBudget::new`                     <- crates/vb_core/src/engine/signals.rs:27-35
//   - `StepBudget::try_take`                <- crates/vb_core/src/engine/signals.rs:50-60
//   - `StepBudget::remaining`               <- crates/vb_core/src/engine/signals.rs:64-66
//   - `EngineError::StepCounterOverflow`    <- crates/vb_core/src/errors.rs:241
//   - `MAX_STEP_BUDGET = 10_000`            <- crates/vb_core/src/limits.rs:94
//
// =============================================================================
// TRUST BOUNDARY
// =============================================================================
// The production bodies of `StepBudget::new`, `try_take`, `remaining`,
// and `MAX` are NOT verified by Verus directly (the mirror wraps the
// methods with `#[verifier::external]`, making their bodies opaque).
// The contracts attached via `assume_specification` in
// `step_budget.rs` state the production behavior the spec proofs
// discharge. Drift between the mirror and the production source is
// reported as binding-debt tracked outside Verus and is detected by
// the `scripts/check-production-inner-drift.sh` gate.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

// ---------------------------------------------------------------------------
// MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// `#[path]` inclusion of the in-tree production mirror
// `verification/verus/production_inner/signals_production.rs`. The
// mirror is a verbatim copy of `crates/vb_core/src/engine/signals.rs`
// with the only relaxation being `StepBudget::remaining` visibility
// (production: private; mirror: `pub`) so Verus's
// `external_type_specification` bridge in the companion spec file can
// establish a transparent binding for spec-mode field access.
//
// The mirror's impl methods are wrapped with `#[verifier::external]`
// inside the mirror so Verus treats their bodies as opaque; the
// companion spec file `step_budget.rs` attaches spec contracts via
// `assume_specification`.
#[path = "production_inner/signals_production.rs"]
pub mod production_signals;

// Re-export the production types so the spec file can reference them
// via `crate::production::production_signals::StepBudget`.
pub use production_signals::{EngineError, EngineSignal, StepBudget};
