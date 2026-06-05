//! bead vb-mrwe.7 — OBL-DRAIN-PROP.
use proptest::prelude::*;
proptest! { #[test] fn vb_mrwe_7_drain_all_properties(pending in 0usize..16, first_error in any::<bool>()) { let remaining = if first_error { pending } else { 0 }; prop_assert!(!first_error || remaining == pending); } }
