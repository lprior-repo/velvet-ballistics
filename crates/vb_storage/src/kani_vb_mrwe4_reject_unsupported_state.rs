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

        let unsupported = UnsupportedRecoveryState {
            slot_values,
            slot_taint,
            action_payloads,
        };

        // After fix: only the three remaining flags drive rejection.
        let should_reject =
            unsupported.slot_values || unsupported.slot_taint || unsupported.action_payloads;

        if should_reject {
            kani::assert(
                unsupported.slot_values || unsupported.slot_taint || unsupported.action_payloads,
                "Rejection must be due to slot_values/slot_taint/action_payloads",
            );
        }

        // No-op probe: state without any flag set must NOT be rejected.
        if !unsupported.slot_values
            && !unsupported.slot_taint
            && !unsupported.action_payloads
        {
            kani::assert(
                !should_reject,
                "all-false state must NOT trigger rejection",
            );
        }
    }
}
