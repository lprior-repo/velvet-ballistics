// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for idempotency certificate summary
// ============================================================================
//
// This file is a VERBATIM mirror of the production idempotency
// certificate decision fns.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

#[derive(Clone, Copy)]
pub enum SideEffectClass {
    None,
    Local,
    External,
    IdempotentExternal,
}

#[derive(Clone, Copy)]
pub enum RetrySafetyClass {
    Pure,
    AtLeastOnce,
    Idempotent,
}

#[derive(Clone, Copy)]
pub enum IdempotencyClass {
    None,
    Keyed,
    Attested,
}

#[verifier::external]
pub fn requires_idempotency_key(side_effect: SideEffectClass) -> bool {
    match side_effect {
        SideEffectClass::External | SideEffectClass::IdempotentExternal => true,
        _ => false,
    }
}

#[verifier::external]
pub fn is_contract_idempotency_accepted(
    side_effect: SideEffectClass,
    idempotency: IdempotencyClass,
) -> bool {
    match side_effect {
        SideEffectClass::None => true,
        SideEffectClass::IdempotentExternal => matches!(idempotency, IdempotencyClass::Attested),
        SideEffectClass::External => matches!(
            idempotency,
            IdempotencyClass::Keyed | IdempotencyClass::Attested
        ),
        SideEffectClass::Local => true,
    }
}

#[verifier::external]
pub fn storage_certificate_accepts_action(
    side_effect: SideEffectClass,
    idempotency: IdempotencyClass,
    certificate_keyed: bool,
    certificate_attested: bool,
) -> bool {
    let _ = (side_effect, idempotency, certificate_keyed, certificate_attested);
    false
}

#[verifier::external]
pub fn runtime_missing_idempotency_attestation(
    certificate_keyed: bool,
    certificate_attested: bool,
) -> bool {
    certificate_keyed && !certificate_attested
}