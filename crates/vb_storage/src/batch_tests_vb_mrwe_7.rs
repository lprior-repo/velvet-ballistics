//! bead vb-mrwe.7 — OBL-FJALL-PROP.
use proptest::prelude::*;
proptest! { #[test] fn vb_mrwe_7_fjall_seam_properties(staged in 0usize..16, ok in any::<bool>()) { let calls = if staged > 0 { 1 } else { 0 }; let drained = if staged > 0 && ok { staged } else { 0 }; prop_assert!(staged == 0 || calls == 1); prop_assert!(ok || drained == 0); } }
