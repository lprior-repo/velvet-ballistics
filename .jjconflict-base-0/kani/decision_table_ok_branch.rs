//! Kani harness: verify Ok(()) branch for all valid Ok combinations.
//!
//! Ok when: side_effect==None OR
//!          (side_effect!=None AND idempotency==IdempotentExternal AND
//!           retry_safety in {Safe, KeyRequired})
//!
//! Obligation: KANI-DECISION-002
//! Requirement: POST-001

#![forbid(unsafe_code)]

use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::ActionId;
use vb_validate::idempotency_contract::is_statically_idempotent_contract;

/// KANI-DECISION-002: Ok branch for all valid Ok combinations.
///
/// Covers all 13 Ok combinations:
/// - side_effect==None: 9 combinations (3 RetrySafety × 3 Idempotency)
/// - side_effect!=None AND idempotency==IdempotentExternal AND retry_safety in {Safe, KeyRequired}: 4 combinations
#[kani::proof]
#[kani::unwind(6)]
fn decision_table_ok_branch() {
    // CASE A: side_effect == None — 9 combinations
    let retry_safeties = [RetrySafety::Safe, RetrySafety::KeyRequired, RetrySafety::Unsafe];
    let idempotencies = [
        Idempotency::DeterministicPure,
        Idempotency::IdempotentExternal,
        Idempotency::AtLeastOnceExternal,
    ];

    let mut j = 0;
    while j < retry_safeties.len() {
        let retry_safety = retry_safeties[j];
        let mut k = 0;
        while k < idempotencies.len() {
            let idempotency = idempotencies[k];

            let contract = ActionContract {
                id: ActionId::new(0),
                input_slot_count: 0,
                output_slot_count: 0,
                max_input_bytes: 0,
                max_output_bytes: 0,
                timeout_ms: 0,
                side_effect: SideEffect::None,
                retry_safety,
                idempotency,
                required_capabilities: Box::new([]),
            };

            let result = is_statically_idempotent_contract(&contract);
            kani::assert(result.is_ok(), "side_effect==None must be Ok regardless of retry/idempotency");

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
    let side_effects_non_none = [SideEffect::Writes, SideEffect::Sends, SideEffect::Creates, SideEffect::Destroys];
    let safe_key_required = [RetrySafety::Safe, RetrySafety::KeyRequired];

    let mut i = 0;
    while i < side_effects_non_none.len() {
        let side_effect = side_effects_non_none[i];
        let mut r = 0;
        while r < safe_key_required.len() {
            let retry_safety = safe_key_required[r];

            let contract = ActionContract {
                id: ActionId::new(0),
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

            let result = is_statically_idempotent_contract(&contract);
            kani::assert(
                result.is_ok(),
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
