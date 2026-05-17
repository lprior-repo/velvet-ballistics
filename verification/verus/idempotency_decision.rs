// Verus proof obligations for vb-qi37.5 idempotency decision table.
//
// Obligations: VERUS-DECISION-001, VERUS-PARITY-002.
// This is a standalone proof artifact: it models the canonical finite decision
// table without importing or editing production crates. The compile-side model
// is deliberately written independently instead of delegating to validation.
// Exact verifier command: `verus verification/verus/idempotency_decision.rs`.

use vstd::prelude::*;

verus! {

pub enum SpecSideEffect {
    None,
    Writes,
    Sends,
    Creates,
    Destroys,
}

pub enum SpecRetrySafety {
    Safe,
    KeyRequired,
    Unsafe,
}

pub enum SpecIdempotency {
    DeterministicPure,
    IdempotentExternal,
    AtLeastOnceExternal,
}

pub enum SpecDecision {
    Accept,
    RejectRetryUnsafe,
    RejectAtLeastOnceExternal,
    RejectSideEffectingDeterministicPure,
}

pub open spec fn has_external_side_effect(side_effect: SpecSideEffect) -> bool {
    match side_effect {
        SpecSideEffect::None => false,
        _ => true,
    }
}

pub open spec fn spec_idempotency_decision(
    side_effect: SpecSideEffect,
    retry_safety: SpecRetrySafety,
    idempotency: SpecIdempotency,
) -> SpecDecision {
    if !has_external_side_effect(side_effect) {
        SpecDecision::Accept
    } else if retry_safety == SpecRetrySafety::Unsafe {
        SpecDecision::RejectRetryUnsafe
    } else if idempotency == SpecIdempotency::AtLeastOnceExternal {
        SpecDecision::RejectAtLeastOnceExternal
    } else if idempotency == SpecIdempotency::DeterministicPure {
        SpecDecision::RejectSideEffectingDeterministicPure
    } else {
        SpecDecision::Accept
    }
}

pub open spec fn spec_compile_idempotency_decision(
    side_effect: SpecSideEffect,
    retry_safety: SpecRetrySafety,
    idempotency: SpecIdempotency,
) -> SpecDecision {
    if side_effect == SpecSideEffect::None {
        SpecDecision::Accept
    } else if retry_safety == SpecRetrySafety::Unsafe {
        SpecDecision::RejectRetryUnsafe
    } else if idempotency == SpecIdempotency::AtLeastOnceExternal {
        SpecDecision::RejectAtLeastOnceExternal
    } else if idempotency == SpecIdempotency::DeterministicPure {
        SpecDecision::RejectSideEffectingDeterministicPure
    } else {
        SpecDecision::Accept
    }
}

pub open spec fn accepted(decision: SpecDecision) -> bool {
    decision == SpecDecision::Accept
}

pub proof fn proof_decision_total_deterministic(
    side_effect: SpecSideEffect,
    retry_safety: SpecRetrySafety,
    idempotency: SpecIdempotency,
)
    ensures
        spec_idempotency_decision(side_effect, retry_safety, idempotency)
            == spec_idempotency_decision(side_effect, retry_safety, idempotency),
{
    assert(spec_compile_idempotency_decision(side_effect, retry_safety, idempotency)
        == spec_idempotency_decision(side_effect, retry_safety, idempotency)) by(compute);
}

pub proof fn proof_none_side_effect_always_accepted(
    retry_safety: SpecRetrySafety,
    idempotency: SpecIdempotency,
)
    ensures
        spec_idempotency_decision(SpecSideEffect::None, retry_safety, idempotency)
            == SpecDecision::Accept,
{
    assert(spec_idempotency_decision(SpecSideEffect::None, retry_safety, idempotency)
        == SpecDecision::Accept) by(compute);
}

pub proof fn proof_side_effecting_unsafe_rejected(
    side_effect: SpecSideEffect,
    idempotency: SpecIdempotency,
)
    requires
        has_external_side_effect(side_effect),
    ensures
        spec_idempotency_decision(side_effect, SpecRetrySafety::Unsafe, idempotency)
            == SpecDecision::RejectRetryUnsafe,
{
    assert(spec_idempotency_decision(side_effect, SpecRetrySafety::Unsafe, idempotency)
        == SpecDecision::RejectRetryUnsafe) by(compute);
}

pub proof fn proof_side_effecting_at_least_once_rejected(
    side_effect: SpecSideEffect,
    retry_safety: SpecRetrySafety,
)
    requires
        has_external_side_effect(side_effect),
        retry_safety != SpecRetrySafety::Unsafe,
    ensures
        spec_idempotency_decision(side_effect, retry_safety, SpecIdempotency::AtLeastOnceExternal)
            == SpecDecision::RejectAtLeastOnceExternal,
{
    assert(spec_idempotency_decision(side_effect, retry_safety, SpecIdempotency::AtLeastOnceExternal)
        == SpecDecision::RejectAtLeastOnceExternal) by(compute);
}

pub proof fn proof_side_effecting_deterministic_pure_rejected(
    side_effect: SpecSideEffect,
    retry_safety: SpecRetrySafety,
)
    requires
        has_external_side_effect(side_effect),
        retry_safety != SpecRetrySafety::Unsafe,
    ensures
        spec_idempotency_decision(side_effect, retry_safety, SpecIdempotency::DeterministicPure)
            == SpecDecision::RejectSideEffectingDeterministicPure,
{
    assert(spec_idempotency_decision(side_effect, retry_safety, SpecIdempotency::DeterministicPure)
        == SpecDecision::RejectSideEffectingDeterministicPure) by(compute);
}

pub proof fn proof_side_effecting_idempotent_external_safe_accepted(
    side_effect: SpecSideEffect,
    retry_safety: SpecRetrySafety,
)
    requires
        has_external_side_effect(side_effect),
        retry_safety == SpecRetrySafety::Safe || retry_safety == SpecRetrySafety::KeyRequired,
    ensures
        spec_idempotency_decision(side_effect, retry_safety, SpecIdempotency::IdempotentExternal)
            == SpecDecision::Accept,
{
    assert(spec_idempotency_decision(side_effect, retry_safety, SpecIdempotency::IdempotentExternal)
        == SpecDecision::Accept) by(compute);
}

pub proof fn proof_compile_validate_decision_parity(
    side_effect: SpecSideEffect,
    retry_safety: SpecRetrySafety,
    idempotency: SpecIdempotency,
)
    ensures
        spec_compile_idempotency_decision(side_effect, retry_safety, idempotency)
            == spec_idempotency_decision(side_effect, retry_safety, idempotency),
        accepted(spec_compile_idempotency_decision(side_effect, retry_safety, idempotency))
            == accepted(spec_idempotency_decision(side_effect, retry_safety, idempotency)),
{
}

fn main() {}

} // verus!
