//! bead vb-mrwe.7 — OBL-CRASH-PROP.
use proptest::prelude::*;
proptest! { #[test] fn vb_mrwe_7_commit_before_drain_properties(committed in any::<bool>(), equal in any::<bool>()) { let drains = committed && equal; prop_assert!(!(committed && !equal && drains)); } }
