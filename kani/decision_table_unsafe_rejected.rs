//! Kani harness: verify Err(SideEffectingRetryUnsafe) when
//! side_effect!=None AND retry_safety==Unsafe (regardless of idempotency).
//!
//! Obligation: KANI-DECISION-003
//! Requirement: POST-002

#![forbid(unsafe_code)]

use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::ActionId;
use vb_validate::idempotency_contract::is_statically_idempotent_contract;

/// KANI-DECISION-003: Err(SideEffectingRetryUnsafe) when side_effect!=None AND retry_safety==Unsafe.
///
/// All 15 combinations (5 non-None side_effects × 3 idempotency variants) must reject.
#[kani::proof]
#[kani::unwind(6)]
fn decision_table_unsafe_rejected() {
    let side_effects_non_none = [
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
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

            let result = is_statically_idempotent_contract(&contract);
            kani::assert(result.is_err(), "side_effect!=None with Unsafe must be Err");
            // Must be specifically SideEffectingRetryUnsafe
            if let Err(err) = &result {
                let reason = err.reason_category();
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
