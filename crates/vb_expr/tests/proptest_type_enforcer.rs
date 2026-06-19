//! Proptest property suite for `vb_expr::eval::type_enforcers`.
//!
//! Bead: vb-bc33k / vb-987b9 — production binding closure for the 5
//! Verus `spec_expect_*` functions in `crates/vb_expr/src/eval/verus.rs`.
//!
//! Each property mirrors a Verus spec:
//!
//! - LEMMA-TYPE-001 (`spec_expect_bool`):
//!     `expect_bool(v) == Ok(b)` iff `v == SlotValue::Bool(b)`,
//!     otherwise `Err(TypeMismatch { expected: "boolean", found: type_name(v) })`.
//! - LEMMA-TYPE-002 (`spec_expect_i64`):
//!     `expect_i64(v) == Ok(n)` iff `v == SlotValue::I64(n)`.
//!     F64 values are explicitly rejected (production behavior).
//! - LEMMA-TYPE-003 (`spec_expect_symbol`):
//!     `expect_symbol(v) == Ok(id)` iff `v == SlotValue::Symbol(id)`.
//! - LEMMA-TYPE-004 (`spec_expect_list`):
//!     `expect_list(v) == Ok(id)` iff `v == SlotValue::List(id)`.
//! - LEMMA-TYPE-005 (`spec_expect_object`):
//!     `expect_object(v) == Ok(id)` iff `v == SlotValue::Object(id)`.
//!
//! Production binding: each property exercises the actual
//! `crate::eval::type_enforcers::expect_*` exec fn via the `#[doc(hidden)]`
//! re-exports in `lib.rs`. The proptests verify that the production exec
//! fns implement exactly the iff-correctness contract their Verus specs
//! claim.  Together with the Kani harnesses in
//! `crates/vb_expr/src/kani/vb_bc33k_type_enforcer.rs` these tests close
//! the L1/L3 lanes for VB-EXPR-TYPE-001..005.

#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::SlotValue;
use vb_core::ids::{BlobId, ListId, ObjectId, SymbolId};
use vb_core::value::FiniteF64;
use vb_expr::{
    ExprError, ExprResult, expect_bool, expect_i64, expect_list, expect_object, expect_symbol,
};

/// Strategy: any finite raw f64.  Used to construct `SlotValue::F64`.
fn arb_finite_f64_raw() -> impl Strategy<Value = f64> {
    any::<f64>().prop_filter("must be finite", |f| f.is_finite())
}

/// Strategy: a `SlotValue::F64` carrying a finite f64.  Filters NaN/Inf
/// and maps via `FiniteF64::new` (which is guaranteed to succeed for
/// filtered inputs); the `Err` arm is mapped to `SlotValue::Null` as a
/// fallback that the cross-property tests will then reject via the
/// expected `TypeMismatch`.
fn arb_f64_slot() -> impl Strategy<Value = SlotValue> {
    arb_finite_f64_raw().prop_map(|f| match FiniteF64::new(f) {
        Ok(f) => SlotValue::F64(f),
        Err(_) => SlotValue::Null,
    })
}

/// Strategy: a `SlotValue` covering every variant.  Symbol/List/Object
/// IDs are constrained to small ranges to avoid overlapping arena IDs
/// from biasing the rejection-property assertions.
fn arb_slot_value() -> impl Strategy<Value = SlotValue> {
    prop_oneof![
        Just(SlotValue::Null),
        any::<bool>().prop_map(SlotValue::Bool),
        any::<i64>().prop_map(SlotValue::I64),
        arb_f64_slot(),
        (0u32..1024).prop_map(|id| SlotValue::Symbol(SymbolId::new(id))),
        (0u32..1024).prop_map(|id| SlotValue::List(ListId::new(id))),
        (0u32..1024).prop_map(|id| SlotValue::Object(ObjectId::new(id))),
        (0u32..1024).prop_map(|id| SlotValue::Blob(BlobId::new(u64::from(id)))),
    ]
}

