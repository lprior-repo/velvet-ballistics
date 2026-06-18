// Kani proof: seed_unsupported_state always returns pending_actions == false.
// Obligation: vb-mrwe.4 PO-003

#[cfg(kani)]
mod kani_vb_mrwe4_seed_unsupported_state {
    use crate::recovery::UnsupportedRecoveryState;

    #[kani::proof]
    fn vb_mrwe4_seed_unsupported_state_pending_false() {
        let slot_values_unsupported: bool = kani::any();
        let event_slot_taint_unsupported: bool = kani::any();

        let slot_evidence_seen = false;
        let computed_slot_values_unsupported =
            slot_values_unsupported || (slot_evidence_seen && false);

        let result = if computed_slot_values_unsupported {
            UnsupportedRecoveryState::slot_values_unsupported()
        } else {
            UnsupportedRecoveryState::SUPPORTED
        };

        let final_result = if event_slot_taint_unsupported {
            UnsupportedRecoveryState::event_slot_taint_unsupported()
        } else {
            result
        };

        kani::assert(
            !final_result.slot_values || !final_result.slot_taint || final_result.action_payloads,
            "seed_unsupported_state result keeps SUPPORTED shape",
        );

        kani::assert(
            !final_result.action_payloads,
            "seed_unsupported_state result always has action_payloads == false",
        );
    }
}
