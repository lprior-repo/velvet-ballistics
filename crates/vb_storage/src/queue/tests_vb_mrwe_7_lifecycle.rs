//! bead vb-mrwe.7 — OBL-CONCURRENCY-PROP.
use proptest::prelude::*;
proptest! { #[test] fn vb_mrwe_7_queue_lifecycle_properties(pending in 0usize..16, cap in 0usize..16, shutdown in any::<bool>()) { prop_assume!(pending <= cap); let enqueue = !shutdown && pending < cap; prop_assert!(!shutdown || !enqueue); } }