// ----------------------------------------------------------------------------
// LEMMA-TYPE-001: expect_bool iff value is Bool.
// ----------------------------------------------------------------------------

proptest! {
    /// spec_expect_bool round-trip: every Bool yields Ok(b).
    #[test]
    fn expect_bool_roundtrips_any_bool(input in any::<bool>()) {
        let value = SlotValue::Bool(input);
        match expect_bool(value) {
            Ok(recovered) => prop_assert_eq!(recovered, input),
            Err(err) => {
                let err_msg = format!("{:?}", err);
                prop_assert!(false, "expect_bool must accept Bool: {}", err_msg);
            }
        }
    }

    /// spec_expect_bool rejection: every non-Bool yields TypeMismatch
    /// `{ expected: "boolean", found: type_name(v) }`.
    #[test]
    fn expect_bool_rejects_non_bool(value in arb_slot_value()) {
        if matches!(value, SlotValue::Bool(_)) {
            return Ok(());
        }
        match expect_bool(value) {
            Err(ExprError::TypeMismatch { expected, found }) => {
                prop_assert_eq!(expected, "boolean");
                prop_assert_eq!(found, value.type_name());
            }
            Err(other) => {
                let msg = format!("{:?}", other);
                prop_assert!(false, "expected TypeMismatch, got {}", msg);
            }
            Ok(other) => {
                let msg = format!("{:?}", other);
                prop_assert!(false, "expect_bool must reject {:?}, got Ok({})", value, msg);
            }
        }
    }

    /// spec_expect_bool total: never panics, always returns Result.
    #[test]
    fn expect_bool_is_total(value in arb_slot_value()) {
        let _result: ExprResult<bool> = expect_bool(value);
    }
}

// ----------------------------------------------------------------------------
// LEMMA-TYPE-002: expect_i64 iff value is I64 (NOT F64).
// ----------------------------------------------------------------------------

proptest! {
    /// spec_expect_i64 round-trip: every I64 yields Ok(n).
    #[test]
    fn expect_i64_roundtrips_any_i64(input in any::<i64>()) {
        let value = SlotValue::I64(input);
        match expect_i64(value) {
            Ok(recovered) => prop_assert_eq!(recovered, input),
            Err(err) => {
                let err_msg = format!("{:?}", err);
                prop_assert!(false, "expect_i64 must accept I64: {}", err_msg);
            }
        }
    }

    /// spec_expect_i64 rejection: every non-I64 (including F64) yields TypeMismatch
    /// `{ expected: "number", found: type_name(v) }`. The `found` label for
    /// both I64 and F64 is `"number"` because production collapses them.
    #[test]
    fn expect_i64_rejects_non_i64(value in arb_slot_value()) {
        if matches!(value, SlotValue::I64(_)) {
            return Ok(());
        }
        match expect_i64(value) {
            Err(ExprError::TypeMismatch { expected, found }) => {
                prop_assert_eq!(expected, "number");
                prop_assert_eq!(found, value.type_name());
            }
            Err(other) => {
                let msg = format!("{:?}", other);
                prop_assert!(false, "expected TypeMismatch, got {}", msg);
            }
            Ok(other) => {
                let msg = format!("{:?}", other);
                prop_assert!(false, "expect_i64 must reject {:?}, got Ok({})", value, msg);
            }
        }
    }

    /// spec_expect_i64 F64 rejection: F64 must NOT be coerced to I64.
    /// This is the explicit F64 rejection note in the spec.
    #[test]
    fn expect_i64_explicitly_rejects_f64(value in arb_finite_f64_raw()) {
        let slot = match FiniteF64::new(value) {
            Ok(f) => SlotValue::F64(f),
            Err(_) => SlotValue::Null,
        };
        match expect_i64(slot) {
            Err(ExprError::TypeMismatch { expected, found }) => {
                prop_assert_eq!(expected, "number");
                prop_assert_eq!(found, "number");
            }
            other => {
                let msg = format!("{:?}", other);
                prop_assert!(false, "F64 must be rejected, got {}", msg);
            }
        }
    }

    /// spec_expect_i64 total: never panics.
    #[test]
    fn expect_i64_is_total(value in arb_slot_value()) {
        let _result: ExprResult<i64> = expect_i64(value);
    }
}

