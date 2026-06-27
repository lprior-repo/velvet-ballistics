// SPDX-License-Identifier: MIT
//
// Extern surface for step_budget Verus spec.
//
// =============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// This file is the production-binding surface for the `step_budget.rs`
// Verus spec. It contains a direct `#[path]` inclusion of the production
// source file `crates/vb_core/src/engine/signals.rs` so any drift in
// field names, discriminant sets, or fn signatures breaks Rust resolution
// at compile time.
//
// The companion spec file `step_budget.rs` declares the stub modules
// `errors`, `limits`, `value` at the crate root so the production file's
// `use crate::*` statements resolve identically to the production crate
// layout. The spec file also declares spec-side mirror types and exec
// method wrappers (with `#[verifier::external]`) and attaches spec
// contracts via `assume_specification`.
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
// and `MAX` are NOT verified by Verus directly (the `#[path]`-included
// bodies are opaque). The contracts attached via `assume_specification`
// in `step_budget.rs` state the production behavior the spec proofs
// discharge. Drift between the mirror and the production source is
// reported as binding-debt tracked outside Verus.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

// ---------------------------------------------------------------------------
// PRODUCTION INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of crates/vb_core/src/engine/signals.rs.
// Declared at the crate root so Verus treats its bodies as opaque
// (external by default — items declared outside `verus!` are external).
//
// The included production bodies are NOT verified by Verus. The
// `#[path]` attribute ensures any drift in field names, discriminant
// sets, or fn signatures will break this Rust resolution at compile
// time. The stub modules `errors`, `limits`, `value` referenced by the
// production file are declared at the crate root in `step_budget.rs`.
#[path = "../../crates/vb_core/src/engine/signals.rs"]
pub mod production_signals;

// Re-export the production types so the spec file can reference them
// via `crate::production::production_signals::StepBudget`.
pub use production_signals::{EngineSignal, StepBudget};