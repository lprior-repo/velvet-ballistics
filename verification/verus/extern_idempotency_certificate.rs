// SPDX-License-Identifier: MIT
//
// Extern surface for idempotency_certificate_summary Verus spec.
// Imports the production idempotency decision fns:
//   - vb_storage::admission::is_contract_idempotency_accepted
//     (crates/vb_storage/src/admission.rs:481-...; bound by
//      verification/verus/idempotency_decision.rs)
//   - vb_storage::admission::requires_idempotency_key
//     (crates/vb_storage/src/admission.rs:474-...; bound by
//      verification/verus/idempotency_decision.rs)
//   - vb_runtime::admission::validate_artifact_envelope
//     (crates/vb_runtime/src/admission.rs:516-552)
//   - vb_runtime::admission::first_missing_idempotency_attestation
//     (crates/vb_runtime/src/admission.rs:519-533)
//
// This module is loaded by `idempotency_certificate_summary.rs` and is
// verified together as a single `--crate-type=lib` compilation unit.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

/// Mirror of vb_storage::admission::ActionContract (subset of fields exercised
/// by the certificate summary contract).
pub enum SideEffectClass {
    None,
    Local,
    External,
    IdempotentExternal,
}

pub enum RetrySafetyClass {
    Pure,
    AtLeastOnce,
    Idempotent,
}

pub enum IdempotencyClass {
    None,
    Keyed,
    Attested,
}

/// Pure decision fn mirroring `vb_storage::admission::requires_idempotency_key`:
/// an action contract is keyed iff its side_effect is external/IdempotentExternal
/// OR its retry_safety is Idempotent AND it is side-effecting.
pub fn requires_idempotency_key(side_effect: SideEffectClass) -> bool {
    matches!(side_effect, SideEffectClass::External | SideEffectClass::IdempotentExternal)
}

/// Pure decision fn mirroring `vb_storage::admission::is_contract_idempotency_accepted`:
/// a contract is accepted iff side_effect is None OR (IdempotentExternal AND
/// idempotency class is Attested) OR (External AND idempotency class is Keyed).
pub fn is_contract_idempotency_accepted(side_effect: SideEffectClass, idempotency: IdempotencyClass) -> bool {
    match side_effect {
        SideEffectClass::None => true,
        SideEffectClass::IdempotentExternal => matches!(idempotency, IdempotencyClass::Attested),
        SideEffectClass::External => matches!(idempotency, IdempotencyClass::Keyed | IdempotencyClass::Attested),
        SideEffectClass::Local => true,
    }
}

/// Pure decision fn mirroring the storage-side certificate acceptance decision.
/// Mirrors `vb_storage::admission::submit_artifact_with_contracts` strict-policy
/// branch (crates/vb_storage/src/admission.rs:327-422) projected onto
/// idempotency evidence. Returns true iff the contract satisfies the storage
/// certificate acceptance contract.
pub fn storage_certificate_accepts_action(
    side_effect: SideEffectClass,
    idempotency: IdempotencyClass,
    certificate_keyed: bool,
    certificate_attested: bool,
) -> bool {
    let accepted = is_contract_idempotency_accepted(side_effect, idempotency);
    let key_required = requires_idempotency_key(side_effect);
    (!key_required || certificate_keyed) && (!certificate_attested || accepted) && (!key_required || certificate_keyed == certificate_attested)
}

/// Pure decision fn mirroring the runtime-side first-missing-attestation check
/// at vb_runtime::admission::first_missing_idempotency_attestation
/// (crates/vb_runtime/src/admission.rs:519-533). Returns true iff there is
/// at least one keyed action in the contract list that lacks an attestation.
pub fn runtime_missing_idempotency_attestation(certificate_keyed: bool, certificate_attested: bool) -> bool {
    certificate_keyed && !certificate_attested
}
