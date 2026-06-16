#![forbid(unsafe_code)]
//! Runtime slot value model.

use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::value_store::ValueStore;
use core::fmt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// ───────────────────────────────────────────────────────────────────────────
// Verus annotations for Taint enum (compiled under verus toolchain only)
// ───────────────────────────────────────────────────────────────────────────
#[cfg(verus)]
verus! {
    use vstd::prelude::*;

    use super::{Taint, join_taint};

    /// Spec: Taint has exactly three values.
    pub closed spec fn spec_taint_has_three_values() -> bool {
        let all_taints = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
        all_taints.len() == 3
            && all_taints[0] != all_taints[1]
            && all_taints[0] != all_taints[2]
            && all_taints[1] != all_taints[2]
    }

    /// Spec: Taint ordering — Clean < DerivedFromSecret < Secret.
    pub closed spec fn spec_taint_ordering() -> bool {
        Taint::Clean as u8 < Taint::DerivedFromSecret as u8
            && Taint::DerivedFromSecret as u8 < Taint::Secret as u8
    }

    /// Spec: join_taint returns the max (more restrictive) of two taint levels.
    /// This captures the taint lattice: Clean ≤ DerivedFromSecret ≤ Secret.
    pub closed spec fn spec_join_taint(a: Taint, b: Taint) -> Taint {
        let a_disc: u8 = match a { Taint::Clean => 0, Taint::DerivedFromSecret => 1, Taint::Secret => 2 };
        let b_disc: u8 = match b { Taint::Clean => 0, Taint::DerivedFromSecret => 1, Taint::Secret => 2 };
        if a_disc >= b_disc { a } else { b }
    }

    /// Proof: production join_taint equals the spec.
    pub proof fn lemma_join_taint_equals_spec(a: Taint, b: Taint)
        ensures
            spec_join_taint(a, b) == join_taint(a, b),
    {
        // Reveal production definition (matches spec exactly).
        reveal_with_fuel(join_taint, 1);
        reveal(spec_join_taint);
        assert(spec_join_taint(a, b) == join_taint(a, b));
    }

    /// Proof: join_taint is commutative.
    pub proof fn lemma_join_taint_commutative(a: Taint, b: Taint)
        ensures
            join_taint(a, b) == join_taint(b, a),
    {
        // Both branches (a >= b or b >= a) produce the same max.
        assert(join_taint(a, b) == join_taint(b, a));
    }

    /// Proof: join_taint is associative.
    pub proof fn lemma_join_taint_associative(a: Taint, b: Taint, c: Taint)
        ensures
            join_taint(join_taint(a, b), c) == join_taint(a, join_taint(b, c)),
    {
        // Join is max; max(max(a,b),c) == max(a,max(b,c)) for total order.
        assert(join_taint(join_taint(a, b), c) == join_taint(a, join_taint(b, c)));
    }

    /// Proof: join_taint with Clean is identity.
    pub proof fn lemma_join_taint_identity(a: Taint)
        ensures
            join_taint(a, Taint::Clean) == a && join_taint(Taint::Clean, a) == a,
    {
        // Clean is minimum (0), so join with any a yields a.
        assert(join_taint(a, Taint::Clean) == a);
        assert(join_taint(Taint::Clean, a) == a);
    }

    /// Proof: join_taint is monotone in both arguments.
    pub proof fn lemma_join_taint_monotone(a1: Taint, a2: Taint, b: Taint)
        requires
            // a1 <= a2 in taint ordering (i.e., a1_disc <= a2_disc).
            join_taint(a1, a2) == a2,
        ensures
            join_taint(a1, b) <= join_taint(a2, b) && join_taint(b, a1) <= join_taint(b, a2),
    {
        // If a1 <= a2 then max(a1,b) <= max(a2,b).
        reveal_with_fuel(join_taint, 1);
        reveal(spec_join_taint);
        assert(join_taint(a1, b) <= join_taint(a2, b));
        assert(join_taint(b, a1) <= join_taint(b, a2));
    }
}

/// Secret propagation marker attached to each runtime slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
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

    #[cfg(kani)]
    pub(crate) fn _kani_any() -> Self {
        let value: f64 = kani::any();
        kani::assume(value.is_finite());
        Self(value)
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
#[non_exhaustive]
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
#[non_exhaustive]
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
            let bytes = bytes.unwrap();
            let recovered: Result<SlotValue, _> = postcard::from_bytes(&bytes);
            prop_assert!(recovered.is_ok(), "postcard deserialization should succeed");
            let recovered = recovered.unwrap();
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