// ----------------------------------------------------------------------------
// LEMMA-TYPE-003: expect_symbol iff value is Symbol.
// ----------------------------------------------------------------------------

proptest! {
    /// spec_expect_symbol round-trip: every Symbol yields Ok(id).
    #[test]
    fn expect_symbol_roundtrips_any_id(id in any::<u32>()) {
        let symbol_id = SymbolId::new(id);
        let value = SlotValue::Symbol(symbol_id);
        match expect_symbol(value) {
            Ok(recovered) => prop_assert_eq!(recovered, symbol_id),
            Err(err) => {
                let err_msg = format!("{:?}", err);
                prop_assert!(false, "expect_symbol must accept Symbol: {}", err_msg);
            }
        }
    }

    /// spec_expect_symbol rejection: every non-Symbol yields TypeMismatch
    /// `{ expected: "text", found: type_name(v) }`.
    #[test]
    fn expect_symbol_rejects_non_symbol(value in arb_slot_value()) {
        if matches!(value, SlotValue::Symbol(_)) {
            return Ok(());
        }
        match expect_symbol(value) {
            Err(ExprError::TypeMismatch { expected, found }) => {
                prop_assert_eq!(expected, "text");
                prop_assert_eq!(found, value.type_name());
            }
            Err(other) => {
                let msg = format!("{:?}", other);
                prop_assert!(false, "expected TypeMismatch, got {}", msg);
            }
            Ok(other) => {
                let msg = format!("{:?}", other);
                prop_assert!(false, "expect_symbol must reject {:?}, got Ok({})", value, msg);
            }
        }
    }

    /// spec_expect_symbol total: never panics.
    #[test]
    fn expect_symbol_is_total(value in arb_slot_value()) {
        let _result: ExprResult<SymbolId> = expect_symbol(value);
    }
}

// ----------------------------------------------------------------------------
// LEMMA-TYPE-004: expect_list iff value is List.
// ----------------------------------------------------------------------------

proptest! {
    /// spec_expect_list round-trip: every List yields Ok(id).
    #[test]
    fn expect_list_roundtrips_any_id(id in any::<u32>()) {
        let list_id = ListId::new(id);
        let value = SlotValue::List(list_id);
        match expect_list(value) {
            Ok(recovered) => prop_assert_eq!(recovered, list_id),
            Err(err) => {
                let err_msg = format!("{:?}", err);
                prop_assert!(false, "expect_list must accept List: {}", err_msg);
            }
        }
    }

    /// spec_expect_list rejection: every non-List yields TypeMismatch
    /// `{ expected: "list", found: type_name(v) }`.
    #[test]
    fn expect_list_rejects_non_list(value in arb_slot_value()) {
        if matches!(value, SlotValue::List(_)) {
            return Ok(());
        }
        match expect_list(value) {
            Err(ExprError::TypeMismatch { expected, found }) => {
                prop_assert_eq!(expected, "list");
                prop_assert_eq!(found, value.type_name());
            }
            Err(other) => {
                let msg = format!("{:?}", other);
                prop_assert!(false, "expected TypeMismatch, got {}", msg);
            }
            Ok(other) => {
                let msg = format!("{:?}", other);
                prop_assert!(false, "expect_list must reject {:?}, got Ok({})", value, msg);
            }
        }
    }

    /// spec_expect_list total: never panics.
    #[test]
    fn expect_list_is_total(value in arb_slot_value()) {
        let _result: ExprResult<ListId> = expect_list(value);
    }
}

// ----------------------------------------------------------------------------
// LEMMA-TYPE-005: expect_object iff value is Object.
// ----------------------------------------------------------------------------

