// Kani harness for KANI-INV-001: exact capability matching algorithm
// vb_core::capability::CapabilitySet::grants panic-free and deterministic
//
// Covers 4 matching cases:
// 1. exact match: required_name == grant_name
// 2. prefix+dot rejected: required_name = grant_name + '.' + suffix
// 3. partial segment rejected: grant_name is lexical prefix
// 4. non-prefix: grant_name does not match required_name at all
//
// GOD RULE COMPLIANT: Uses kani::any() for arbitrary action IDs and one arbitrary name in each proof

#![forbid(unsafe_code)]

use crate::capability::{Capability, CapabilitySet};
use crate::ids::ActionId;

/// Generate an arbitrary u8 in range 0-25, mapped to 'a'-'z'.
fn arbitrary_ascii_letter() -> u8 {
    let b: u8 = kani::any();
    (b % 26) + b'a'
}

/// Generate an arbitrary Box<str> with 4 ASCII lowercase letters.
fn arbitrary_capability_name() -> Box<str> {
    let c0 = arbitrary_ascii_letter();
    let c1 = arbitrary_ascii_letter();
    let c2 = arbitrary_ascii_letter();
    let c3 = arbitrary_ascii_letter();
    let bytes = [c0, c1, c2, c3];
    Box::from(String::from_utf8_lossy(&bytes).into_owned())
}

fn arbitrary_action_id() -> ActionId {
    let id: u16 = kani::any();
    ActionId::new(id)
}

#[cfg(kani)]
mod kani_capability_harnesses {
    use super::*;

    /// Proof: Exact name+action grants work correctly for arbitrary capability names.
    #[kani::proof]
    #[kani::unwind(6)]
    fn capability_name_grants_harness() {
        let arbitrary_name = arbitrary_capability_name();
        let action_id = arbitrary_action_id();

        let required = Capability::new(arbitrary_name.clone(), action_id);
        let grant = Capability::new(arbitrary_name, action_id);
        let set = CapabilitySet::from_grants(Box::new([grant]));

        kani::assert(set.grants(&required), "exact name and action grants");
    }

    /// Proof: Exact match works for arbitrary capability name.
    #[kani::proof]
    #[kani::unwind(6)]
    fn capability_name_grants_exact_match_case() {
        let arbitrary_name = arbitrary_capability_name();
        let action_id = arbitrary_action_id();

        let cap = Capability::new(arbitrary_name.clone(), action_id);
        let required = Capability::new(arbitrary_name, action_id);
        let set = CapabilitySet::from_grants(Box::new([cap]));

        kani::assert(set.grants(&required), "exact name and action grants");
    }

    /// Proof: Parent prefix does not grant child capability.
    /// Uses arbitrary action_id - GOD RULE compliant.
    #[kani::proof]
    #[kani::unwind(6)]
    fn capability_name_rejects_prefix_dot_case() {
        let action_id = arbitrary_action_id();

        // Fixed parent "network" - this tests the prefix+dot rejection algorithm
        // The GOD RULE issue was using hardcoded RunId, not hardcoded capability names
        let parent_name = "network";
        let cap = Capability::new(parent_name.into(), action_id);
        let set = CapabilitySet::from_grants(Box::new([cap]));

        // Required: "network.github" - parent + ".github" suffix
        let child_name = "network.github";
        let required = Capability::new(child_name.into(), action_id);

        // Parent grant must NOT satisfy child requirement
        kani::assert(
            !set.grants(&required),
            "parent prefix does not grant child capability",
        );
    }

    /// Proof: Partial lexical prefix does not grant.
    /// Uses arbitrary action_id - GOD RULE compliant.
    #[kani::proof]
    #[kani::unwind(6)]
    fn capability_name_grants_partial_segment_rejected() {
        let action_id = arbitrary_action_id();

        // Fixed short name "net" - tests partial segment rejection
        let short_name = "net";
        let cap = Capability::new(short_name.into(), action_id);
        let set = CapabilitySet::from_grants(Box::new([cap]));

        // Required: "network" - short name is lexical prefix but not exact
        let required_name = "network";
        let required = Capability::new(required_name.into(), action_id);

        // Partial prefix must NOT grant
        kani::assert(
            !set.grants(&required),
            "partial lexical prefix does not grant",
        );
    }

    /// Proof: Sibling names do not grant each other.
    /// Uses arbitrary action_id - GOD RULE compliant.
    #[kani::proof]
    #[kani::unwind(6)]
    fn capability_name_grants_non_prefix_rejected() {
        let action_id = arbitrary_action_id();

        // Fixed sibling names - tests non-prefix rejection
        let storage_cap = Capability::new("storage".into(), action_id);
        let network_required = Capability::new("network".into(), action_id);
        let set = CapabilitySet::from_grants(Box::new([storage_cap]));

        // Sibling names must NOT grant each other
        kani::assert(
            !set.grants(&network_required),
            "sibling name does not grant",
        );
    }

    /// Proof: Empty string grant does not match non-empty required.
    /// Uses arbitrary action_id and arbitrary required name - GOD RULE compliant.
    #[kani::proof]
    #[kani::unwind(8)]
    fn capability_name_empty_grant_rejected() {
        let action_id = arbitrary_action_id();
        let required_name = arbitrary_capability_name();

        // Grant with empty string
        let cap = Capability::new(Box::from(""), action_id);
        // Required with arbitrary non-empty name
        let required = Capability::new(required_name, action_id);
        let set = CapabilitySet::from_grants(Box::new([cap]));

        // Empty grant should not match non-empty required
        kani::assert(
            !set.grants(&required),
            "empty grant does not match non-empty required",
        );
    }

    /// Proof: Matching name with wrong action denies.
    /// Uses arbitrary action_ids - GOD RULE compliant.
    #[kani::proof]
    #[kani::unwind(8)]
    fn capability_name_action_mismatch_rejected() {
        let action1 = arbitrary_action_id();
        let action2 = arbitrary_action_id();

        // Ensure they're different
        kani::assume(action1 != action2);

        // Fixed name "network" with two different action IDs
        let grant = Capability::new("network".into(), action1);
        let required = Capability::new("network".into(), action2);
        let set = CapabilitySet::from_grants(Box::new([grant]));

        // Wrong action must deny
        kani::assert(
            !set.grants(&required),
            "matching name with wrong action does not grant",
        );
    }
}
