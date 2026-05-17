// Kani harness for KAN-CHECK-CAP-001: check_capability action match/mismatch and name grant/deny
// Verifies no UB, no panic, and Ok or Err(CapabilityDenied) for all combinations

#![forbid(unsafe_code)]

use crate::admission::{AdmissionError, check_capability};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::ActionId;

#[cfg(kani)]
mod kani_capability_harnesses {
    use super::*;

    #[kani::proof]
    fn check_capability_harness() {
        let req_action: u16 = kani::any();
        let grant_action: u16 = kani::any();
        let req_action_id = ActionId::new(req_action);
        let grant_action_id = ActionId::new(grant_action);

        let req_name: [u8; 16] = kani::any();
        let grant_name: [u8; 16] = kani::any();
        let req_name_lossy = String::from_utf8_lossy(&req_name);
        let req_name_str = match req_name_lossy.split('\0').next() {
            Some(value) => value,
            None => "cap",
        };
        let grant_name_lossy = String::from_utf8_lossy(&grant_name);
        let grant_name_str = match grant_name_lossy.split('\0').next() {
            Some(value) => value,
            None => "cap",
        };

        let required = Capability::new(req_name_str.into(), req_action_id);
        let grant = Capability::new(grant_name_str.into(), grant_action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(req_action_id, &required, &granted);

        match result {
            Ok(()) => {}
            Err(AdmissionError::CapabilityDenied { .. }) => {}
            Err(_) => {
                kani::assert(false, "Only CapabilityDenied expected for denied cases");
            }
        }
    }

    #[kani::proof]
    fn check_capability_grants_exact_match() {
        let action_id = ActionId::new(7);
        let required = Capability::new("action".into(), action_id);
        let exact =
            CapabilitySet::from_grants(Box::new([Capability::new("action".into(), action_id)]));

        kani::assert(
            check_capability(action_id, &required, &exact).is_ok(),
            "exact grant is accepted",
        );
    }

    #[kani::proof]
    fn check_capability_action_match_name_grants() {
        let action_id = ActionId::new(1);
        let required = Capability::new("network".into(), action_id);
        let grant = Capability::new("network".into(), action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(result.is_ok(), "action match + name grants → Ok");
    }

    #[kani::proof]
    fn check_capability_action_match_name_denies() {
        let action_id = ActionId::new(1);
        let required = Capability::new("secrets".into(), action_id);
        let grant = Capability::new("network".into(), action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(
            matches!(&result, Err(AdmissionError::CapabilityDenied { .. })),
            "action match + name denies -> CapabilityDenied",
        );
        std::mem::forget(result);
    }

    #[kani::proof]
    fn check_capability_action_mismatch_name_grants() {
        let action_id = ActionId::new(1);
        let required = Capability::new("network".into(), action_id);
        let grant = Capability::new("network".into(), ActionId::new(99));
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(
            matches!(&result, Err(AdmissionError::CapabilityDenied { .. })),
            "action mismatch -> CapabilityDenied regardless of name",
        );
        std::mem::forget(result);
    }

    #[kani::proof]
    fn check_capability_action_mismatch_name_denies() {
        let action_id = ActionId::new(1);
        let required = Capability::new("secrets".into(), action_id);
        let grant = Capability::new("network".into(), ActionId::new(99));
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(
            matches!(&result, Err(AdmissionError::CapabilityDenied { .. })),
            "action mismatch + name denies -> CapabilityDenied",
        );
        std::mem::forget(result);
    }

    #[kani::proof]
    fn check_capability_hierarchical_rejects_subpath() {
        let action_id = ActionId::new(1);
        let required = Capability::new("network.api".into(), action_id);
        let grant = Capability::new("network".into(), action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(
            matches!(&result, Err(AdmissionError::CapabilityDenied { .. })),
            "prefix grant must not satisfy subpath requirement",
        );
        std::mem::forget(result);
    }

    #[kani::proof]
    fn check_capability_partial_segment_rejected() {
        let action_id = ActionId::new(1);
        let required = Capability::new("network".into(), action_id);
        let grant = Capability::new("net".into(), action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(
            matches!(&result, Err(AdmissionError::CapabilityDenied { .. })),
            "partial segment must not grant",
        );
        std::mem::forget(result);
    }
}
