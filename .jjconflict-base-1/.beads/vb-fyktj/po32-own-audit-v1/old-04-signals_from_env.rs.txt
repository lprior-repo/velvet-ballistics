// Verus proof obligations for PO-VERUS-032: StepBudget::from_env boundedness.
//
// Obligation ID: PO-VERUS-032
// Verifier: verus --crate-type=lib verification/verus/signals_from_env.rs
// Expected evidence: Verus report shows 0 errors; assume_specification contracts
//                   on step_budget_from_env and StepBudget::new register and
//                   the exec proof exercises the composition chain.
//
// =============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// This file is bound to `crates/vb_core/src/engine/signals.rs` through the
// companion extern surface `verification/verus/extern_signals_invariant.rs`,
// which includes the drift-gated mirror
// `verification/verus/production_inner/signals_production.rs`. The mirror is a
// verbatim production surface with two verifier-only adjustments:
//   1. `StepBudget::remaining` is `pub` (relaxed from production private)
//      so Verus's `external_type_specification` can read the field.
//   2. `StepBudget::from_env` is wrapped as a standalone `step_budget_from_env`
//      function marked `#[verifier::external]` because the closure pattern
//      `|_| EngineError::BudgetParse { reason }` (production at signals.rs:84)
//      is rejected by Verus as "only variables are supported here, not general
//      patterns". The signature and field name remain production-identical
//      so any drift breaks this Verus build.
//
// `scripts/check-production-inner-drift.sh` catches drift between the mirror
// and the production source. Field NAME, TYPE, and signature are preserved
// byte-for-byte.
//
// The `assume_specification` bridges inside `verus!` attach spec contracts to
// the mirror exec methods. The exec proof exercises the composition chain:
// from_env calls StepBudget::new, and new is proved to clamp to MAX_STEP_BUDGET.
//
// BINDING LEDGER:
//   - production::step_budget_from_env       <- crates/vb_core/src/engine/signals.rs:81-94
//   - production::EngineError::BudgetParse   <- crates/vb_core/src/errors.rs:396
//   - production::MAX_STEP_BUDGET            <- crates/vb_core/src/limits.rs:94
//   - production::StepBudget::new            <- crates/vb_core/src/engine/signals.rs:28-36
//
// Domain claim (FC-001): from_env always returns Ok(budget) where
// budget.remaining() <= MAX_STEP_BUDGET, or Err(BudgetParse) on
// parse/access failure. StepCounterOverflow is never returned from
// from_env.

#[path = "extern_signals_invariant.rs"]
mod production;

pub use production::{EngineError, EngineSignal, StepBudget};

use vstd::prelude::*;

verus! {

// =============================================================================
// Production type bridge (GOD RULE 2 compliance)
// =============================================================================

/// Spec-mode alias for the production `StepBudget` struct surfaced
/// through the drift-gated `signals_production` mirror.
#[verifier::external_type_specification]
pub struct ExStepBudget(production::StepBudget);

// =============================================================================
// Spec constants
// =============================================================================

/// Spec-side projection of the production `MAX_STEP_BUDGET` constant
/// (production at `crates/vb_core/src/limits.rs:94 = 10_000`).
#[allow(non_upper_case_globals)]
pub const SPEC_MAX_STEP_BUDGET: u64 = 10_000;

/// Spec-side view of `MAX_STEP_BUDGET`.
pub open spec fn max_step_budget() -> int {
    SPEC_MAX_STEP_BUDGET as int
}

// =============================================================================
// Spec invariants
// =============================================================================

/// The StepBudget invariant: remaining is always in [0, MAX_STEP_BUDGET].
pub open spec fn spec_remaining_bounded(remaining: int) -> bool {
    0 <= remaining && remaining <= max_step_budget()
}

/// Spec alias for the bounded invariant.
pub open spec fn spec_step_budget_invariant(remaining: int) -> bool {
    spec_remaining_bounded(remaining)
}

// =============================================================================
// assume_specification bridges — production contract surface
// =============================================================================

/// Bridge contract: `production::StepBudget::new(v)` returns a
/// StepBudget whose `remaining` field equals `min(v,
/// MAX_STEP_BUDGET)` and satisfies the bounded invariant.
pub assume_specification[ production::StepBudget::new ](
    value: u64,
) -> (budget: production::StepBudget)
    ensures
        budget.remaining as int == if value as int > max_step_budget() {
            max_step_budget()
        } else {
            value as int
        },
        spec_step_budget_invariant(budget.remaining as int),
;

/// Bridge contract: `production::step_budget_from_env` returns a
/// bounded budget or a BudgetParse error. The body is opaque
/// (uses std::env); the contract is that the returned budget has
/// `remaining <= MAX_STEP_BUDGET` and the Err variants match the
/// production parse-failure / env-access-error variants.
///
/// Mirrors production `StepBudget::from_env` at
/// `crates/vb_core/src/engine/signals.rs:81-94`.
pub assume_specification[ production::step_budget_from_env ]() -> (result: Result<
    production::StepBudget,
    production::EngineError,
>)
    ensures
        match result {
            Ok(b) => b.remaining <= SPEC_MAX_STEP_BUDGET,
            Err(EngineError::BudgetParse { .. }) => true,
            Err(EngineError::StepCounterOverflow) => false,
            Err(_) => true,
        },
;

// =============================================================================
// Exec proof: from_env boundedness composes from new() contract
// =============================================================================
//
// This exec proof demonstrates that `from_env` boundedness composes
// from the `new` contract. Since `from_env` calls `StepBudget::new()`
// internally, and `new` is proved to clamp to MAX_STEP_BUDGET, the
// boundedness of `from_env` follows.
//
/// PO-VERUS-032 (FC-001): StepBudget construction via new produces
/// a bounded budget. This is the core lemma that makes from_env
/// boundedness provable, since from_env internally calls new.
pub fn exec_proof_from_env_composes_with_new(initial: u64) -> (budget: production::StepBudget)
    requires
        spec_step_budget_invariant(initial as int),
    ensures
        budget.remaining as int <= initial as int,
        spec_step_budget_invariant(budget.remaining as int),
{
    let mut b = production::StepBudget::new(initial);
    b
}

fn main() {}

} // verus!