proptest! {
    /// spec_expect_object round-trip: every Object yields Ok(id).
    #[test]
    fn expect_object_roundtrips_any_id(id in any::<u32>()) {
        let object_id = ObjectId::new(id);
        let value = SlotValue::Object(object_id);
        match expect_object(value) {
            Ok(recovered) => prop_assert_eq!(recovered, object_id),
            Err(err) => {
                let err_msg = format!("{:?}", err);
                prop_assert!(false, "expect_object must accept Object: {}", err_msg);
            }
        }
    }

    /// spec_expect_object rejection: every non-Object yields TypeMismatch
    /// `{ expected: "object", found: type_name(v) }`.
    #[test]
    fn expect_object_rejects_non_object(value in arb_slot_value()) {
        if matches!(value, SlotValue::Object(_)) {
            return Ok(());
        }
        match expect_object(value) {
            Err(ExprError::TypeMismatch { expected, found }) => {
                prop_assert_eq!(expected, "object");
                prop_assert_eq!(found, value.type_name());
            }
            Err(other) => {
                let msg = format!("{:?}", other);
                prop_assert!(false, "expected TypeMismatch, got {}", msg);
            }
            Ok(other) => {
                let msg = format!("{:?}", other);
                prop_assert!(false, "expect_object must reject {:?}, got Ok({})", value, msg);
            }
        }
    }

    /// spec_expect_object total: never panics.
    #[test]
    fn expect_object_is_total(value in arb_slot_value()) {
        let _result: ExprResult<ObjectId> = expect_object(value);
    }
}

// ----------------------------------------------------------------------------
// LEMMA-TYPE-006 (cross-property): iff-correctness invariant.
// The 5 expect_* functions form a partition: at most one accepts any
// SlotValue, and exactly one accepts the matching variant.
// ----------------------------------------------------------------------------

proptest! {
    /// Cross-property: at most one expect_* accepts any SlotValue.
    /// Verifies the partition property across all 5 type_enforcers.
    #[test]
    fn exactly_zero_or_one_enforcers_accept(value in arb_slot_value()) {
        let ok_bool = expect_bool(value).is_ok();
        let ok_i64 = expect_i64(value).is_ok();
        let ok_symbol = expect_symbol(value).is_ok();
        let ok_list = expect_list(value).is_ok();
        let ok_object = expect_object(value).is_ok();

        let ok_count = [ok_bool, ok_i64, ok_symbol, ok_list, ok_object]
            .iter()
            .filter(|b| **b)
            .count();

        prop_assert!(
            ok_count <= 1,
            "iff-correctness violated: {:?} accepted by {} enforcers",
            value,
            ok_count
        );
    }

    /// Cross-property: Blob is rejected by all 5 enforcers.
    /// Verifies the LEMMA-TYPE-006 partition coverage of variants that
    /// no expect_* matches.
    #[test]
    fn blob_rejected_by_all_enforcers(id in 0u32..1024) {
        let value = SlotValue::Blob(BlobId::new(u64::from(id)));
        prop_assert!(expect_bool(value).is_err(), "blob must be rejected by expect_bool");
        prop_assert!(expect_i64(value).is_err(), "blob must be rejected by expect_i64");
        prop_assert!(expect_symbol(value).is_err(), "blob must be rejected by expect_symbol");
        prop_assert!(expect_list(value).is_err(), "blob must be rejected by expect_list");
        prop_assert!(expect_object(value).is_err(), "blob must be rejected by expect_object");
    }

    /// Cross-property: Null is rejected by all 5 enforcers.
    #[test]
    fn null_rejected_by_all_enforcers(_unit in Just(())) {
        let value = SlotValue::Null;
        prop_assert!(expect_bool(value).is_err());
        prop_assert!(expect_i64(value).is_err());
        prop_assert!(expect_symbol(value).is_err());
        prop_assert!(expect_list(value).is_err());
        prop_assert!(expect_object(value).is_err());
    }
}
