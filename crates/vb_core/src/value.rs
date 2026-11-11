#![forbid(unsafe_code)]
//! Runtime slot value model.

mod display;
mod finite;
mod slot;
mod taint;

pub use display::SlotValueDisplay;
pub use finite::FiniteF64;
pub use slot::{ConstValue, SlotValue};
pub use taint::{SlotCell, Taint, join_taint};

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::ids::{ListId, SymbolId};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn slot_value_postcard_roundtrips_for_all_variants(
            val in prop_oneof![
                Just(SlotValue::Null),
                any::<bool>().prop_map(SlotValue::Bool),
                any::<i64>().prop_map(SlotValue::I64),
                (0u32..1000).prop_map(|id| SlotValue::Symbol(SymbolId::new(id))),
                (0u32..1000).prop_map(|id| SlotValue::List(ListId::new(id))),
            ]
        ) {
            let bytes = postcard::to_allocvec(&val);
            prop_assert!(bytes.is_ok(), "postcard serialization should succeed");
            let Ok(bytes) = bytes else { return Ok(()) };
            let recovered: Result<SlotValue, _> = postcard::from_bytes(&bytes);
            prop_assert!(recovered.is_ok(), "postcard deserialization should succeed");
            let Ok(recovered) = recovered else { return Ok(()) };
            prop_assert_eq!(val, recovered);
        }
    }

    proptest! {
        #[test]
        fn taint_ordering_is_reflexive(taint in prop_oneof![
            Just(Taint::Clean),
            Just(Taint::Secret),
            Just(Taint::DerivedFromSecret),
        ]) {
            prop_assert_eq!(taint, taint);
        }
    }

    proptest! {
        #[test]
        fn finite_f64_rejects_nan(nan_bits in 0u64..) {
            let val = f64::from_bits(nan_bits);
            if val.is_nan() {
                prop_assert!(FiniteF64::new(val).is_err());
            }
        }
    }
}

#[cfg(test)]
#[path = "value/tests.rs"]
mod tests;
