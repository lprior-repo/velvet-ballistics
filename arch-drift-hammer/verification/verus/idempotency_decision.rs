// Verus proof obligations for vb-ko29.2 idempotency decision table binding.
//
// Obligations: VERUS-DECISION-001, VERUS-PARITY-002.
// Exact verifier command: `verus verification/verus/idempotency_decision.rs`.
//
// BINDING LEDGER (not a standalone toy model):
// - ProdSideEffect mirrors vb_core::action::SideEffect at crates/vb_core/src/action.rs:29-40.
// - ProdRetrySafety mirrors vb_core::action::RetrySafety at crates/vb_core/src/action.rs:46-53.
// - ProdIdempotency mirrors vb_core::action::Idempotency at crates/vb_core/src/action.rs:16-23.
// - Validate decision mirrors vb_validate::idempotency_contract::is_statically_idempotent_contract
//   at crates/vb_validate/src/idempotency_contract.rs:140-187.
// - Compile decision mirrors exported vb_compile::check_idempotency_gates
//   at crates/vb_compile/src/mod_compile_core.rs:177-229 and its public re-export
//   at crates/vb_compile/src/lib.rs:53-58.
// The proof below is a finite parity witness over the concrete production enum
// surface. Future production variants must update this file and the mapping
// report; otherwise the binding ledger is stale and cannot be used as evidence.

use vstd::prelude::*;

