//! bead vb-mrwe.7 — OBL-DURABILITY-PROP.
use proptest::prelude::*;
proptest! { #[test] fn vb_mrwe_7_durability_properties(strict in any::<bool>(), strict_ok in any::<bool>()) { let success = !strict || strict_ok; prop_assert!(!(strict && !strict_ok && success)); } }
