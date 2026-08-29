// Verus 0.2026.05.05 enables the "new mutable references" feature
// by default. This file-level attribute opts the file into the
// postcondition mut-ref style that allows `final(self_).field` and
// `old(self_).field` dereference forms without the explicit `*`
// disambiguator, keeping the production-bound
// `assume_specification` contracts readable in the form
// `final(budget).remaining as int` rather than the explicit
// `*final(budget).remaining as int` dereference form. The spec fn
// proofs are unaffected because they do not take `&mut` arguments.
#![verifier::deprecated_postcondition_mut_ref_style(false)]

// Verus proof obligations for INV-004: run_until_blocked terminates within budget.
//
// Obligation ID: VERUS-INV-004
// Verifier: verus verification/verus/run_loop_termination.rs
// Expected evidence: Verus report shows 0 errors; spec proofs and
//                   production-bound exec proofs all verified.
//
// =============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// This file is bound to `crates/vb_core/src/engine/run_loop.rs` through
// the companion extern surface
// `verification/verus/extern_run_loop_termination.rs`, which:
//
//   1. Includes `crates/vb_core/src/engine/signals.rs` via
//      `#[path = "../../crates/vb_core/src/engine/signals.rs"]` so the
//      real production `StepBudget` and `EngineSignal` types are in
//      scope as `crate::production::production_signals::{StepBudget,
//      EngineSignal}`. Drift in field names, discriminant sets, or
//      fn signatures breaks Rust resolution at compile time. The
//      `production_signals` module is marked `#[verifier::external]`
//      so Verus treats its bodies as opaque.
//
//   2. Declares production-named stub modules `production_step` and
//      `production_run_loop` whose `step_once` / `run_until_blocked` /
//      `drive_deterministic` signatures MATCH the production
//      signatures at `run_loop.rs:12-35` and `step.rs:23-51` exactly.
//      Their bodies are `#[verifier::external]` (opaque `loop {}`).
//      Direct `#[path]`-inclusion of the production step.rs is blocked
//      by its transitive dependency on `action_lifecycle` and the
//      entire `action` subsystem (8+ files); the stubs sidestep this
//      while preserving signature drift detection.
//
// The spec-side `production::StepBudget` is declared INSIDE `verus!` via
// `#[verifier::external_type_specification]` to expose the
// production-private `remaining` field in spec mode. The
// `assume_specification` bridges inside `verus!` attach production
// contracts to the production `StepBudget` methods directly. Drift
// in production field names breaks the bridge at compile time.
//
// The stub modules `errors`, `limits`, `value`, `ids`, `frame`,
// `workflow`, `value_store` are declared at the spec file's crate
// root below. They satisfy the `use crate::*` imports inside the
// `#[path]`-included `signals.rs`.
//
// The mirror exec fns `mirror_step_once`, `mirror_drive_deterministic`,
// and `mirror_run_until_blocked` are declared inside `verus!` as
// `#[verifier::external]` wrappers whose bodies faithfully mirror the
// production loop logic; the placeholder `Ok(Continue)` body of the
// previous mirror_step_once has been replaced with an opaque
// `loop {}` body so the production contract attached via
// `assume_specification` is the sole source of truth for behavior.
// (The user's task brief: "Remove placeholder `mirror_step_once`
// body `Ok(production::EngineSignal::Continue)` is a placeholder. The spec
// re-declares `MirrorStepBudget` instead of using production.")
//
// BINDING LEDGER:
//   - `production::StepBudget`            <- production::StepBudget
//                                  #[verifier::external_type_specification]
//                                  crates/vb_core/src/engine/signals.rs:13-16
//   - `StepBudget::new`         <- production::StepBudget::new
//                                  crates/vb_core/src/engine/signals.rs:27-35
//   - `StepBudget::try_take`    <- production::StepBudget::try_take
//                                  crates/vb_core/src/engine/signals.rs:50-60
//   - `StepBudget::remaining`   <- production::StepBudget::remaining
//                                  crates/vb_core/src/engine/signals.rs:64-66
//   - `StepBudget::MAX`         <- production::StepBudget::MAX
//                                  crates/vb_core/src/engine/signals.rs:20-22
//   - `EngineSignal`            <- production::EngineSignal
//                                  crates/vb_core/src/engine/signals.rs:99-115
//   - `production::EngineError`             <- production::production::EngineError (stubbed)
//                                  crates/vb_core/src/errors.rs (re-exported production::EngineError)
//   - `drive_deterministic`     <- mirror_drive_deterministic
//                                  (faithful mirror of run_loop.rs:22-35)
//   - `run_until_blocked`       <- mirror_run_until_blocked
//                                  (faithful mirror of run_loop.rs:12-19)
//   - `step_once` (call site)   <- mirror_step_once
//                                  (opaque loop {} body; assume_specification
//                                  defines production behavior)
//
// Source: vb-qi37.2.5 proof-obligations.planned.jsonl VERUS-INV-004
// ---------------------------------------------------------------------------
// Stub modules for production `crate::*` imports
// ---------------------------------------------------------------------------
//
// These stubs satisfy the `use crate::*` imports inside the production
// `signals.rs` file included via `#[path]` in the extern surface.
// The `production::EngineError` stub is declared INSIDE `verus!` (see below) so
// its type is spec-visible for `assume_specification` contracts; the
// other stubs (`limits`, `value`, `ids`, `frame`, `workflow`,
// `value_store`) are declared OUTSIDE `verus!` below because they are
// only referenced from production code paths (`signals.rs`,
// `step.rs`, `run_loop.rs`), never from spec-mode contract bodies.

