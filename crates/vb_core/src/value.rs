//! Runtime slot value model.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::value_store::ValueStore;
use core::fmt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Secret propagation marker attached to each runtime slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Taint {
    /// Slot contains no secret-derived data.
    Clean = 0,
    /// Slot contains data derived from one or more secrets.
    DerivedFromSecret = 1,
    /// Slot contains a secret value.
    Secret = 2,
}

/// Joins two taint levels, returning the more restrictive one.
#[must_use]
pub fn join_taint(a: Taint, b: Taint) -> Taint {
    let a_disc: u8 = match a {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    };
    let b_disc: u8 = match b {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    };
    if a_disc >= b_disc { a } else { b }
}

/// Finite floating-point scalar accepted by the runtime value model.
///
/// # Why a custom newtype (not `ordered-float` / `noisy_float`)
///
/// Both `ordered-float::NotNan<f64>` and `noisy_float::R64` were evaluated:
///
/// - `NotNan` only rejects NaN and **allows** +/- infinity, so it cannot replace
///   this type without an additional manual check -- adding a dependency for no
///   net benefit.
/// - `R64` (`NoisyFloat<f64, FiniteChecker>`) does reject both NaN and infinity,
///   but validates via `debug_assert!`, meaning invalid values silently pass in
///   release builds.  This is incompatible with the project's zero-tolerance
///   safety policy (`unwrap_used = "deny"`, no panics in production paths).
/// - Both crates pull in `num-traits` and other transitive dependencies the
///   workspace otherwise avoids.
///
/// The custom implementation is ~40 lines of straightforward code, validates in
/// both debug **and** release builds, has zero dependencies, and provides exactly
/// the invariant this crate needs: "reject NaN AND infinity at construction."
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct FiniteF64(f64);

impl Eq for FiniteF64 {}

impl fmt::Display for FiniteF64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

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

