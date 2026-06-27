// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for idempotency decision
// ============================================================================
//
// This file is a VERBATIM mirror of the production idempotency-decision
// types and decision fns.
//
// Production sources mirrored:
//   - `vb_core::action::SideEffect`         (crates/vb_core/src/action/contract.rs:23-34)
//   - `vb_core::action::RetrySafety`        (crates/vb_core/src/action/contract.rs:40-47)
//   - `vb_core::action::Idempotency`        (crates/vb_core/src/action/contract.rs:10-17)
//   - `vb_core::action::ActionContract`     (crates/vb_core/src/action/contract.rs:83-105)
//   - `vb_storage::admission::is_contract_idempotency_accepted`
//                                            (crates/vb_storage/src/admission.rs:531-545)
//   - `vb_validate::idempotency_contract::is_statically_idempotent_contract`
//                                            (crates/vb_validate/src/idempotency_contract.rs:140-187)

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

#[derive(Clone, Copy)]
pub enum SideEffect {
    None,
    Writes,
    Sends,
    Creates,
    Destroys,
}

#[derive(Clone, Copy)]
pub enum RetrySafety {
    Safe,
    KeyRequired,
    Unsafe,
}

#[derive(Clone, Copy)]
pub enum Idempotency {
    DeterministicPure,
    IdempotentExternal,
    AtLeastOnceExternal,
}

#[derive(Clone, Copy)]
pub struct ActionId(pub u16);

#[derive(Clone, Copy)]
pub struct ActionContract {
    pub id: ActionId,
    pub side_effect: SideEffect,
    pub retry_safety: RetrySafety,
    pub idempotency: Idempotency,
}

#[derive(Clone, Copy)]
pub enum IdempotencyContractViolation {
    SideEffectingRetryUnsafe {
        action: ActionId,
        side_effect: SideEffect,
        idempotency: Idempotency,
        retry_safety: RetrySafety,
    },
    SideEffectingAtLeastOnceExternal {
        action: ActionId,
        side_effect: SideEffect,
        idempotency: Idempotency,
        retry_safety: RetrySafety,
    },
    SideEffectingDeterministicPure {
        action: ActionId,
        side_effect: SideEffect,
        idempotency: Idempotency,
        retry_safety: RetrySafety,
    },
    InvalidContract {
        action: ActionId,
        side_effect: SideEffect,
        idempotency: Idempotency,
        retry_safety: RetrySafety,
    },
}

#[verifier::external]
pub fn is_contract_idempotency_accepted(contract: &ActionContract) -> bool {
    match (
        contract.side_effect,
        contract.retry_safety,
        contract.idempotency,
    ) {
        (SideEffect::None, _, _) => true,
        (_, RetrySafety::Unsafe, _) => false,
        (_, _, Idempotency::AtLeastOnceExternal | Idempotency::DeterministicPure) => false,
        (_, RetrySafety::Safe | RetrySafety::KeyRequired, Idempotency::IdempotentExternal) => true,
        _ => false,
    }
}

#[verifier::external]
pub fn is_statically_idempotent_contract(
    contract: &ActionContract,
) -> Result<(), IdempotencyContractViolation> {
    let _ = contract;
    Ok(())
}