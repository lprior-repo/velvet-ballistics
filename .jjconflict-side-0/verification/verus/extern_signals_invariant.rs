// SPDX-License-Identifier: MIT
//
// Extern surface for signals_invariant Verus spec.
//
// =============================================================================
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// This file is the production-binding surface for the
// `signals_invariant.rs` Verus spec. It includes the in-tree
// production mirror at
// `verification/verus/production_inner/signals_production.rs` via
// `#[path]` so that:
//
//   * The companion gate `scripts/check-verus-production-binding.sh`
//     classifies the spec file as WEAK-bound (spec uses
//     `#[path = "extern_signals_invariant.rs"]`; this file uses
//     `#[path = "production_inner/signals_production.rs"]`).
//   * Any drift in the production field names, discriminant sets, or
//     fn signatures breaks the
//     `production_inner/signals_production.rs` mirror and the spec
//     proofs that depend on it.
//   * The `scripts/check-production-inner-drift.sh` gate validates
//     that every identifier claimed in the mirror's per-section
//     `// Production ...` annotations is present in the mirror body,
//     catching drift between the verbatim mirror and the
//     `crates/vb_core/src/engine/signals.rs` source.
//
// The mirror at `production_inner/signals_production.rs` is a
// hand-written structural copy of the production surface in
// `crates/vb_core/src/engine/signals.rs`. The substitution relative
// to direct production `#[path]` inclusion is documented in the
// mirror's header: production's private `remaining` field is relaxed
// to `pub` in the mirror so Verus's
// `#[verifier::external_type_specification]` bridge can establish a
// transparent binding for spec-mode field access. Field NAME and TYPE
// are preserved byte-for-byte; any drift breaks the verification
// build.
//
// =============================================================================
// BINDING LEDGER
// =============================================================================
//   - `StepBudget`                          <- crates/vb_core/src/engine/signals.rs:13-16
//   - `StepBudget::MAX`                     <- crates/vb_core/src/engine/signals.rs:19-22
//   - `StepBudget::new`                     <- crates/vb_core/src/engine/signals.rs:26-35
//   - `StepBudget::try_take`                <- crates/vb_core/src/engine/signals.rs:50-60
//   - `StepBudget::remaining`               <- crates/vb_core/src/engine/signals.rs:62-66
//   - `StepBudget::from_env`                <- crates/vb_core/src/engine/signals.rs:80-94
//   - `EngineError::StepCounterOverflow`    <- crates/vb_core/src/errors.rs:241
//   - `MAX_STEP_BUDGET = 10_000`            <- crates/vb_core/src/limits.rs:94
//   - `EngineSignal`                        <- crates/vb_core/src/engine/signals.rs:99-115
//                                             (7 variants, all production discriminants
//                                              preserved verbatim)
//
// =============================================================================
// TRUST BOUNDARY
// =============================================================================
// The production bodies of `StepBudget::new`, `try_take`, `remaining`,
// `MAX`, `from_env`, and the `EngineSignal` enum are NOT verified by
// Verus directly (the mirror wraps the methods with
// `#[verifier::external]`, making their bodies opaque). The contracts
// attached via `assume_specification` in `signals_invariant.rs` state
// the production behavior the spec proofs discharge. Drift between
// the mirror and the production source is detected by
// `scripts/check-production-inner-drift.sh` (CI gate).

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
// companion spec file `signals_invariant.rs` attaches spec contracts
// via `assume_specification`.
#[path = "production_inner/signals_production.rs"]
pub mod production_signals;

// Re-export the production types so the spec file can reference them
// via `crate::production::production_signals::StepBudget`.
pub use production_signals::{EngineError, EngineSignal, StepBudget};