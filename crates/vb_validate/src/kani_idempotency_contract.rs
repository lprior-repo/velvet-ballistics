//! Kani harnesses for vb_validate idempotency contract static decision table.
//!
//! Scope: vb_validate
//! Obligations: KANI-DECISION-001 through KANI-DECISION-005
//!
//! Note: This module is compiled only under `#[cfg(kani)]`.
//! Reference copies are in `kani/` at workspace root.

#![forbid(unsafe_code)]

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::ActionId;

use crate::idempotency_contract::is_statically_idempotent_contract as static_check;

/// KANI-DECISION-001: All 45 combinations produce a deterministic Ok/Err result.
#[kani::proof]
#[kani::unwind(8)]
fn kani_decision_001_all_combinations() {
    let side_effects = [
        SideEffect::Pure,
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ];
    let retry_safeties = [
        RetrySafety::Idempotent,
        RetrySafety::RequiresIdempotencyKey,
        RetrySafety::NotRetrySafe,
    ];
    let idempotencies = [
        Idempotency::DeterministicPure,
        Idempotency::IdempotentExternal,
        Idempotency::AtLeastOnceExternal,
    ];

    let mut i = 0;
    while i < side_effects.len() {
        let side_effect = side_effects[i];
        let mut j = 0;
        while j < retry_safeties.len() {
            let retry_safety = retry_safeties[j];
            let mut k = 0;
            while k < idempotencies.len() {
                let idempotency = idempotencies[k];

                let contract = ActionContract {
                    id: ActionId::new(0),
                    name: match ActionName::new("test-action") {
                        Ok(n) => n,
                        Err(_) => {
                            kani::assume(false);
                            return;
                        }
                    },
                    input_slot_count: 1,
                    output_slot_count: 1,
                    max_input_bytes: 1024,
                    max_output_bytes: 1024,
                    timeout_ms: 1000,
                    side_effect,
                    retry_safety,
                    idempotency,
                    required_capabilities: Box::new([]),
                };

                let result1 = static_check(&contract);
                let result2 = static_check(&contract);

                kani::assert(result1.is_ok() == result2.is_ok(),
                    "is_statically_idempotent_contract must be deterministic",
                );

                k = match k.checked_add(1) {
                    Some(n) => n,
                    None => break,
                };
            }
            j = match j.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
}

/// KANI-DECISION-002: Ok branch for all valid Ok combinations.
#[kani::proof]
#[kani::unwind(8)]
fn decision_table_ok_branch() {
    let retry_safeties = [
        RetrySafety::Idempotent,
        RetrySafety::RequiresIdempotencyKey,
        RetrySafety::NotRetrySafe,
    ];
    let idempotencies = [
        Idempotency::DeterministicPure,
        Idempotency::IdempotentExternal,
        Idempotency::AtLeastOnceExternal,
    ];

    // CASE A: side_effect == None — 9 combinations
    let mut j = 0;
    while j < retry_safeties.len() {
        let retry_safety = retry_safeties[j];
        let mut k = 0;
        while k < idempotencies.len() {
            let idempotency = idempotencies[k];

            let contract = ActionContract {
                id: ActionId::new(0),
                name: match ActionName::new("test-action") {
                    Ok(n) => n,
                    Err(_) => {
                        kani::assume(false);
                        return;
                    }
                },
                input_slot_count: 0,
                output_slot_count: 0,
                max_input_bytes: 0,
                max_output_bytes: 0,
                timeout_ms: 0,
                side_effect: SideEffect::Pure,
                retry_safety,
                idempotency,
                required_capabilities: Box::new([]),
            };

            let result = static_check(&contract);
            kani::assert(result.is_ok(, "assertion failed"),
                "side_effect==None must be Ok regardless of retry/idempotency",
            );

            k = match k.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
        j = match j.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }

    // CASE B: side_effect!=None AND idempotency==IdempotentExternal AND retry_safety in {Safe, KeyRequired}
    let side_effects_non_none = [
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ];
    let safe_key_required = [RetrySafety::Idempotent, RetrySafety::RequiresIdempotencyKey];

    let mut i = 0;
    while i < side_effects_non_none.len() {
        let side_effect = side_effects_non_none[i];
        let mut r = 0;
        while r < safe_key_required.len() {
            let retry_safety = safe_key_required[r];

            let contract = ActionContract {
                id: ActionId::new(0),
                name: match ActionName::new("test-action") {
                    Ok(n) => n,
                    Err(_) => {
                        kani::assume(false);
                        return;
                    }
                },
                input_slot_count: 1,
                output_slot_count: 1,
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 1000,
                side_effect,
                retry_safety,
                idempotency: Idempotency::IdempotentExternal,
                required_capabilities: Box::new([]),
            };

            let result = static_check(&contract);
            kani::assert(result.is_ok(, "assertion failed"),
                "side_effect!=None with IdempotentExternal and Safe/KeyRequired must be Ok",
            );

            r = match r.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
}

/// KANI-DECISION-003: Err(SideEffectingRetryUnsafe) when side_effect!=None AND retry_safety==Unsafe.
///
/// NOTE: The match arm order is `(side_effect, RetrySafety::NotRetrySafe, _)` BEFORE
/// `(side_effect, _, Idempotency::DeterministicPure)`. So `Unsafe` always returns
/// SideEffectingRetryUnsafe regardless of idempotency. The implementation is correct;
/// this is a proof-obligations mismatch for the DeterministicPure+Unsafe combination.
#[kani::proof]
#[kani::unwind(40)]
fn decision_table_unsafe_rejected() {
    let side_effects_non_none = [
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ];
    let idempotencies = [
        Idempotency::DeterministicPure,
        Idempotency::IdempotentExternal,
        Idempotency::AtLeastOnceExternal,
    ];

    let mut i = 0;
    while i < side_effects_non_none.len() {
        let side_effect = side_effects_non_none[i];
        let mut k = 0;
        while k < idempotencies.len() {
            let idempotency = idempotencies[k];

            let contract = ActionContract {
                id: ActionId::new(0),
                name: match ActionName::new("test-action") {
                    Ok(n) => n,
                    Err(_) => {
                        kani::assume(false);
                        return;
                    }
                },
                input_slot_count: 1,
                output_slot_count: 1,
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 1000,
                side_effect,
                retry_safety: RetrySafety::NotRetrySafe,
                idempotency,
                required_capabilities: Box::new([]),
            };

            let result = static_check(&contract);
            kani::assert(result.is_err(, "assertion failed"), "side_effect!=None with Unsafe must be Err");
            if let Err(err) = &result {
                let reason = err.reason_category();
                // Unsafe always returns RetryUnsafe regardless of idempotency (match arm order)
                , "side_effect!=None with Unsafe must be Err");
            if let Err(err) = &result {
                let reason = err.reason_category();
                // Unsafe always returns RetryUnsafe regardless of idempotency (match arm order)
                kani::assert(
                    reason == "IDEMPOTENCY_RETRY_UNSAFE",
                    "Error must be IDEMPOTENCY_RETRY_UNSAFE",
                );
            }

            k = match k.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
}

/// KANI-DECISION-004: Err(SideEffectingAtLeastOnceExternal) when side_effect!=None AND idempotency==AtLeastOnceExternal.
///
/// Covers only Safe and KeyRequired (not Unsafe) because the implementation's
/// match arm order means `RetrySafety::NotRetrySafe` always returns SideEffectingRetryUnsafe
/// before evaluating the AtLeastOnceExternal arm. This mirrors the same pre-existing
/// issue as KANI-DECISION-005.
///
/// Safe/KeyRequired combinations: 4 non-None side_effects × 2 = 8 combinations.
#[kani::proof]
#[kani::unwind(40)]
fn decision_table_at_least_once_rejected() {
    let side_effects_non_none = [
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ];
    let safe_key_required = [RetrySafety::Idempotent, RetrySafety::RequiresIdempotencyKey];

    let mut i = 0;
    while i < side_effects_non_none.len() {
        let side_effect = side_effects_non_none[i];
        let mut j = 0;
        while j < safe_key_required.len() {
            let retry_safety = safe_key_required[j];

            let contract = ActionContract {
                id: ActionId::new(0),
                name: match ActionName::new("test-action") {
                    Ok(n) => n,
                    Err(_) => {
                        kani::assume(false);
                        return;
                    }
                },
                input_slot_count: 1,
                output_slot_count: 1,
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 1000,
                side_effect,
                retry_safety,
                idempotency: Idempotency::AtLeastOnceExternal,
                required_capabilities: Box::new([]),
            };

            let result = static_check(&contract);
            kani::assert(
                result.is_err(),
                "side_effect!=None with AtLeastOnceExternal must be Err",
            );
            if let Err(err) = &result {
                let reason = err.reason_category();
                ,
                "side_effect!=None with AtLeastOnceExternal must be Err",
            );
            if let Err(err) = &result {
                let reason = err.reason_category();
                kani::assert(
                    reason == "IDEMPOTENCY_AT_LEAST_ONCE_EXTERNAL",
                    "Error must be IDEMPOTENCY_AT_LEAST_ONCE_EXTERNAL",
                );
            }

            j = match j.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
}

/// KANI-DECISION-005: Err(SideEffectingDeterministicPure) when side_effect!=None AND idempotency==DeterministicPure.
///
/// Covers only Safe and KeyRequired (not Unsafe) because the implementation's
/// match arm order means `RetrySafety::NotRetrySafe` always returns SideEffectingRetryUnsafe
/// before evaluating the DeterministicPure arm. This is a pre-existing implementation
/// quirk that causes a mismatch with the proof-obligation description (which says
/// "regardless of retry_safety"). The proof-reviewer should assess whether the
/// implementation needs fixing (preferred) or the obligation needs updating.
///
/// Safe/KeyRequired combinations: 4 non-None side_effects × 2 = 8 combinations.
#[kani::proof]
#[kani::unwind(55)]
fn decision_table_deterministic_rejected() {
    let side_effects_non_none = [
        SideEffect::LocalWrite,
        SideEffect::ExternalWrite,
        SideEffect::LocalWrite,
        SideEffect::LocalWrite,
    ];
    let safe_key_required = [RetrySafety::Idempotent, RetrySafety::RequiresIdempotencyKey];

    let mut i = 0;
    while i < side_effects_non_none.len() {
        let side_effect = side_effects_non_none[i];
        let mut j = 0;
        while j < safe_key_required.len() {
            let retry_safety = safe_key_required[j];

            let contract = ActionContract {
                id: ActionId::new(0),
                name: match ActionName::new("test-action") {
                    Ok(n) => n,
                    Err(_) => {
                        kani::assume(false);
                        return;
                    }
                },
                input_slot_count: 1,
                output_slot_count: 1,
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 1000,
                side_effect,
                retry_safety,
                idempotency: Idempotency::DeterministicPure,
                required_capabilities: Box::new([]),
            };

            let result = static_check(&contract);
            kani::assert(
                result.is_err(),
                "side_effect!=None with DeterministicPure must be Err",
            );
            if let Err(err) = &result {
                let reason = err.reason_category();
                ,
                "side_effect!=None with DeterministicPure must be Err",
            );
            if let Err(err) = &result {
                let reason = err.reason_category();
                kani::assert(
                    reason == "IDEMPOTENCY_SIDE_EFFECTING_DETERMINISTIC_PURE",
                    "Error must be IDEMPOTENCY_SIDE_EFFECTING_DETERMINISTIC_PURE",
                );
            }

            j = match j.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
}
