//! Kani harness: cross-crate parity between check_idempotency_gates (vb_compile)
//! and is_statically_idempotent_contract (vb_validate).
//!
//! Both must agree on Ok/Err result for all 45 (side_effect, retry_safety, idempotency)
//! combinations.
//!
//! Obligation: KANI-PARITY-001 (CRITICAL)
//! Requirement: POST-010
//!
//! NOTE: This harness verifies parity on the basic Ok/Err dimension. The two
//! functions return different error types (CompileError vs IdempotencyContractViolation),
//! so only the boolean acceptance result is compared. Error-variant parity would
//! require additional normalization that is out of scope for this obligation.

#![forbid(unsafe_code)]

use vb_compile::check_idempotency_gates;
use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::ActionId;
use vb_validate::idempotency_contract::is_statically_idempotent_contract;

/// KANI-PARITY-001: check_idempotency_gates and is_statically_idempotent_contract
/// agree on Ok/Err for all 45 combinations.
///
/// Combinations:
/// - 5 SideEffect variants: None, Writes, Sends, Creates, Destroys
/// - 3 RetrySafety variants: Safe, KeyRequired, Unsafe
/// - 3 Idempotency variants: DeterministicPure, IdempotentExternal, AtLeastOnceExternal
#[kani::proof]
#[kani::unwind(8)]
fn idempotency_gate_parity() {
    let side_effects = [
        SideEffect::None,
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ];
    let retry_safeties = [RetrySafety::Safe, RetrySafety::KeyRequired, RetrySafety::Unsafe];
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

                // Static validation result
                let static_result = is_statically_idempotent_contract(&contract);

                // Compile-time gate result — wrap single contract in slice
                let contracts = [contract];
                let compile_result = check_idempotency_gates(&contracts);

                // Parity: both must agree on Ok/Err
                // (error variant types differ: IdempotencyContractViolation vs CompileError)
                kani::assert(
                    static_result.is_ok() == compile_result.is_ok(),
                    "check_idempotency_gates and is_statically_idempotent_contract \
                     must agree on Ok/Err for all 45 combinations",
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