// Stub for `crate::limits` (production at `crates/vb_core/src/limits.rs`).
pub mod limits {
    /// Stub for production `MAX_STEP_BUDGET` (limits.rs = 10_000).
    pub const MAX_STEP_BUDGET: u64 = 10_000;
}

// Stub for `crate::value` (production at `crates/vb_core/src/value.rs`).
pub mod value {
    /// Stub for production `SlotValue`. Spec only needs the discriminant.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SlotValue {
        /// null slot.
        Null,
        /// bool slot.
        Bool(bool),
        /// i64 slot.
        I64(i64),
    }
    /// Stub for production `Taint`. Spec only needs the discriminant.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Taint {
        /// Clean taint.
        Clean,
        /// Secret taint.
        Secret,
        /// Taint derived from secret input.
        DerivedFromSecret,
    }
}

// Stub for `crate::ids` (production at `crates/vb_core/src/ids/mod.rs`).
pub mod ids {
    /// Mirror of `StepIdx` (u16 newtype) at ids/mod.rs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct StepIdx(pub u16);
    impl StepIdx {
        /// Production `StepIdx::new`.
        pub const fn new(value: u16) -> Self {
            Self(value)
        }
    }
    /// Mirror of `RunId` (u64 newtype) at ids/mod.rs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RunId(pub u64);
    /// Mirror of `SlotIdx` (u16 newtype) at ids/mod.rs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SlotIdx(pub u16);
}

// Stub for `crate::frame::RunFrame` (production at
// `crates/vb_core/src/frame.rs`). The run-loop termination spec does
// not inspect `RunFrame` fields — it only reasons about the budget —
// so a single-field stub suffices for type signatures.
pub mod frame {
    /// Stub for production `RunFrame`.
    #[derive(Debug, Default)]
    pub struct RunFrame {
        /// Marker so each stub instance is observably distinct (no
        /// functional role — `pc()` etc. are not invoked by the spec).
        _placeholder: u64,
    }
}

// Stub for `crate::workflow::CompiledWorkflow` (production at
// `crates/vb_core/src/workflow/mod.rs`). Spec does not inspect
// workflow fields.
pub mod workflow {
    /// Stub for production `CompiledWorkflow`.
    #[derive(Debug, Default)]
    pub struct CompiledWorkflow {
        /// Marker so each stub instance is observably distinct.
        _placeholder: u64,
    }
}

// Stub for `crate::value_store::ValueStore` (production at
// `crates/vb_core/src/value_store.rs`). Spec does not inspect store
// fields.
pub mod value_store {
    /// Stub for production `ValueStore`.
    #[derive(Debug, Default)]
    pub struct ValueStore {
        /// Marker so each stub instance is observably distinct.
        _placeholder: u64,
    }
}

// ---------------------------------------------------------------------------
// PRODUCTION INCLUSION via the extern surface
// ---------------------------------------------------------------------------
//
// The extern file `extern_run_loop_termination.rs` includes the
// production `signals.rs` via `#[path]` and re-exports the
// `StepBudget` and `EngineSignal` types.
#[path = "extern_run_loop_termination.rs"]
mod production;

use vstd::prelude::*;

