// Minimal experiment: does #[verifier::external_type_specification] allow
// accessing a PRIVATE field of the target type in spec mode?
//
// Hypothesis: private fields of an external type are accessible in spec
// mode through the external_type_specification bridge.

#![allow(dead_code)]

use vstd::prelude::*;

pub mod errors {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum EngineError {
        StepCounterOverflow,
        BudgetParse {
            reason: &'static str,
        },
    }
}

pub mod limits {
    pub const MAX_STEP_BUDGET: u64 = 10_000;
}

pub mod value {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SlotValue {
        I64(i64),
        Bool(bool),
        Null,
    }
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Taint {
        Clean,
        Secret,
        DerivedFromSecret,
    }
}

#[path = "extern_signals_try_take.rs"]
mod production;

pub use production::StepBudget;

// Try to use #[verifier::external_type_specification] to bridge
// production::StepBudget (which has PRIVATE remaining field).
verus! {

#[verifier::external_type_specification]
pub struct ExStepBudget(production::StepBudget);

#[verifier::external_type_specification]
pub struct ExEngineError(crate::errors::EngineError);

pub open spec fn spec_new(v: int) -> int {
    if v > 10000 { 10000 } else { v }
}

pub assume_specification[ production::StepBudget::new ](
    value: u64,
) -> (budget: production::StepBudget)
    ensures
        budget.remaining as int == spec_new(value as int),
;

fn test_private_field_access(value: u64) -> (budget: production::StepBudget)
    ensures
        budget.remaining as int == spec_new(value as int),
{
    production::StepBudget::new(value)
}

fn test_method_call_access(value: u64) -> (budget: production::StepBudget)
    ensures
        budget.remaining() as int == spec_new(value as int),
{
    production::StepBudget::new(value)
}

fn main() {}

} // verus!
