// Verus proof obligation for EngineSignal::Finished canonical payload shape.
//
// Source model: `crates/vb_core/src/engine/signals.rs`.
// Registry obligation: VB-CORE-SIGNAL-001.
// Exact verifier command: `verus verification/verus/signals_invariant.rs`.

use vstd::prelude::*;

verus! {

pub enum SpecSlotValue {
    Null,
    BoolFalse,
    BoolTrue,
    I64Zero,
}

pub enum SpecTaint {
    Clean,
    DerivedFromSecret,
    Secret,
}

pub enum SpecEngineSignal {
    Continue,
    Finished(SpecSlotValue, SpecTaint),
    StepBudgetExhausted,
    AwaitingAction,
    AwaitingWait,
    AwaitingAsk,
}

pub open spec fn spec_engine_signal_finished_taint(signal: SpecEngineSignal) -> bool {
    match signal {
        SpecEngineSignal::Finished(_, _) => true,
        _ => false,
    }
}

pub open spec fn spec_finished_value(signal: SpecEngineSignal) -> Option<SpecSlotValue> {
    match signal {
        SpecEngineSignal::Finished(value, _) => Some(value),
        _ => None,
    }
}

pub open spec fn spec_finished_taint(signal: SpecEngineSignal) -> Option<SpecTaint> {
    match signal {
        SpecEngineSignal::Finished(_, taint) => Some(taint),
        _ => None,
    }
}

pub proof fn proof_finished_carries_taint(value: SpecSlotValue, taint: SpecTaint)
    ensures
        spec_engine_signal_finished_taint(SpecEngineSignal::Finished(value, taint)),
        spec_finished_value(SpecEngineSignal::Finished(value, taint)) == Some(value),
        spec_finished_taint(SpecEngineSignal::Finished(value, taint)) == Some(taint),
{
    assert(spec_engine_signal_finished_taint(SpecEngineSignal::Finished(value, taint))) by(compute);
    assert(spec_finished_value(SpecEngineSignal::Finished(value, taint)) == Some(value)) by(compute);
    assert(spec_finished_taint(SpecEngineSignal::Finished(value, taint)) == Some(taint)) by(compute);
}

pub proof fn proof_non_finished_has_no_finished_payload(signal: SpecEngineSignal)
    requires
        !spec_engine_signal_finished_taint(signal),
    ensures
        spec_finished_value(signal) == Option::<SpecSlotValue>::None,
        spec_finished_taint(signal) == Option::<SpecTaint>::None,
{
    assert(!spec_engine_signal_finished_taint(signal) ==> spec_finished_value(signal) == Option::<SpecSlotValue>::None) by(compute);
    assert(!spec_engine_signal_finished_taint(signal) ==> spec_finished_taint(signal) == Option::<SpecTaint>::None) by(compute);
}

fn main() {}

} // verus!
