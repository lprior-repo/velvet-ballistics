//! Kani harness: verify Err(SideEffectingDeterministicPure) when
//! side_effect!=None AND idempotency==DeterministicPure (regardless of retry_safety).
//!
//! Obligation: KANI-DECISION-005
//! Requirement: POST-004

#![forbid(unsafe_code)]

use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::ActionId;
use vb_validate::idempotency_contract::is_statically_idempotent_contract;

/// KANI-DECISION-005: Err(SideEffectingDeterministicPure) when side_effect!=None AND idempotency==DeterministicPure.
///
/// All 15 combinations (5 non-None side_effects × 3 RetrySafety variants) must reject.
#[kani::proof]
#[kani::unwind(6)]
fn decision_table_deterministic_rejected() {
    let side_effects_non_none = [
        SideEffect::Writes,
        SideEffect::Sends,
        SideEffect::Creates,
        SideEffect::Destroys,
    ];
    let retry_safeties = [RetrySafety::Safe, RetrySafety::KeyRequired, RetrySafety::Unsafe];

    let mut i = 0;
    while i < side_effects_non_none.len() {
        let side_effect = side_effects_non_none[i];
        let mut j = 0;
        while j < retry_safeties.len() {
            let retry_safety = retry_safeties[j];

            let contract = ActionContract {
                id: ActionId::new(0),
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

            let result = is_statically_idempotent_contract(&contract);
            kani::assert(
                result.is_err(),
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