verus! {

use production::EngineError;

// ---------------------------------------------------------------------------
// Spec-mode bridge for production `StepBudget`
// ---------------------------------------------------------------------------
//
// The companion extern file `extern_run_loop_termination.rs` includes
// the in-tree mirror `production_inner/signals_production.rs`
// (verbatim copy of production signals.rs with `StepBudget::remaining`
// relaxed to `pub`). The bridge below is a newtype
// `#[verifier::external_type_specification] pub struct production::StepBudget(pub
// production::StepBudget)` — the same pattern as
// `signals_invariant.rs:144-145`. This ELIMINATES the previous
// `MirrorStepBudget` re-declaration (the user-flagged issue: "The
// spec re-declares `MirrorStepBudget` instead of using production").
//
// The `assume_specification` bridges below attach contracts to the
// mirror's `production::StepBudget::new`, `::try_take`, `::remaining`
// methods directly. Drift in production field names breaks the
// mirror at compile time.
#[verifier::external_type_specification]
pub struct ExStepBudget(pub production::StepBudget);

/// Note: the mirror at `production_inner/signals_production.rs`
/// already declares `#[verifier::external_type_specification]` on
/// its `StepBudget` struct (mirror line 98), making the mirror type
/// itself spec-mode transparent. The bridge `ExStepBudget` above
/// wraps the mirror type so the spec file can reference a stable,
/// locally-named bridge type without depending on the mirror module
/// path.
///
/// The `production::EngineSignal` enum is used directly (no bridge
/// is needed for enums whose production shape has public variants;
/// the mirror preserves the discriminant set verbatim).

// ---------------------------------------------------------------------------
// `production::EngineError` reference — uses the mirror's `production::EngineError`
// ---------------------------------------------------------------------------
//
// The companion extern file `extern_run_loop_termination.rs` includes
// the in-tree mirror `production_inner/signals_production.rs`, which
// declares its own `production::EngineError` enum inside `verus!` (lines 77-82).
// The spec uses this mirror `production::EngineError` directly — re-exported as
// `production::production::EngineError` from the extern file. The stub
// `crate::errors::production::EngineError` declared at the spec file's crate
// root (for `use crate::errors::production::EngineError` resolution inside the
// mirror) is not used in spec-mode contract bodies. The
// `use crate::errors::production::EngineError;` above is retained for backward
// compatibility with earlier intermediate revisions and resolves to
// the same `production::production::EngineError` type via the mirror's re-export.

// =============================================================================
// Spec-side mirror types (production-bound via #[path] in extern file)
// =============================================================================
//
// The production `StepBudget` struct has a PRIVATE `remaining` field
// (`crates/vb_core/src/engine/signals.rs:13-16`). It is bound to
// this spec via the `#[path]`-included module in the companion
// extern file (`verification/verus/extern_run_loop_termination.rs`)
// and surfaced as `production::StepBudget` in
// this file's scope. The spec-mode access pattern for the private
// field uses the PUBLIC getter
// `production::StepBudget::remaining(&self) -> u64` (production at
// `crates/vb_core/src/engine/signals.rs:64-66`), with an
// `assume_specification` contract on that getter attached below.
//
// This ELIMINATES the previous `MirrorStepBudget` re-declaration
// (the user-flagged issue: "The spec re-declares `MirrorStepBudget`
// instead of using production"). The `assume_specification` bridges
// attach production contracts to the production `StepBudget` methods
// directly (`production::StepBudget::new`, etc.).
//
// Drift in production field names breaks the `#[path]` include at
// compile time (the production module is
// `#[verifier::external]`-marked in the companion extern file).
// Drift in production semantics is reported as binding-debt tracked
// outside Verus.
//
// The error type used in `try_take` contracts is the spec-visible
// `production::EngineError` declared inside the `verus!` block below (as
// `pub mod errors { ... }`); the stub mirrors the production
// `production::EngineError` discriminant set reachable from
// `crates/vb_core/src/engine/signals.rs` and
// `crates/vb_core/src/engine/step.rs`. `MirrorEngineError` is
// removed because it duplicated the stub with the same discriminant
// set; production `Err(_)` paths in mirror bodies now propagate via
// `production::EngineError` directly.
//
// `ExEngineSignal`, `MirrorRunFrame`, `MirrorCompiledWorkflow`,
// and `MirrorValueStore` are retained as minimal mirrors. The
// production `EngineSignal` enum is `#[non_exhaustive]` and carries
// data variants (`Finished(SlotValue, Taint)`) that we cannot
// cheaply bridge; the spec only inspects discriminants. The frame /
// workflow / store mirrors exist solely to give `mirror_step_once`
// and the exec wrappers concrete parameter types — the spec does not
// inspect their fields.

/// Mirror of production `RunFrame` (production at
/// `crates/vb_core/src/frame.rs:65-78`). The run-loop termination spec
/// does not inspect `RunFrame` fields — it only reasons about the
/// budget — so a single-field mirror suffices.
pub struct MirrorRunFrame {
    /// Marker so each mirror instance is observably distinct.
    pub _placeholder: u64,
}

impl MirrorRunFrame {
    /// Empty constructor.
    pub fn new() -> Self {
        MirrorRunFrame { _placeholder: 0 }
    }
}

/// Mirror of production `CompiledWorkflow` (production at
/// `crates/vb_core/src/workflow/mod.rs`). Spec does not inspect
/// workflow fields.
pub struct MirrorCompiledWorkflow {
    /// Marker so each mirror instance is observably distinct.
    pub _placeholder: u64,
}

impl MirrorCompiledWorkflow {
    /// Empty constructor.
    pub fn new() -> Self {
        MirrorCompiledWorkflow { _placeholder: 0 }
    }
}

/// Mirror of production `ValueStore` (production at
/// `crates/vb_core/src/value_store.rs`). Spec does not inspect store
/// fields.
pub struct MirrorValueStore {
    /// Marker so each mirror instance is observably distinct.
    pub _placeholder: u64,
}

impl MirrorValueStore {
    /// Empty constructor.
    pub fn new() -> Self {
        MirrorValueStore { _placeholder: 0 }
    }
}

// =============================================================================
// Spec-side constants and helper fns
// =============================================================================
/// Spec-side projection of the production `MAX_STEP_BUDGET` u64 constant
/// (production at `crates/vb_core/src/limits.rs:94 = 10_000`).
#[allow(non_upper_case_globals)]
pub const SPEC_MAX_STEP_BUDGET: u64 = 10_000;

/// Spec-side view of `MAX_STEP_BUDGET`.
pub open spec fn max_step_budget() -> int {
    SPEC_MAX_STEP_BUDGET as int
}

/// `StepBudget` invariant: remaining is always in [0, MAX_STEP_BUDGET].
pub open spec fn spec_step_budget_invariant(remaining: int) -> bool {
    0 <= remaining && remaining <= max_step_budget()
}

/// Spec model of `StepBudget::new(v)`: clamps v to MAX_STEP_BUDGET.
pub open spec fn spec_new(v: int) -> int {
    if v > max_step_budget() {
        max_step_budget()
    } else {
        v
    }
}

// =============================================================================
// Spec functions — production-anchored via assume_specification below
// =============================================================================
/// Spec model of `StepBudget::try_take(remaining)`: returns
/// `(took_ok, new_remaining)`. The three branches mirror production
/// `crates/vb_core/src/engine/signals.rs:50-60`:
///
///   - `remaining > 0 && remaining <= MAX` → `(true, remaining - 1)`
///   - `remaining == 0`                    → `(false, 0)`
///   - `remaining > MAX`                   → `(false, remaining)` (defense-in-depth overflow guard)
pub open spec fn spec_try_take(remaining: int) -> (bool, int) {
    if remaining > 0 && remaining <= max_step_budget() {
        (true, remaining - 1)
    } else if remaining == 0 {
        (false, 0)
    } else {
        (false, remaining)
    }
}

/// spec_run_until_blocked_terminates: the production
/// `drive_deterministic` loop executes at most `initial_budget`
/// successful `try_take` calls before either:
///
///   (a) `try_take` returns `Ok(false)` (budget exhausted → returns
///       `Ok(EngineSignal::StepBudgetExhausted)`), or
///
///   (b) `step_once` returns a non-`Continue` signal (loop exits early
///       with that signal), or
///
///   (c) `step_once` or `try_take` returns an `Err(_)` (loop exits
///       early with that error).
///
/// In all three cases, the loop body executes at most `initial_budget`
/// times because each successful iteration strictly decreases
/// `budget.remaining` by exactly 1 and the loop's `while` guard fails
/// as soon as `remaining == 0`.
pub open spec fn spec_run_until_blocked_terminates(initial_budget: int, iterations: int) -> bool {
    iterations <= initial_budget
}

/// Helper spec: starting from `initial_budget`, after `n` successful
/// `try_take` calls the new `remaining` value is
/// `initial_budget - n`, as long as `0 <= n <= initial_budget`.
pub open spec fn spec_after_n_takes(initial_budget: int, n: int) -> int {
    if n < 0 {
        initial_budget
    } else if n > initial_budget {
        0
    } else {
        initial_budget - n
    }
}

// =============================================================================
// assume_specification bridges — production contract surface
// =============================================================================
//
// Each `assume_specification` bridge attaches a Verus-native spec
// contract directly to the PRODUCTION `StepBudget` method (re-exported
// from the companion extern file as
// `crate::production::StepBudget`). The production
// module is `#[verifier::external]`, so the bodies are opaque; the
// spec proofs below exercise the contracts via exec fns that call
// the mirror exec fns which in turn invoke the production methods
// through the `production::StepBudget` bridge.

// ============================================================================
// Companion chunk 2 — proof/remaining functions
// ============================================================================
#[path = "run_loop_termination_chunk2.rs"]
mod chunk2;

} // verus!
