// Kani harness for KANI-INV-001: exact capability matching algorithm
// vb_core::capability::CapabilitySet::grants panic-free and deterministic
//
// Covers 4 matching cases:
// 1. exact match: required_name == grant_name
// 2. prefix+dot rejected: required_name = grant_name + '.' + suffix
// 3. partial segment rejected: grant_name is lexical prefix
// 4. non-prefix: grant_name does not match required_name at all

#![forbid(unsafe_code)]

use crate::capability::{Capability, CapabilitySet};
use crate::ids::ActionId;

#[cfg(kani)]
mod kani_capability_harnesses {
    use super::*;

    #[kani::proof]
    fn capability_name_grants_harness() {
        let grant_name: [u8; 32] = kani::any();
        let required_name: [u8; 32] = kani::any();

        let grant_lossy = String::from_utf8_lossy(&grant_name);
        let grant_str = match grant_lossy.split('\0').next() {
            Some(value) => value,
            None => "",
        };
        let required_lossy = String::from_utf8_lossy(&required_name);
        let required_str = match required_lossy.split('\0').next() {
            Some(value) => value,
            None => "",
        };

        let action: u16 = kani::any();
        let action_id = ActionId::new(action);

        let cap = Capability::new(grant_str.into(), action_id);
        let required = Capability::new(required_str.into(), action_id);
        let set = CapabilitySet::from_grants(Box::new([cap]));

        let _result = set.grants(&required);

        let empty_cap = Capability::new("".into(), action_id);
        let empty_set = CapabilitySet::from_grants(Box::new([empty_cap]));
        let _ = empty_set.grants(&required);
    }

    #[kani::proof]
    fn capability_name_grants_exact_match_case() {
        let action_id = ActionId::new(1);

        let cap = Capability::new("network".into(), action_id);
        let required = Capability::new("network".into(), action_id);
        let set = CapabilitySet::from_grants(Box::new([cap]));

        assert!(set.grants(&required));
    }

    #[kani::proof]
    fn capability_name_rejects_prefix_dot_case() {
        let action_id = ActionId::new(1);

        let cap = Capability::new("network".into(), action_id);
        let required = Capability::new("network.github".into(), action_id);
        let set = CapabilitySet::from_grants(Box::new([cap]));

        assert!(!set.grants(&required));
    }

    #[kani::proof]
    fn capability_name_grants_partial_segment_rejected() {
        let action_id = ActionId::new(1);

        let cap = Capability::new("net".into(), action_id);
        let required = Capability::new("network".into(), action_id);
        let set = CapabilitySet::from_grants(Box::new([cap]));

        assert!(!set.grants(&required));
    }

    #[kani::proof]
    fn capability_name_grants_non_prefix_rejected() {
        let action_id = ActionId::new(1);

        let cap = Capability::new("storage".into(), action_id);
        let required = Capability::new("network".into(), action_id);
        let set = CapabilitySet::from_grants(Box::new([cap]));

        assert!(!set.grants(&required));
    }

    #[kani::proof]
    fn capability_name_empty_grant_rejected() {
        let action_id = ActionId::new(1);

        let cap = Capability::new("".into(), action_id);
        let required = Capability::new("network".into(), action_id);
        let set = CapabilitySet::from_grants(Box::new([cap]));

        assert!(!set.grants(&required));
    }

    #[kani::proof]
    fn capability_name_action_mismatch_rejected() {
        let cap = Capability::new("network".into(), ActionId::new(2));
        let required = Capability::new("network".into(), ActionId::new(1));
        let set = CapabilitySet::from_grants(Box::new([cap]));

        assert!(!set.grants(&required));
    }
}
