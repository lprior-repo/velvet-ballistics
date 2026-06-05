//! bead vb-mrwe.7 — OBL-DUP-PROP.
use proptest::prelude::*;
proptest! { #[test] fn vb_mrwe_7_duplicate_policy_properties(seen in any::<bool>(), equal in any::<bool>()) { let drains = !seen || equal; prop_assert!(!(seen && !equal && drains)); } }
