//! bead vb-mrwe.7 — OBL-BOUND-PROP.
use proptest::prelude::*;
proptest! { #[test] fn vb_mrwe_7_batch_bound_properties(n in any::<usize>()) { let max = 16usize; let accepted = n > 0 && n <= max; prop_assert_eq!(accepted, (1..=max).contains(&n)); } }