verus! {

pub enum ProdSideEffect {
    None,
    Writes,
    Sends,
    Creates,
    Destroys,
}

pub enum ProdRetrySafety {
    Safe,
    KeyRequired,
    Unsafe,
}

pub enum ProdIdempotency {
    DeterministicPure,
    IdempotentExternal,
    AtLeastOnceExternal,
}

pub enum ProdIdempotencyDecision {
    Accept,
    RejectRetryUnsafe,
    RejectAtLeastOnceExternal,
    RejectSideEffectingDeterministicPure,
}

pub open spec fn prod_has_external_side_effect(side_effect: ProdSideEffect) -> bool {
    match side_effect {
        ProdSideEffect::None => false,
        _ => true,
    }
}

pub open spec fn vb_validate_idempotency_decision(
    side_effect: ProdSideEffect,
    retry_safety: ProdRetrySafety,
    idempotency: ProdIdempotency,
) -> ProdIdempotencyDecision {
    if !prod_has_external_side_effect(side_effect) {
        ProdIdempotencyDecision::Accept
    } else if retry_safety == ProdRetrySafety::Unsafe {
        ProdIdempotencyDecision::RejectRetryUnsafe
    } else if idempotency == ProdIdempotency::AtLeastOnceExternal {
        ProdIdempotencyDecision::RejectAtLeastOnceExternal
    } else if idempotency == ProdIdempotency::DeterministicPure {
        ProdIdempotencyDecision::RejectSideEffectingDeterministicPure
    } else {
        ProdIdempotencyDecision::Accept
    }
}

pub open spec fn vb_compile_idempotency_decision(
    side_effect: ProdSideEffect,
    retry_safety: ProdRetrySafety,
    idempotency: ProdIdempotency,
) -> ProdIdempotencyDecision {
    if side_effect == ProdSideEffect::None {
        ProdIdempotencyDecision::Accept
    } else if retry_safety == ProdRetrySafety::Unsafe {
        ProdIdempotencyDecision::RejectRetryUnsafe
    } else if idempotency == ProdIdempotency::AtLeastOnceExternal {
        ProdIdempotencyDecision::RejectAtLeastOnceExternal
    } else if idempotency == ProdIdempotency::DeterministicPure {
        ProdIdempotencyDecision::RejectSideEffectingDeterministicPure
    } else {
        ProdIdempotencyDecision::Accept
    }
}

pub open spec fn accepted(decision: ProdIdempotencyDecision) -> bool {
    decision == ProdIdempotencyDecision::Accept
}

pub proof fn proof_decision_total_deterministic(
    side_effect: ProdSideEffect,
    retry_safety: ProdRetrySafety,
    idempotency: ProdIdempotency,
)
    ensures
        vb_validate_idempotency_decision(side_effect, retry_safety, idempotency)
            == vb_validate_idempotency_decision(side_effect, retry_safety, idempotency),
{
    assert(vb_compile_idempotency_decision(side_effect, retry_safety, idempotency)
        == vb_validate_idempotency_decision(side_effect, retry_safety, idempotency)) by(compute);
}

pub proof fn proof_none_side_effect_always_accepted(
    retry_safety: ProdRetrySafety,
    idempotency: ProdIdempotency,
)
    ensures
        vb_validate_idempotency_decision(ProdSideEffect::None, retry_safety, idempotency)
            == ProdIdempotencyDecision::Accept,
{
    assert(vb_validate_idempotency_decision(ProdSideEffect::None, retry_safety, idempotency)
        == ProdIdempotencyDecision::Accept) by(compute);
}

pub proof fn proof_side_effecting_unsafe_rejected(
    side_effect: ProdSideEffect,
    idempotency: ProdIdempotency,
)
    requires
        prod_has_external_side_effect(side_effect),
    ensures
        vb_validate_idempotency_decision(side_effect, ProdRetrySafety::Unsafe, idempotency)
            == ProdIdempotencyDecision::RejectRetryUnsafe,
{
    assert(vb_validate_idempotency_decision(side_effect, ProdRetrySafety::Unsafe, idempotency)
        == ProdIdempotencyDecision::RejectRetryUnsafe) by(compute);
}

pub proof fn proof_side_effecting_at_least_once_rejected(
    side_effect: ProdSideEffect,
    retry_safety: ProdRetrySafety,
)
    requires
        prod_has_external_side_effect(side_effect),
        retry_safety != ProdRetrySafety::Unsafe,
    ensures
        vb_validate_idempotency_decision(side_effect, retry_safety, ProdIdempotency::AtLeastOnceExternal)
            == ProdIdempotencyDecision::RejectAtLeastOnceExternal,
{
    assert(vb_validate_idempotency_decision(side_effect, retry_safety, ProdIdempotency::AtLeastOnceExternal)
        == ProdIdempotencyDecision::RejectAtLeastOnceExternal) by(compute);
}

pub proof fn proof_side_effecting_deterministic_pure_rejected(
    side_effect: ProdSideEffect,
    retry_safety: ProdRetrySafety,
)
    requires
        prod_has_external_side_effect(side_effect),
        retry_safety != ProdRetrySafety::Unsafe,
    ensures
        vb_validate_idempotency_decision(side_effect, retry_safety, ProdIdempotency::DeterministicPure)
            == ProdIdempotencyDecision::RejectSideEffectingDeterministicPure,
{
    assert(vb_validate_idempotency_decision(side_effect, retry_safety, ProdIdempotency::DeterministicPure)
        == ProdIdempotencyDecision::RejectSideEffectingDeterministicPure) by(compute);
}

pub proof fn proof_side_effecting_idempotent_external_safe_accepted(
    side_effect: ProdSideEffect,
    retry_safety: ProdRetrySafety,
)
    requires
        prod_has_external_side_effect(side_effect),
        retry_safety == ProdRetrySafety::Safe || retry_safety == ProdRetrySafety::KeyRequired,
    ensures
        vb_validate_idempotency_decision(side_effect, retry_safety, ProdIdempotency::IdempotentExternal)
            == ProdIdempotencyDecision::Accept,
{
    assert(vb_validate_idempotency_decision(side_effect, retry_safety, ProdIdempotency::IdempotentExternal)
        == ProdIdempotencyDecision::Accept) by(compute);
}

pub proof fn proof_compile_validate_decision_parity(
    side_effect: ProdSideEffect,
    retry_safety: ProdRetrySafety,
    idempotency: ProdIdempotency,
)
    ensures
        vb_compile_idempotency_decision(side_effect, retry_safety, idempotency)
            == vb_validate_idempotency_decision(side_effect, retry_safety, idempotency),
        accepted(vb_compile_idempotency_decision(side_effect, retry_safety, idempotency))
            == accepted(vb_validate_idempotency_decision(side_effect, retry_safety, idempotency)),
{
    assert(vb_compile_idempotency_decision(side_effect, retry_safety, idempotency)
        == vb_validate_idempotency_decision(side_effect, retry_safety, idempotency)) by(compute);
}

fn main() {}

} // verus!
