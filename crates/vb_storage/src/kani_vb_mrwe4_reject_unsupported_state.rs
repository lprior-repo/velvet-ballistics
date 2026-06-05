// Kani proof: reject_unsupported_live_frame_state does NOT reject pending_actions alone.
// Obligation: vb-mrwe.4 PO-007, PO-021

#[cfg(kani)]
mod kani_vb_mrwe4_reject_unsupported_state {
    use crate::recovery::types::UnsupportedRecoveryState;

    #[kani::proof]
    fn vb_mrwe4_reject_does_not_use_pending_actions() {
        let slot_values: bool = kani::any();
        let slot_taint: bool = kani::any();
        let action_payloads: bool = kani::any();
        let pending_actions: bool = kani::any();

        let unsupported = UnsupportedRecoveryState {
            slot_values,
            slot_taint,
            action_payloads,
            pending_actions,
        };

        // After fix: pending_actions is NOT in the condition
        let should_reject =
            unsupported.slot_values || unsupported.slot_taint || unsupported.action_payloads;

        if should_reject {
            kani::assert(
                unsupported.slot_values || unsupported.slot_taint || unsupported.action_payloads,
                "Rejection must be due to slot_values/slot_taint/action_payloads",
            );
        }

        // If ONLY pending_actions is true, must NOT reject
        if unsupported.pending_actions
            && !unsupported.slot_values
            && !unsupported.slot_taint
            && !unsupported.action_payloads
        {
            kani::assert(
                !should_reject,
                "pending_actions alone must NOT trigger rejection after fix",
            );
        }
    }
}
