//! Runtime slot value model.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Secret propagation marker attached to each runtime slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Taint {
    /// Slot contains no secret-derived data.
    Clean = 0,
    /// Slot contains a secret value.
    Secret = 1,
    /// Slot contains data derived from one or more secrets.
    DerivedFromSecret = 2,
}

/// Finite floating-point scalar accepted by the runtime value model.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct FiniteF64(f64);

impl Eq for FiniteF64 {}

impl FiniteF64 {
    /// Creates a finite floating-point value, rejecting NaN and infinities.
    pub fn new(value: f64) -> CoreResult<Self> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(CoreError::NonFiniteNumber)
        }
    }

    /// Returns the raw finite floating-point value.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl Serialize for FiniteF64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for FiniteF64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        f64::deserialize(deserializer).and_then(|value| {
            Self::new(value).map_err(|err| serde::de::Error::custom(err.to_string()))
        })
    }
}

/// Compact handle-based runtime value stored in numeric slots.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SlotValue {
    /// Explicit null value.
    Null,
    /// Boolean scalar.
    Bool(bool),
    /// Signed integer scalar for deterministic arithmetic scaffolding.
    I64(i64),
    /// Finite floating-point scalar.
    F64(FiniteF64),
    /// Interned symbol handle.
    Symbol(SymbolId),
    /// Runtime list arena handle.
    List(ListId),
    /// Runtime object arena handle.
    Object(ObjectId),
    /// Runtime blob arena/storage handle.
    Blob(BlobId),
}

impl Eq for SlotValue {}

/// Compile-time constant value, smaller than SlotValue.
/// Constants cannot hold runtime-allocated handles (List, Object, Blob).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstValue {
    /// Explicit null value.
    Null,
    /// Boolean scalar.
    Bool(bool),
    /// Signed integer scalar for deterministic arithmetic scaffolding.
    I64(i64),
    /// Finite floating-point scalar.
    F64(FiniteF64),
    /// Interned symbol handle.
    Symbol(SymbolId),
}

impl ConstValue {
    /// Convert to a runtime slot value.
    pub fn to_slot_value(&self) -> CoreResult<SlotValue> {
        match self {
            Self::Null => Ok(SlotValue::Null),
            Self::Bool(v) => Ok(SlotValue::Bool(*v)),
            Self::I64(v) => Ok(SlotValue::I64(*v)),
            Self::F64(v) => Ok(SlotValue::F64(*v)),
            Self::Symbol(v) => Ok(SlotValue::Symbol(*v)),
        }
    }
}

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
mod tests {
    use super::*;
    use crate::errors::CoreError;
    use crate::ids::{BlobId, ListId, ObjectId, SymbolId};

    // -- FiniteF64 rejection tests --

    #[test]
    fn finite_f64_rejects_nan_returns_non_finite_number() {
        let result = FiniteF64::new(f64::NAN);

        assert_eq!(result, Err(CoreError::NonFiniteNumber));
    }

    #[test]
    fn finite_f64_rejects_positive_infinity_returns_non_finite_number() {
        let result = FiniteF64::new(f64::INFINITY);

        assert_eq!(result, Err(CoreError::NonFiniteNumber));
    }

    #[test]
    fn finite_f64_rejects_negative_infinity_returns_non_finite_number() {
        let result = FiniteF64::new(f64::NEG_INFINITY);

        assert_eq!(result, Err(CoreError::NonFiniteNumber));
    }

    #[test]
    fn finite_f64_accepts_zero() {
        let result = FiniteF64::new(0.0);

        assert!(result.is_ok());
        assert_eq!(result.map_err(|_| String::new()).map(|v| v.get()), Ok(0.0));
    }

    #[test]
    fn finite_f64_accepts_negative_one() {
        let result = FiniteF64::new(-1.0);

        assert!(result.is_ok());
        assert_eq!(result.map_err(|_| String::new()).map(|v| v.get()), Ok(-1.0));
    }

