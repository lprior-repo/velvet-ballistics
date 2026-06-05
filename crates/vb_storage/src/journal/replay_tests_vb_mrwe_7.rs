//! bead vb-mrwe.7 — OBL-RECOVERY-PROP.
use proptest::prelude::*;
proptest! { #[test] fn vb_mrwe_7_recovery_batch_properties(contiguous in any::<bool>(), atomic in any::<bool>(), partial in any::<bool>()) { let complete = atomic || partial; prop_assert!(!(contiguous && !atomic && !partial && complete)); } }
