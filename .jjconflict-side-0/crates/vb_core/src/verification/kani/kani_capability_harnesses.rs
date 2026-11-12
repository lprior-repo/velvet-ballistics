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
        let action_id = ActionId::new(7);
        let exact =
            CapabilitySet::from_grants(Box::new([Capability::new("action".into(), action_id)]));
        let prefix =
            CapabilitySet::from_grants(Box::new([Capability::new("action".into(), action_id)]));
        let partial =
            CapabilitySet::from_grants(Box::new([Capability::new("act".into(), action_id)]));
        let sibling =
            CapabilitySet::from_grants(Box::new([Capability::new("storage".into(), action_id)]));
        let wrong_action = CapabilitySet::from_grants(Box::new([Capability::new(
            "action".into(),
            ActionId::new(8),
        )]));
        let required = Capability::new("action".into(), action_id);
        let child_required = Capability::new("action.dispatch".into(), action_id);

        kani::assert(exact.grants(&required), "exact name and action grants");
        kani::assert(
            !prefix.grants(&child_required),
            "parent prefix does not grant child capability",
        );
        kani::assert(
            !partial.grants(&required),
            "partial lexical prefix does not grant",
        );
        kani::assert(!sibling.grants(&required), "sibling name does not grant");
        kani::assert(
            !wrong_action.grants(&required),
            "matching name with wrong action does not grant",
        );
        kani::assert(
            !CapabilitySet::empty().grants(&required),
            "empty set does not grant",
        );
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
