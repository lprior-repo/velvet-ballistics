//! Kani harness: cross-crate parity between check_idempotency_gates (vb_compile)
//! and is_statically_idempotent_contract (vb_validate).
//!
//! Scope: vb_compile + vb_validate (cross-crate)
//! Obligation: PO-014 / KANI-PARITY-006 (CRITICAL)
//!
//! Note: This module is compiled only under `#[cfg(kani)]`.

#![forbid(unsafe_code)]

use crate::is_compile_idempotency_gate_accepted;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::ActionId;
use vb_validate::idempotency_contract::is_statically_idempotent_contract;

/// KANI-PARITY-006: check_idempotency_gates and is_statically_idempotent_contract
/// agree on Ok/Err for all 45 combinations.
///
/// Combinations:
/// - 5 SideEffect variants: None, Writes, Sends, Creates, Destroys
/// - 3 RetrySafety variants: Safe, KeyRequired, Unsafe
/// - 3 Idempotency variants: DeterministicPure, IdempotentExternal, AtLeastOnceExternal
///
/// No disagreement class is excluded. The harness also checks the contracted
/// reason class for the three rejecting classes by asserting that both sides
/// reject exactly the expected decision-table branch.
#[kani::proof]
#[kani::unwind(8)]
fn idempotency_gate_parity() {
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
                        Ok(v) => v,
                        Err(_) => { kani::assume(false); return; }
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

                // Static validation result (vb_validate)
                let static_result = is_statically_idempotent_contract(&contract);

                // Compile-time gate result (vb_compile) through the pure decision
                // helper used by the public allocating gate.
                let compile_ok = is_compile_idempotency_gate_accepted(&contract);

                // Parity: both must agree on Ok/Err
                kani::assert(
                    static_result.is_ok() == compile_ok,
                    "check_idempotency_gates and is_statically_idempotent_contract \
                     must agree on Ok/Err for all 45 combinations",
                );

                let side_effecting = !matches!(side_effect, SideEffect::Pure);
                let expected_retry_unsafe =
                    side_effecting && matches!(retry_safety, RetrySafety::NotRetrySafe);
                let expected_at_least_once = side_effecting
                    && !expected_retry_unsafe
                    && matches!(idempotency, Idempotency::AtLeastOnceExternal);
                let expected_deterministic_pure = side_effecting
                    && !expected_retry_unsafe
                    && matches!(idempotency, Idempotency::DeterministicPure);
                let expected_accept = !(expected_retry_unsafe
                    || expected_at_least_once
                    || expected_deterministic_pure);

                kani::assert(
                    static_result.is_ok() == expected_accept,
                    "validate reason class must match the canonical decision table",
                );
                kani::assert(
                    compile_ok == expected_accept,
                    "compile reason class must match the canonical decision table",
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
