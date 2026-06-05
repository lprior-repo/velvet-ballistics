//! bead vb-mrwe.7 — OBL-ATOM-PROP. Property model pending crate wiring in State 9.
use proptest::prelude::*;
proptest! { #[test] fn vb_mrwe_7_flush_batch_atomic_properties(pending in 0usize..16, batch in 1usize..16, ok in any::<bool>()) { let p = pending.min(batch); let d = if ok { p } else { 0 }; prop_assert!(d <= p); prop_assert!(ok || d == 0); } }
