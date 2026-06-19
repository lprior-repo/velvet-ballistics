//! Proptest generators for property testing delta computation.

use proptest::prelude::*;
use vb_core::frame::StepState;
use vb_core::ids::{BlobId, ListId, ObjectId, SymbolId};
use vb_core::value::{SlotValue, Taint};

// Generate a random `SlotValue` variant.
prop_compose! {
    fn arb_slot_value()(
        variant in prop_oneof![
            Just(SlotValue::Null),
            any::<bool>().prop_map(SlotValue::Bool),
            any::<i64>().prop_map(SlotValue::I64),
            (0u32..1000u32).prop_map(|id| SlotValue::Symbol(SymbolId::new(id))),
            (0u32..1000u32).prop_map(|id| SlotValue::List(ListId::new(id))),
            (0u32..1000u32).prop_map(|id| SlotValue::Object(ObjectId::new(id))),
            (0u64..1000u64).prop_map(|id| SlotValue::Blob(BlobId::new(id))),
        ]
    ) -> SlotValue {
        variant
    }
}

// Generate a random `Taint` variant.
prop_compose! {
    fn arb_taint()(
        t in prop_oneof![
            Just(Taint::Clean),
            Just(Taint::DerivedFromSecret),
            Just(Taint::Secret),
        ]
    ) -> Taint {
        t
    }
}

// Generate a random `StepState` variant.
prop_compose! {
    fn arb_step_state()(
        s in prop_oneof![
            Just(StepState::Pending),
            Just(StepState::Running),
            Just(StepState::Succeeded),
            Just(StepState::Failed),
            Just(StepState::Skipped),
            Just(StepState::Waiting),
            Just(StepState::Asking),
            Just(StepState::Cancelled),
        ]
    ) -> StepState {
        s
    }
}
