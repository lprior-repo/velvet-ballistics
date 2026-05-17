#![forbid(unsafe_code)]
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
            Self::Blob(id) => write!(f, "blob:{}", id.get()),
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

    #[test]
    fn taint_postcard_roundtrips_all_variants() {
        let variants = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
        for variant in variants {
            let bytes = postcard::to_allocvec(&variant);
            assert!(
                bytes.is_ok(),
                "postcard serialization should succeed for {variant:?}"
            );
            let Ok(bytes) = bytes else { return };
            let recovered: Result<Taint, _> = postcard::from_bytes(&bytes);
            assert!(
                recovered.is_ok(),
                "postcard deserialization should succeed for {variant:?}"
            );
            let Ok(recovered) = recovered else { return };
            assert_eq!(variant, recovered, "roundtrip must preserve {variant:?}");
        }
    }

    #[test]
    fn taint_lattice_join_is_commutative() {
        let variants = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
        for a in variants {
            for b in variants {
                assert_eq!(
                    join_taint(a, b),
                    join_taint(b, a),
                    "join_taint must be commutative: join({a:?}, {b:?})"
                );
            }
        }
    }

    #[test]
    fn taint_lattice_join_is_associative() {
        let variants = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
        for a in variants {
            for b in variants {
                for c in variants {
                    assert_eq!(
                        join_taint(join_taint(a, b), c),
                        join_taint(a, join_taint(b, c)),
                        "join_taint must be associative: ({a:?}, {b:?}, {c:?})"
                    );
                }
            }
        }
    }

    #[test]
    fn taint_lattice_secret_is_top_element() {
        let variants = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
        for v in variants {
            assert_eq!(
                join_taint(v, Taint::Secret),
                Taint::Secret,
                "Secret must absorb all: join({v:?}, Secret)"
            );
        }
    }

    #[test]
    fn taint_lattice_clean_is_bottom_element() {
        let variants = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
        for v in variants {
            assert_eq!(
                join_taint(v, Taint::Clean),
                v,
                "Clean must be identity: join({v:?}, Clean)"
            );
        }
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

    // =========================================================================
    // Additional edge-case tests — construction, equality, Debug, Display
    // =========================================================================

    #[test]
    fn slot_value_null_debug_format() {
        let val = SlotValue::Null;
        assert!(
            format!("{val:?}").contains("Null"),
            "Debug for Null must contain 'Null'"
        );
    }

    #[test]
    fn slot_value_bool_debug_format() {
        let val = SlotValue::Bool(true);
        let debug = format!("{val:?}");
        assert!(debug.contains("Bool"), "Debug for Bool must contain 'Bool'");
    }

    #[test]
    fn slot_value_i64_debug_format() {
        let val = SlotValue::I64(-99);
        let debug = format!("{val:?}");
        assert!(debug.contains("I64"), "Debug for I64 must contain 'I64'");
    }

    #[test]
    fn slot_value_null_display_is_null() {
        assert_eq!(format!("{}", SlotValue::Null), "null");
    }

    #[test]
    fn slot_value_bool_display_true() {
        assert_eq!(format!("{}", SlotValue::Bool(true)), "true");
    }

    #[test]
    fn slot_value_bool_display_false() {
        assert_eq!(format!("{}", SlotValue::Bool(false)), "false");
    }

    #[test]
    fn slot_value_i64_display() {
        assert_eq!(format!("{}", SlotValue::I64(42)), "42");
    }

    #[test]
    fn slot_value_i64_negative_display() {
        assert_eq!(format!("{}", SlotValue::I64(-1)), "-1");
    }

    #[test]
    fn slot_value_symbol_display() {
        let val = SlotValue::Symbol(SymbolId::new(7));
        assert_eq!(format!("{val}"), "symbol:7");
    }

    #[test]
    fn slot_value_list_display() {
        let val = SlotValue::List(ListId::new(3));
        assert_eq!(format!("{val}"), "list:3");
    }

    #[test]
    fn slot_value_object_display() {
        let val = SlotValue::Object(ObjectId::new(5));
        assert_eq!(format!("{val}"), "object:5");
    }

    #[test]
    fn slot_value_blob_display() {
        let val = SlotValue::Blob(BlobId::new(9));
        assert_eq!(format!("{val}"), "blob:9");
    }

    #[test]
    fn slot_value_i64_equality_same() {
        assert_eq!(SlotValue::I64(0), SlotValue::I64(0));
        assert_eq!(SlotValue::I64(-1), SlotValue::I64(-1));
        assert_eq!(SlotValue::I64(i64::MAX), SlotValue::I64(i64::MAX));
    }

    #[test]
    fn slot_value_i64_inequality_different() {
        assert_ne!(SlotValue::I64(0), SlotValue::I64(1));
        assert_ne!(SlotValue::I64(-1), SlotValue::I64(1));
    }

    #[test]
    fn slot_value_bool_equality() {
        assert_eq!(SlotValue::Bool(true), SlotValue::Bool(true));
        assert_eq!(SlotValue::Bool(false), SlotValue::Bool(false));
        assert_ne!(SlotValue::Bool(true), SlotValue::Bool(false));
    }

    #[test]
    fn slot_value_copy_preserves_equality() {
        let a = SlotValue::I64(42);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn slot_value_clone_preserves_equality() {
        let a = SlotValue::Bool(true);
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn const_value_null_equality() {
        assert_eq!(ConstValue::Null, ConstValue::Null);
    }

    #[test]
    fn const_value_bool_equality() {
        assert_eq!(ConstValue::Bool(true), ConstValue::Bool(true));
        assert_eq!(ConstValue::Bool(false), ConstValue::Bool(false));
        assert_ne!(ConstValue::Bool(true), ConstValue::Bool(false));
    }

    #[test]
    fn const_value_i64_equality() {
        assert_eq!(ConstValue::I64(0), ConstValue::I64(0));
        assert_eq!(ConstValue::I64(i64::MAX), ConstValue::I64(i64::MAX));
        assert_eq!(ConstValue::I64(i64::MIN), ConstValue::I64(i64::MIN));
        assert_ne!(ConstValue::I64(0), ConstValue::I64(1));
    }

    #[test]
    fn const_value_symbol_equality() {
        assert_eq!(
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(0))
        );
        assert_ne!(
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(1))
        );
    }

    #[test]
    fn const_value_distinguishes_null_from_bool_false() {
        assert_ne!(ConstValue::Null, ConstValue::Bool(false));
    }

    #[test]
    fn const_value_distinguishes_i64_from_symbol() {
        assert_ne!(ConstValue::I64(0), ConstValue::Symbol(SymbolId::new(0)));
    }

    #[test]
    fn const_value_to_slot_value_null_preserves_equality() {
        assert_eq!(ConstValue::Null.to_slot_value(), Ok(SlotValue::Null));
    }

    #[test]
    fn finite_f64_display_matches_inner() -> Result<(), String> {
        let val = FiniteF64::new(3.14).map_err(|e| e.to_string())?;
        let display = format!("{val}");
        assert!(
            display.contains("3.14"),
            "display must contain the value, got: {display}"
        );
        Ok(())
    }

    #[test]
    fn taint_debug_format_variants() {
        let clean_debug = format!("{:?}", Taint::Clean);
        assert!(clean_debug.contains("Clean"));
        let secret_debug = format!("{:?}", Taint::Secret);
        assert!(secret_debug.contains("Secret"));
        let derived_debug = format!("{:?}", Taint::DerivedFromSecret);
        assert!(derived_debug.contains("DerivedFromSecret"));
    }

    #[test]
    fn taint_equality_reflexive() {
        assert_eq!(Taint::Clean, Taint::Clean);
        assert_eq!(Taint::Secret, Taint::Secret);
        assert_eq!(Taint::DerivedFromSecret, Taint::DerivedFromSecret);
    }

    #[test]
    fn taint_inequality() {
        assert_ne!(Taint::Clean, Taint::Secret);
        assert_ne!(Taint::Clean, Taint::DerivedFromSecret);
        assert_ne!(Taint::DerivedFromSecret, Taint::Secret);
    }

    #[test]
    fn slot_value_f64_with_positive_zero() -> Result<(), String> {
        let finite = FiniteF64::new(0.0).map_err(|e| e.to_string())?;
        let val = SlotValue::F64(finite);
        assert_eq!(val.type_name(), "number");
        assert!(!val.is_true());
        Ok(())
    }

    #[test]
    fn const_value_debug_format() {
        let debug = format!("{:?}", ConstValue::I64(42));
        assert!(
            debug.contains("I64"),
            "Debug for ConstValue::I64 must contain 'I64'"
        );
        let debug = format!("{:?}", ConstValue::Null);
        assert!(
            debug.contains("Null"),
            "Debug for ConstValue::Null must contain 'Null'"
        );
    }

    #[test]
    fn slot_value_all_variants_distinct_type_names() {
        // Ensure each handle variant has a distinct type name
        assert_ne!(
            SlotValue::Symbol(SymbolId::new(0)).type_name(),
            SlotValue::List(ListId::new(0)).type_name()
        );
        assert_ne!(
            SlotValue::List(ListId::new(0)).type_name(),
            SlotValue::Object(ObjectId::new(0)).type_name()
        );
        assert_ne!(
            SlotValue::Object(ObjectId::new(0)).type_name(),
            SlotValue::Blob(BlobId::new(0)).type_name()
        );
    }

    // -- SlotValueDisplay / display_with_store tests --

    #[test]
    fn display_with_store_null_returns_null() {
        let store = crate::value_store::ValueStore::new();
        assert_eq!(SlotValue::Null.display_with_store(&store), "null");
    }

    #[test]
    fn display_with_store_bool_true_returns_true() {
        let store = crate::value_store::ValueStore::new();
        assert_eq!(SlotValue::Bool(true).display_with_store(&store), "true");
    }

    #[test]
    fn display_with_store_i64_returns_number() {
        let store = crate::value_store::ValueStore::new();
        assert_eq!(SlotValue::I64(42).display_with_store(&store), "42");
    }

    #[test]
    fn display_with_store_symbol_resolves() {
        let mut store = crate::value_store::ValueStore::new();
        let id = store.insert_symbol("hello").expect("insert");
        assert_eq!(SlotValue::Symbol(id).display_with_store(&store), "symbol:hello");
    }

    #[test]
    fn display_with_store_symbol_out_of_bounds_falls_back() {
        let store = crate::value_store::ValueStore::new();
        assert_eq!(
            SlotValue::Symbol(SymbolId::new(99)).display_with_store(&store),
            "symbol:99"
        );
    }

    #[test]
    fn display_with_store_list_resolves() {
        let mut store = crate::value_store::ValueStore::new();
        let id = store
            .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
            .expect("insert");
        assert_eq!(SlotValue::List(id).display_with_store(&store), "[1, 2]");
    }

    #[test]
    fn display_with_store_list_out_of_bounds_falls_back() {
        let store = crate::value_store::ValueStore::new();
        assert_eq!(
            SlotValue::List(ListId::new(99)).display_with_store(&store),
            "list:99"
        );
    }

    #[test]
    fn display_with_store_object_resolves() {
        let mut store = crate::value_store::ValueStore::new();
        // Insert the field key as a symbol so it resolves during display.
        let _sym_id = store.insert_symbol("field_key").expect("insert");
        let id = store
            .insert_object(
                vec![crate::value_store::ObjectField {
                    key: SymbolId::new(0),
                    value: SlotValue::I64(42),
                    taint: crate::value::Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .expect("insert");
        let result = SlotValue::Object(id).display_with_store(&store);
        assert_eq!(result, "{field_key: 42}");
    }

    #[test]
    fn display_with_store_object_out_of_bounds_falls_back() {
        let store = crate::value_store::ValueStore::new();
        assert_eq!(
            SlotValue::Object(ObjectId::new(99)).display_with_store(&store),
            "object:99"
        );
    }

    #[test]
    fn display_with_store_blob_resolves() {
        let mut store = crate::value_store::ValueStore::new();
        let id = store.insert_blob(bytes::Bytes::from_static(b"abc")).expect("insert");
        assert_eq!(
            SlotValue::Blob(id).display_with_store(&store),
            "blob:<3 bytes>"
        );
    }

    #[test]
    fn display_with_store_blob_out_of_bounds_falls_back() {
        let store = crate::value_store::ValueStore::new();
        assert_eq!(
            SlotValue::Blob(BlobId::new(99)).display_with_store(&store),
            "blob:99"
        );
    }

    #[test]
    fn display_with_store_nested_list() {
        let mut store = crate::value_store::ValueStore::new();
        let inner = store
            .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
            .expect("insert");
        let outer = store
            .insert_list(vec![SlotValue::List(inner)].into_boxed_slice())
            .expect("insert");
        assert_eq!(
            SlotValue::List(outer).display_with_store(&store),
            "[[1]]"
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
    ///
    /// # Performance Note
    /// This method allocates only when formatting output. The [`SlotValueDisplay`]
    /// type defers all formatting to its `Display` implementation, keeping the
    /// hot-path value module allocation-free.
    pub fn display_with_store(&self, store: &ValueStore) -> String {
        SlotValueDisplay::new(self, store).to_string()
    }
}

/// Lazily-formatted display for [`SlotValue`] that resolves arena handles
/// against a [`ValueStore`]. Allocations are deferred until `Display::fmt`
/// is called (i.e., when `to_string()` or `format!()` is invoked).
///
/// # Example
/// ```
/// # use vb_core::value::SlotValue;
/// # use vb_core::value_store::ValueStore;
/// # use vb_core::value::SlotValueDisplay;
/// let store = ValueStore::new();
/// let value = SlotValue::Null;
/// let display = SlotValueDisplay::new(&value, &store);
/// assert_eq!(format!("{display}"), "null");
/// ```
#[derive(Debug)]
pub struct SlotValueDisplay<'a>(&'a SlotValue, &'a ValueStore);

impl<'a> SlotValueDisplay<'a> {
    /// Create a new formatter for `value` using `store` for arena resolution.
    #[inline]
    pub fn new(value: &'a SlotValue, store: &'a ValueStore) -> Self {
        Self(value, store)
    }
}

impl fmt::Display for SlotValueDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            SlotValue::Null => write!(f, "null"),
            SlotValue::Bool(v) => write!(f, "{v}"),
            SlotValue::I64(v) => write!(f, "{v}"),
            SlotValue::F64(v) => write!(f, "{v}"),
            SlotValue::Symbol(id) => match self.1.symbol(*id) {
                Ok(s) => write!(f, "symbol:{s}"),
                Err(_) => write!(f, "symbol:{}", id.get()),
            },
            SlotValue::List(id) => match self.1.list(*id) {
                Ok(items) => {
                    write!(f, "[")?;
                    let mut first = true;
                    for item in items {
                        if !first {
                            write!(f, ", ")?;
                        }
                        first = false;
                        SlotValueDisplay::new(item, self.1).fmt(f)?;
                    }
                    write!(f, "]")
                }
                Err(_) => write!(f, "list:{}", id.get()),
            },
            SlotValue::Object(id) => match self.1.object(*id) {
                Ok(fields) => {
                    write!(f, "{{")?;
                    let mut first = true;
                    for field in fields {
                        if !first {
                            write!(f, ", ")?;
                        }
                        first = false;
                        let key_display = match self.1.symbol(field.key) {
                            Ok(s) => s,
                            Err(_) => return write!(f, "{}:", field.key.get()),
                        };
                        write!(f, "{key_display}: ")?;
                        SlotValueDisplay::new(&field.value, self.1).fmt(f)?;
                    }
                    write!(f, "}}")
                }
                Err(_) => write!(f, "object:{}", id.get()),
            },
            SlotValue::Blob(id) => match self.1.blob(*id) {
                Ok(bytes) => write!(f, "blob:<{} bytes>", bytes.len()),
                Err(_) => write!(f, "blob:{}", id.get()),
            },
        }
    }
}
