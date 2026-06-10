//! Kani harness: verify is_statically_idempotent_contract returns deterministic
//! result for all 45 combinations (5 SideEffect × 3 RetrySafety × 3 Idempotency).
//!
//! Obligation: KANI-DECISION-001
//! Requirement: INV-003

#![forbid(unsafe_code)]

use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::ActionId;
use vb_validate::idempotency_contract::is_statically_idempotent_contract;

/// KANI-DECISION-001: All 45 combinations produce a deterministic Ok/Err result.
///
/// Enumerates all valid enum combinations and verifies the function returns
/// consistently across multiple calls with the same input.
#[kani::proof]
#[kani::unwind(6)]
fn is_statically_idempotent_contract() {
    // Enumerate all SideEffect variants
    let side_effects = [
        SideEffect::None,
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ];

    // Enumerate all RetrySafety variants
    let retry_safeties = [RetrySafety::Idempotent, RetrySafety::RequiresIdempotencyKey, RetrySafety::NotRetrySafe];

    // Enumerate all Idempotency variants
    let idempotencies = [
        Idempotency::DeterministicPure,
        Idempotency::IdempotentExternal,
        Idempotency::AtLeastOnceExternal,
    ];

    // Iterating over all 45 combinations (5 × 3 × 3)
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
                    id: ActionId::new(0), // constant; result must be independent of id
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

                // First call
                let result1 = is_statically_idempotent_contract(&contract);
                // Second call — must be identical (determinism)
                let result2 = is_statically_idempotent_contract(&contract);

                // Property: same Ok/Err result on repeated calls
                kani::assert(
                    result1.is_ok() == result2.is_ok(),
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