    #[test]
    fn finite_f64_accepts_max_finite() {
        let result = FiniteF64::new(f64::MAX);

        assert!(result.is_ok());
        let inner = result.map_err(|_| String::new()).map(|v| v.get());
        assert_eq!(inner, Ok(f64::MAX));
    }

    #[test]
    fn finite_f64_get_returns_inner_value() {
        let value = match FiniteF64::new(3.14) {
            Ok(v) => v,
            Err(e) => panic!("finite f64 creation failed: {e}"),
        };

        assert_eq!(value.get(), 3.14);
    }

    // -- SlotValue type_name tests --

    #[test]
    fn slot_value_type_name_returns_correct_names() {
        assert_eq!(SlotValue::Null.type_name(), "null");
        assert_eq!(SlotValue::Bool(true).type_name(), "boolean");
        assert_eq!(SlotValue::Bool(false).type_name(), "boolean");
        assert_eq!(SlotValue::I64(0).type_name(), "number");
        let f64_val = match FiniteF64::new(1.0) {
            Ok(v) => v,
            Err(e) => panic!("finite f64 creation failed: {e}"),
        };
        assert_eq!(SlotValue::F64(f64_val).type_name(), "number");
        assert_eq!(SlotValue::Symbol(SymbolId::new(1)).type_name(), "symbol");
        assert_eq!(SlotValue::List(ListId::new(1)).type_name(), "list");
        assert_eq!(SlotValue::Object(ObjectId::new(1)).type_name(), "object");
        assert_eq!(SlotValue::Blob(BlobId::new(1)).type_name(), "blob");
    }

    // -- SlotValue is_true tests --

    #[test]
    fn slot_value_is_true_only_for_bool_true() {
        assert!(SlotValue::Bool(true).is_true());
    }

    #[test]
    fn slot_value_is_true_returns_false_for_bool_false() {
        assert!(!SlotValue::Bool(false).is_true());
    }

    #[test]
    fn slot_value_is_true_returns_false_for_null() {
        assert!(!SlotValue::Null.is_true());
    }

    // -- ConstValue to_slot_value mapping tests --

    #[test]
    fn const_value_to_slot_value_maps_null_correctly() {
        let result = ConstValue::Null.to_slot_value();

        assert_eq!(result, Ok(SlotValue::Null));
    }

    #[test]
    fn const_value_to_slot_value_maps_bool_true_correctly() {
        let result = ConstValue::Bool(true).to_slot_value();

        assert_eq!(result, Ok(SlotValue::Bool(true)));
    }

    #[test]
    fn const_value_to_slot_value_maps_i64_correctly() {
        let result = ConstValue::I64(42).to_slot_value();

        assert_eq!(result, Ok(SlotValue::I64(42)));
    }

    #[test]
    fn const_value_to_slot_value_maps_f64_correctly() {
        let finite = match FiniteF64::new(2.5) {
            Ok(v) => v,
            Err(e) => panic!("finite f64 creation failed: {e}"),
        };
        let result = ConstValue::F64(finite).to_slot_value();

        assert_eq!(result, Ok(SlotValue::F64(finite)));
    }

    #[test]
    fn const_value_to_slot_value_maps_symbol_correctly() {
        let result = ConstValue::Symbol(SymbolId::new(7)).to_slot_value();

        assert_eq!(result, Ok(SlotValue::Symbol(SymbolId::new(7))));
    }
}

impl SlotValue {
    /// Returns the stable runtime type name for diagnostics.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "boolean",
            Self::I64(_) | Self::F64(_) => "number",
            Self::Symbol(_) => "symbol",
            Self::List(_) => "list",
            Self::Object(_) => "object",
            Self::Blob(_) => "blob",
        }
    }

    /// Returns true only for `Bool(true)`.
    #[must_use]
    pub const fn is_true(&self) -> bool {
        matches!(self, Self::Bool(true))
    }
}