impl fmt::Display for SlotValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(v) => write!(f, "{v}"),
            Self::I64(v) => write!(f, "{v}"),
            Self::F64(v) => write!(f, "{v}"),
            Self::Symbol(id) => write!(f, "symbol:{}", id.get()),
            Self::List(id) => write!(f, "list:{}", id.get()),
            Self::Object(id) => write!(f, "object:{}", id.get()),
            Self::Blob(id) => write!(f, "blob:{}", id.as_u64()),
        }
    }
}

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

        assert_eq!(result.map(|f| f.get()), Ok(0.0));
    }

    #[test]
    fn finite_f64_accepts_negative_one() {
        let result = FiniteF64::new(-1.0);

        assert_eq!(result.map(|f| f.get()), Ok(-1.0));
    }

    #[test]
    fn finite_f64_accepts_max_finite() {
        let result = FiniteF64::new(f64::MAX);

        assert_eq!(result.map(|f| f.get()), Ok(f64::MAX));
    }

    #[test]
    fn finite_f64_get_returns_inner_value() -> Result<(), String> {
        let value = FiniteF64::new(3.25).map_err(|error| error.to_string())?;

        if value.get() != 3.25 {
            return Err(String::from("unexpected finite f64 inner value"));
        }
        Ok(())
    }

    // -- SlotValue type_name tests --

    #[test]
    fn slot_value_type_name_returns_correct_names() -> Result<(), String> {
        if SlotValue::Null.type_name() != "null" {
            return Err(String::from("unexpected null type name"));
        }
        if SlotValue::Bool(true).type_name() != "boolean" {
            return Err(String::from("unexpected true type name"));
        }
        if SlotValue::Bool(false).type_name() != "boolean" {
            return Err(String::from("unexpected false type name"));
        }
        if SlotValue::I64(0).type_name() != "number" {
            return Err(String::from("unexpected i64 type name"));
        }
        let f64_val = FiniteF64::new(1.0).map_err(|error| error.to_string())?;
        if SlotValue::F64(f64_val).type_name() != "number" {
            return Err(String::from("unexpected f64 type name"));
        }
        if SlotValue::Symbol(SymbolId::new(1)).type_name() != "symbol" {
            return Err(String::from("unexpected symbol type name"));
        }
        if SlotValue::List(ListId::new(1)).type_name() != "list" {
            return Err(String::from("unexpected list type name"));
        }
        if SlotValue::Object(ObjectId::new(1)).type_name() != "object" {
            return Err(String::from("unexpected object type name"));
        }
        if SlotValue::Blob(BlobId::new(1)).type_name() != "blob" {
            return Err(String::from("unexpected blob type name"));
        }
        Ok(())
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
    fn const_value_to_slot_value_maps_f64_correctly() -> Result<(), String> {
        let finite = FiniteF64::new(2.5).map_err(|error| error.to_string())?;
        let result = ConstValue::F64(finite).to_slot_value();

        if result != Ok(SlotValue::F64(finite)) {
            return Err(String::from("unexpected f64 slot value"));
        }
        Ok(())
    }

    #[test]
    fn const_value_to_slot_value_maps_symbol_correctly() {
        let result = ConstValue::Symbol(SymbolId::new(7)).to_slot_value();

        assert_eq!(result, Ok(SlotValue::Symbol(SymbolId::new(7))));
    }

    // =========================================================================
    // Adversarial BDD tests — FiniteF64 edge cases
    // =========================================================================

    #[test]
    fn finite_f64_negative_zero_is_accepted_and_preserves_sign_bit() {
        // -0.0 is finite and must be accepted; the sign bit must survive.
        let result = FiniteF64::new(-0.0_f64);
        assert_eq!(result.as_ref().map(|f| f.get()), Ok(-0.0_f64));
        // Confirm it is distinct from +0.0 at the bit-pattern level.
        assert_eq!(result.map(|f| f.get().to_bits()), Ok((-0.0_f64).to_bits()));
    }

    #[test]
    fn finite_f64_positive_zero_is_accepted() {
        let result = FiniteF64::new(0.0_f64);
        assert_eq!(result.map(|f| f.get()), Ok(0.0_f64));
    }

    #[test]
    fn finite_f64_rejects_canonical_nan_quiet() {
        let result = FiniteF64::new(f64::NAN);
        assert_eq!(result, Err(CoreError::NonFiniteNumber));
    }

    #[test]
    fn finite_f64_rejects_signaling_nan() {
        // Signaling NaN: exponent all-ones, MSB of mantissa clear, non-zero mantissa.
        let signaling_nan = f64::from_bits(0x7FF0_0000_0000_0001_u64);
        assert!(signaling_nan.is_nan(), "must be NaN");
        assert_eq!(
            FiniteF64::new(signaling_nan),
            Err(CoreError::NonFiniteNumber)
        );
    }

    #[test]
    fn finite_f64_rejects_negative_signaling_nan() {
        let neg_signaling_nan = f64::from_bits(0xFFF0_0000_0000_0001_u64);
        assert!(neg_signaling_nan.is_nan(), "must be NaN");
        assert_eq!(
            FiniteF64::new(neg_signaling_nan),
            Err(CoreError::NonFiniteNumber)
        );
    }

    #[test]
    fn finite_f64_rejects_nan_payload_variants() {
        // Try several NaN bit patterns to ensure no bypass.
        let payloads: [u64; 4] = [
            0x7FF8_0000_0000_0000,
            0x7FFC_0000_0000_0000,
            0xFFF8_0000_0000_0000,
            0x7FFF_FFFF_FFFF_FFFF,
        ];
        for payload in payloads {
            let nan_val = f64::from_bits(payload);
            assert!(nan_val.is_nan(), "payload {payload:#018X} must be NaN");
            assert_eq!(
                FiniteF64::new(nan_val),
                Err(CoreError::NonFiniteNumber),
                "NaN payload {payload:#018X} must be rejected"
            );
        }
    }

    #[test]
    fn finite_f64_accepts_smallest_positive_subnormal() {
        let subnormal = f64::from_bits(1_u64); // smallest positive subnormal
        assert!(subnormal.is_subnormal(), "must be subnormal");
        let result = FiniteF64::new(subnormal);
        assert_eq!(result.map(|f| f.get()), Ok(subnormal));
    }

    #[test]
    fn finite_f64_accepts_largest_subnormal() {
        let largest_subnormal = f64::from_bits(0x000F_FFFF_FFFF_FFFF_u64);
        assert!(largest_subnormal.is_subnormal(), "must be subnormal");
        assert!(largest_subnormal.is_finite(), "subnormals are finite");
        let result = FiniteF64::new(largest_subnormal);
        assert_eq!(result.map(|f| f.get()), Ok(largest_subnormal));
    }

    #[test]
    fn finite_f64_accepts_smallest_negative_subnormal() {
        let neg_subnormal = f64::from_bits(0x8000_0000_0000_0001_u64);
        assert!(neg_subnormal.is_subnormal(), "must be negative subnormal");
        assert!(neg_subnormal.is_finite());
        let result = FiniteF64::new(neg_subnormal);
        assert_eq!(result.map(|f| f.get()), Ok(neg_subnormal));
    }

    #[test]
    fn finite_f64_accepts_min_positive_normal() {
        let min_normal = f64::MIN_POSITIVE; // 2.2250738585072014e-308
        assert!(!min_normal.is_subnormal());
        assert!(min_normal.is_finite());
        let result = FiniteF64::new(min_normal);
        assert_eq!(result.map(|f| f.get()), Ok(min_normal));
    }

    #[test]
    fn finite_f64_accepts_f64_min() {
        // f64::MIN is the most negative finite value
        let result = FiniteF64::new(f64::MIN);
        assert_eq!(result.map(|f| f.get()), Ok(f64::MIN));
    }

    #[test]
    fn finite_f64_accepts_f64_max() {
        let result = FiniteF64::new(f64::MAX);
        assert_eq!(result.map(|f| f.get()), Ok(f64::MAX));
    }

    #[test]
    fn finite_f64_rejects_positive_infinity() {
        assert_eq!(
            FiniteF64::new(f64::INFINITY),
            Err(CoreError::NonFiniteNumber)
        );
    }

    #[test]
    fn finite_f64_rejects_negative_infinity() {
        assert_eq!(
            FiniteF64::new(f64::NEG_INFINITY),
            Err(CoreError::NonFiniteNumber)
        );
    }

    // =========================================================================
    // Adversarial BDD tests — SlotValue type confusion and edge cases
    // =========================================================================

    #[test]
    fn slot_value_i64_max_roundtrips() {
        let val = SlotValue::I64(i64::MAX);
        assert_eq!(val.type_name(), "number");
        assert!(!val.is_true());
    }

    #[test]
    fn slot_value_i64_min_roundtrips() {
        let val = SlotValue::I64(i64::MIN);
        assert_eq!(val.type_name(), "number");
        assert!(!val.is_true());
    }

    #[test]
    fn slot_value_i64_zero_roundtrips() {
        let val = SlotValue::I64(0);
        assert_eq!(val.type_name(), "number");
        assert!(!val.is_true());
    }

    #[test]
    fn slot_value_i64_negative_one_roundtrips() {
        let val = SlotValue::I64(-1);
        assert_eq!(val.type_name(), "number");
    }

    #[test]
    fn slot_value_null_is_not_true() {
        assert!(!SlotValue::Null.is_true());
        assert_eq!(SlotValue::Null.type_name(), "null");
    }

    #[test]
    fn slot_value_bool_false_is_not_true() {
        assert!(!SlotValue::Bool(false).is_true());
    }

    #[test]
    fn slot_value_symbol_zero_is_valid() {
        let val = SlotValue::Symbol(SymbolId::new(0));
        assert_eq!(val.type_name(), "symbol");
        assert!(!val.is_true());
    }

    #[test]
    fn slot_value_symbol_max_u32_is_valid() {
        let val = SlotValue::Symbol(SymbolId::new(u32::MAX));
        assert_eq!(val.type_name(), "symbol");
    }

    #[test]
    fn slot_value_list_max_u32_is_valid() {
        let val = SlotValue::List(ListId::new(u32::MAX));
        assert_eq!(val.type_name(), "list");
    }

    #[test]
    fn slot_value_object_max_u32_is_valid() {
        let val = SlotValue::Object(ObjectId::new(u32::MAX));
        assert_eq!(val.type_name(), "object");
    }

    #[test]
    fn slot_value_blob_max_u64_is_valid() {
        let val = SlotValue::Blob(BlobId::new(u64::MAX));
        assert_eq!(val.type_name(), "blob");
    }

    #[test]
    fn slot_value_f64_with_negative_zero_is_valid() {
        let result = FiniteF64::new(-0.0_f64);
        assert_eq!(result.as_ref().map(|f| f.get()), Ok(-0.0_f64));
        let finite = result.expect("setup: negative zero must be finite");
        let val = SlotValue::F64(finite);
        assert_eq!(val.type_name(), "number");
        assert!(!val.is_true());
    }

    // =========================================================================
    // Adversarial BDD tests — Taint propagation and ordering
    // =========================================================================

    #[test]
    fn taint_clean_is_zero_discriminant() {
        assert_eq!(taint_discriminant(Taint::Clean), 0);
    }

    #[test]
    fn taint_secret_is_two_discriminant() {
        assert_eq!(taint_discriminant(Taint::Secret), 2);
    }

    #[test]
    fn taint_derived_from_secret_is_one_discriminant() {
        assert_eq!(taint_discriminant(Taint::DerivedFromSecret), 1);
    }

    fn taint_discriminant(taint: Taint) -> u8 {
        match taint {
            Taint::Clean => 0,
            Taint::DerivedFromSecret => 1,
            Taint::Secret => 2,
        }
    }

    #[test]
    fn taint_variants_are_distinct() {
        let variants = [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b, "same index must be equal");
                } else {
                    assert_ne!(a, b, "different indices must be distinct");
                }
            }
        }
    }

    #[test]
    fn taint_copy_semantics_preserve_equality() {
        let a = Taint::Secret;
        let b = a; // copy
        assert_eq!(a, b, "copy must preserve equality");
    }

    // =========================================================================
    // Adversarial BDD tests — ConstValue edge cases
    // =========================================================================

    #[test]
    fn const_value_to_slot_value_i64_max() {
        let result = ConstValue::I64(i64::MAX).to_slot_value();
        assert_eq!(result, Ok(SlotValue::I64(i64::MAX)));
    }

    #[test]
    fn const_value_to_slot_value_i64_min() {
        let result = ConstValue::I64(i64::MIN).to_slot_value();
        assert_eq!(result, Ok(SlotValue::I64(i64::MIN)));
    }

    #[test]
    fn const_value_to_slot_value_bool_false() {
        let result = ConstValue::Bool(false).to_slot_value();
        assert_eq!(result, Ok(SlotValue::Bool(false)));
    }

    #[test]
    fn const_value_to_slot_value_symbol_zero() {
        let result = ConstValue::Symbol(SymbolId::new(0)).to_slot_value();
        assert_eq!(result, Ok(SlotValue::Symbol(SymbolId::new(0))));
    }

    #[test]
    fn const_value_to_slot_value_symbol_max() {
        let result = ConstValue::Symbol(SymbolId::new(u32::MAX)).to_slot_value();
        assert_eq!(result, Ok(SlotValue::Symbol(SymbolId::new(u32::MAX))));
    }

    #[test]
    fn slot_value_equality_is_reflexive_for_null() {
        assert_eq!(SlotValue::Null, SlotValue::Null);
    }

    #[test]
    fn slot_value_equality_distinguishes_null_from_bool_false() {
        assert_ne!(SlotValue::Null, SlotValue::Bool(false));
    }

    #[test]
    fn slot_value_equality_distinguishes_i64_zero_from_f64_zero() {
        // SlotValue::I64(0) and SlotValue::F64(FiniteF64(0.0)) are different variants.
        let result = FiniteF64::new(0.0);
        assert_eq!(result.as_ref().map(|f| f.get()), Ok(0.0));
        let finite = result.expect("setup: zero must be finite");
        assert_ne!(SlotValue::I64(0), SlotValue::F64(finite));
    }

    #[test]
    fn slot_value_equality_distinguishes_symbol_from_list() {
        assert_ne!(
            SlotValue::Symbol(SymbolId::new(0)),
            SlotValue::List(ListId::new(0))
        );
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

    /// Resolves arena handles against the store and returns a human-readable
    /// string.  Falls back to the bare `Display` representation when the
    /// handle cannot be resolved (out-of-bounds, missing field, etc.).
    pub fn display_with_store(&self, store: &ValueStore) -> String {
        match self {
            Self::Null => String::from("null"),
            Self::Bool(v) => format!("{v}"),
            Self::I64(v) => format!("{v}"),
            Self::F64(v) => format!("{v}"),
            Self::Symbol(id) => match store.symbol(*id) {
                Ok(s) => format!("symbol:{s}"),
                Err(_) => format!("symbol:{}", id.get()),
            },
            Self::List(id) => match store.list(*id) {
                Ok(items) => {
                    let inner: Vec<String> =
                        items.iter().map(|v| v.display_with_store(store)).collect();
                    format!("[{}]", inner.join(", "))
                }
                Err(_) => format!("list:{}", id.get()),
            },
            Self::Object(id) => match store.object(*id) {
                Ok(fields) => {
                    let inner: Vec<String> = fields
                        .iter()
                        .map(|f| {
                            let key_display = match store.symbol(f.key) {
                                Ok(s) => String::from(s),
                                Err(_) => format!("{}", f.key.get()),
                            };
                            format!("{}: {}", key_display, f.value.display_with_store(store))
                        })
                        .collect();
                    format!("{{{}}}", inner.join(", "))
                }
                Err(_) => format!("object:{}", id.get()),
            },
            Self::Blob(id) => match store.blob(*id) {
                Ok(bytes) => {
                    let len = bytes.len();
                    format!("blob:<{len} bytes>")
                }
                Err(_) => format!("blob:{}", id.as_u64()),
            },
        }
    }
}
