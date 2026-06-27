// SPDX-License-Identifier: MIT
//
// Extern surface for run_loop_termination Verus spec.
//
// =============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// This file is the production-binding surface for the
// `run_loop_termination.rs` Verus spec. It binds the run-loop termination
// proofs to the production execution functions declared at
// `crates/vb_core/src/engine/run_loop.rs`:
//
//   - `run_until_blocked`     (run_loop.rs:12-19)
//   - `drive_deterministic`   (run_loop.rs:22-35)
//
// and to the production step / signal / budget types in:
//
//   - `crates/vb_core/src/engine/signals.rs`  (StepBudget, EngineSignal)
//
// The file provides a direct `#[path]` inclusion of
// `crates/vb_core/src/engine/signals.rs` so the real production
// `StepBudget` and `EngineSignal` types are in scope under
// `crate::production::production_signals::*`. Drift in their field
// names, discriminant sets, or fn signatures breaks Rust resolution
// at compile time.
//
// The stub modules `errors`, `limits`, `value`, `ids`, `frame`,
// `workflow`, `value_store` are declared at the spec file's crate root
// (so the included `signals.rs` can resolve its `use crate::*`
// imports and so the spec-side mirror exec fns can reference them by
// type). The spec-side MIRROR types and exec fn wrappers are declared
// INSIDE `verus!` in the companion spec file (`run_loop_termination.rs`)
// following the established `signals_invariant.rs` / `signals_try_take.rs`
// pattern.
//
// =============================================================================
// BINDING LEDGER
// =============================================================================
//   - `StepBudget::new`           <- crates/vb_core/src/engine/signals.rs:27-35
//   - `StepBudget::try_take`      <- crates/vb_core/src/engine/signals.rs:50-60
//   - `StepBudget::remaining`     <- crates/vb_core/src/engine/signals.rs:64-66
//   - `StepBudget::MAX`           <- crates/vb_core/src/engine/signals.rs:20-22
//   - `EngineSignal` discriminant <- crates/vb_core/src/engine/signals.rs:99-115
//   - `drive_deterministic`       <- crates/vb_core/src/engine/run_loop.rs:22-35
//   - `run_until_blocked`         <- crates/vb_core/src/engine/run_loop.rs:12-19
//
// =============================================================================
// TRUST BOUNDARY
// =============================================================================
// The production bodies of `StepBudget::new`, `try_take`, `remaining`,
// `MAX` are NOT verified by Verus directly (the `#[path]`-included
// bodies are opaque). The contracts attached via `assume_specification`
// in the companion spec file state the production behavior the spec
// proofs discharge. The spec-side mirror exec fns (`mirror_*`) declared
// inside `verus!` have `#[verifier::external]` bodies that faithfully
// mirror the production logic. Drift between the mirror and the
// production source is reported as binding-debt tracked outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// Stub modules for production `crate::*` imports (declared at spec file root)
// ---------------------------------------------------------------------------
//
// The spec file declares these stubs at its crate root so the
// `#[path]`-included `signals.rs` can resolve `use crate::errors::*`,
// `use crate::limits::*`, `use crate::value::*`, `use crate::ids::*`,
// and so the spec-side mirror exec fns can type their `plan`, `run`,
// `store` parameters. This file contains NO stub modules — they all
// live in `run_loop_termination.rs` so the crate root of that file is
// the canonical location (matching the `signals_invariant.rs` pattern).

// ---------------------------------------------------------------------------
// PRODUCTION INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of `crates/vb_core/src/engine/signals.rs`.
// The included file is treated as opaque by Verus (its items are
// external by virtue of being declared outside `verus!`). The `#[path]`
// attribute ensures any drift in field names, discriminant sets, or
// fn signatures breaks Rust resolution at compile time.
#[path = "../../crates/vb_core/src/engine/signals.rs"]
pub mod production_signals;

// Re-export the production types so the spec file can attach contracts
// to them via `assume_specification` and reference them as
// `crate::production::production_signals::{StepBudget, EngineSignal}`.
pub use production_signals::{EngineSignal, StepBudget};
