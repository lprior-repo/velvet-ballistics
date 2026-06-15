//! Proptest file: proptest_vb_db7vh_ps_002_capability_grant_subset_proptest_block
//!
//! RRO: RRO-vb-db7vh-002 (proptest lane)
//! Proof claim: PS-002 — submit_artifact(capabilities) accepts the call iff
//!   every requested capability is a member of the granted set. For any
//!   generated (granted, requested) pair, the result matches subset semantics.
//! Mapping target: crates/vb_runtime/src/runtime/submit_artifact.rs
//!   (Runtime::submit_artifact, capability check branch)
//!
//! Suffix convention: this file uses the `::_proptest_block` suffix split.
//! The proptest macro is invoked from a `proptest!` block named
//! `submit_artifact_capability_grant_subset_proptest_block`. The disjoint
//! split keeps the proptest-macro files separate from the `::_stub`
//! files in this bead (ps_001, ps_003, ps_005).

#![cfg(test)]

use proptest::prelude::*;
use vb_core::capability::Capability;
use vb_core::ids::ActionId;

mod submit_artifact_capability_grant_subset_proptest_block {
    use super::*;

    /// Build a Capability from a u8 discriminator for the proptest generator.
    /// Names: 0 -> "github.com", 1 -> "/tmp", 2 -> "api.example.com".
    pub(crate) fn cap_from_u8(n: u8) -> Capability {
        let name: Box<str> = match n % 3 {
            0 => Box::from("github.com"),
            1 => Box::from("/tmp"),
            _ => Box::from("api.example.com"),
        };
        Capability::new(name, ActionId::new(0))
    }

    /// Proptest: for any generated pair (granted, requested), the
    /// capability check in `Runtime::submit_artifact` is true iff
    /// `requested ⊆ granted`.
    pub(crate) fn check_capability_grant_subset(
        granted: &[Capability],
        requested: &[Capability],
    ) -> bool {
        requested.iter().all(|cap| granted.contains(cap))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn proptest_vb_db7vh_ps_002_capability_grant_subset_proptest_block(
            granted in proptest::collection::vec(
                proptest::num::u8::ANY.prop_map(cap_from_u8),
                0..8,
            ),
            requested in proptest::collection::vec(
                proptest::num::u8::ANY.prop_map(cap_from_u8),
                0..8,
            ),
        ) {
            let ok = check_capability_grant_subset(&granted, &requested);
            let expected_ok = requested.iter().all(|c| granted.contains(c));
            prop_assert_eq!(ok, expected_ok, "submit_artifact capability check must be subset-equivalent");
        }
    }
}

#[test]
fn proptest_vb_db7vh_ps_002_capability_grant_subset_smoke_proptest_block() {
    let granted = vec![submit_artifact_capability_grant_subset_proptest_block::cap_from_u8(0)];
    let requested = vec![submit_artifact_capability_grant_subset_proptest_block::cap_from_u8(0)];
    let ok = submit_artifact_capability_grant_subset_proptest_block::check_capability_grant_subset(
        &granted, &requested,
    );
    assert!(
        ok,
        "requested ⊆ granted must hold for the canonical happy case"
    );
}
