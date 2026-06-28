// SPDX-License-Identifier: MIT
//
// Extern surface for step_budget Verus spec.
//
// =============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// This file is the production-binding surface for the `step_budget.rs`
// Verus spec. It contains a direct `#[path]` inclusion of the
// production source file `crates/vb_core/src/engine/signals.rs` so
// any drift in field names, discriminant sets, or fn signatures
// breaks Rust resolution at compile time.
//
// The companion spec file `step_budget.rs` declares the stub modules
// `errors`, `limits`, `value` at the crate root so the production
// file's `use crate::*` statements resolve identically to the
// production crate layout. The spec file attaches spec contracts to
// production methods via `assume_specification[ production::... ]`
// — the production method bodies are NOT verified by Verus directly
// because the included file is declared OUTSIDE the `verus!` block
// and items outside `verus!` are opaque to the verifier.
//
// The `pub` visibility on `StepBudget::remaining` is the single
// production-side change that enables STRONG binding here. Verus's
// `external_type_specification` requires fields declared in the
// underlying exec type to be `pub` for transparent-datatype bridging
// (the bridge is what makes the production type nameable from spec
// mode where `assume_specification` signatures are written). Field
// NAME and TYPE match production byte-for-byte; any drift breaks the
// build.
//
// =============================================================================
// BINDING LEDGER
// =============================================================================
//   - `StepBudget`                          <- crates/vb_core/src/engine/signals.rs:13-17
//   - `StepBudget::MAX`                     <- crates/vb_core/src/engine/signals.rs:21-23
//   - `StepBudget::new`                     <- crates/vb_core/src/engine/signals.rs:28-36
//   - `StepBudget::try_take`                <- crates/vb_core/src/engine/signals.rs:51-61
//   - `StepBudget::remaining`               <- crates/vb_core/src/engine/signals.rs:65-67
//   - `EngineError::StepCounterOverflow`    <- crates/vb_core/src/errors.rs:241
//   - `MAX_STEP_BUDGET = 10_000`            <- crates/vb_core/src/limits.rs:94
//
// =============================================================================
// TRUST BOUNDARY
// =============================================================================
// The production bodies of `StepBudget::new`, `try_take`, `remaining`,
// `MAX`, `from_env`, and the `EngineSignal` enum are NOT verified by
// Verus directly (the `#[path]`-included bodies are opaque). The
// contracts attached via `assume_specification` in `step_budget.rs`
// state the production behavior the spec proofs discharge. Any drift
// in production field names, discriminant sets, or fn signatures
// breaks this Rust resolution at compile time.

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
// sets, or fn signatures will break Rust resolution at compile time.
// The stub modules `errors`, `limits`, `value` referenced by the
// production file are declared at the crate root in `step_budget.rs`.
#[path = "../../crates/vb_core/src/engine/signals.rs"]
pub mod production_signals;

// Re-export the production types so the spec file can reference them
// via `crate::production::production_signals::StepBudget`. Note:
// `EngineError` cannot be re-exported here because the production file's
// `use crate::errors::EngineError;` is a private import — the spec
// references the stub `crate::errors::EngineError` directly via the
// `errors` module declared at the crate root of the spec file.
pub use production_signals::{EngineSignal, StepBudget};