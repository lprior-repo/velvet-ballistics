//! Property-based tests for boundary conditions.

use crate::{FiniteF64, SlotIdx, StepIdx};
use proptest::prelude::*;

// Property: Slot index roundtrip for all valid u16 values.
proptest! {
    #[test]
    fn slot_index_roundtrip(idx in 0u16..=u16::MAX) {
        let slot = SlotIdx::new(idx);
        assert_eq!(slot.get(), idx);
    }
}

// Property: Step index roundtrip for all valid u16 values.
proptest! {
    #[test]
    fn step_index_roundtrip(idx in 0u16..=u16::MAX) {
        let step = StepIdx::new(idx);
        assert_eq!(step.get(), idx);
    }
}

// Property: FiniteF64 accepts all finite values.
proptest! {
    #[test]
    fn finite_f64_accepts_finite(value in -1e308f64..=1e308f64) {
        if value.is_finite() {
            assert!(FiniteF64::new(value).is_ok());
        }
    }
}

// Property: FiniteF64 rejects non-finite values.
#[test]
fn finite_f64_rejects_non_finite() {
    assert!(FiniteF64::new(f64::INFINITY).is_err());
    assert!(FiniteF64::new(f64::NEG_INFINITY).is_err());
    assert!(FiniteF64::new(f64::NAN).is_err());
}

// Property: RunId roundtrip.
proptest! {
    #[test]
    fn run_id_roundtrip(val in 1u64..=u64::MAX) {
        let id = crate::RunId::new(val);
        assert_eq!(id.get(), val);
    }
}

// Property: EventSeq roundtrip.
proptest! {
    #[test]
    fn event_seq_roundtrip(val in 0u64..=u64::MAX) {
        let seq = crate::EventSeq::new(val);
        assert_eq!(seq.get(), val);
    }
}

// Property: SeqNo roundtrip.
proptest! {
    #[test]
    fn seq_no_roundtrip(val in 0u64..=u64::MAX) {
        let seq = crate::SeqNo::new(val);
        assert_eq!(seq.get(), val);
    }
}
