#![forbid(unsafe_code)]
//! Kani harnesses for `vb_expr::eval::type_enforcers` — vb-bc33k.
//!
//! Production targets (in `crate::eval::type_enforcers`):
//! - `expect_bool`     ↔ `spec_expect_bool`     (LEMMA-TYPE-001)
//! - `expect_i64`      ↔ `spec_expect_i64`      (LEMMA-TYPE-002)
//! - `expect_symbol`   ↔ `spec_expect_symbol`   (LEMMA-TYPE-003)
//! - `expect_list`     ↔ `spec_expect_list`     (LEMMA-TYPE-004)
//! - `expect_object`   ↔ `spec_expect_object`   (LEMMA-TYPE-005)
//!
//! Each harness uses `kani::any()` to enumerate every SlotValue variant and
//! asserts the iff-correctness invariant the Verus spec captures.

use crate::ExprError;
use crate::eval::type_enforcers::{
    expect_bool, expect_i64, expect_list, expect_object, expect_symbol,
};
use vb_core::SlotValue;
use vb_core::ids::{BlobId, ListId, ObjectId, SymbolId};

/// Kani-any helper that produces an arbitrary SlotValue covering all variants.
fn kani_any_slot_value() -> SlotValue {
    // 0..7 picks the variant; per-variant payload is kani::any.
    let tag: u8 = kani::any();
    match tag % 8 {
        0 => SlotValue::Null,
        1 => SlotValue::Bool(kani::any()),
        2 => SlotValue::I64(kani::any()),
        3 => SlotValue::Symbol(SymbolId::new(kani::any::<u32>())),
        4 => SlotValue::List(ListId::new(kani::any::<u32>())),
        5 => SlotValue::Object(ObjectId::new(kani::any::<u32>())),
        6 => SlotValue::Blob(BlobId::new(kani::any::<u64>())),
        _ => SlotValue::Null,
    }
}

/// Kani harness for LEMMA-TYPE-001: expect_bool iff value is Bool.
#[kani::proof]
#[kani::unwind(8)]
fn kani_type_enforcer_expect_bool_iff_bool() {
    let value: SlotValue = kani_any_slot_value();
    let result = expect_bool(value);
    match value {
        SlotValue::Bool(b) => {
            kani::assert(result.is_ok(), "expect_bool must accept Bool");
            let Ok(recovered) = result else {
                return;
            };
            kani::assert(
                recovered == b,
                "expect_bool must round-trip the inner bool",
            );
        }
        _ => {
            kani::assert(result.is_err(), "expect_bool must reject non-Bool");
            let Err(ExprError::TypeMismatch { expected, found }) = result else {
                return;
            };
            kani::assert(expected == "boolean", "expected type is 'boolean'");
            kani::assert(
                found == value.type_name(),
                "found type matches SlotValue::type_name",
            );
        }
    }
}

/// Kani harness for LEMMA-TYPE-002: expect_i64 iff value is I64 (NOT F64).
#[kani::proof]
#[kani::unwind(8)]
fn kani_type_enforcer_expect_i64_iff_i64() {
    let value: SlotValue = kani_any_slot_value();
    let result = expect_i64(value);
    match value {
        SlotValue::I64(n) => {
            kani::assert(result.is_ok(), "expect_i64 must accept I64");
            let Ok(recovered) = result else {
                return;
            };
            kani::assert(
                recovered == n,
                "expect_i64 must round-trip the inner i64",
            );
        }
        _ => {
            kani::assert(result.is_err(), "expect_i64 must reject non-I64");
            let Err(ExprError::TypeMismatch { expected, .. }) = result else {
                return;
            };
            kani::assert(expected == "number", "expected type is 'number'");
        }
    }
}

/// Kani harness for LEMMA-TYPE-003: expect_symbol iff value is Symbol.
#[kani::proof]
#[kani::unwind(8)]
fn kani_type_enforcer_expect_symbol_iff_symbol() {
    let value: SlotValue = kani_any_slot_value();
    let result = expect_symbol(value);
    match value {
        SlotValue::Symbol(id) => {
            kani::assert(result.is_ok(), "expect_symbol must accept Symbol");
            let Ok(recovered) = result else {
                return;
            };
            kani::assert(
                recovered == id,
                "expect_symbol must round-trip the SymbolId",
            );
        }
        _ => {
            kani::assert(result.is_err(), "expect_symbol must reject non-Symbol");
            let Err(ExprError::TypeMismatch { expected, .. }) = result else {
                return;
            };
            kani::assert(expected == "text", "expected type is 'text'");
        }
    }
}

/// Kani harness for LEMMA-TYPE-004: expect_list iff value is List.
#[kani::proof]
#[kani::unwind(8)]
fn kani_type_enforcer_expect_list_iff_list() {
    let value: SlotValue = kani_any_slot_value();
    let result = expect_list(value);
    match value {
        SlotValue::List(id) => {
            kani::assert(result.is_ok(), "expect_list must accept List");
            let Ok(recovered) = result else {
                return;
            };
            kani::assert(
                recovered == id,
                "expect_list must round-trip the ListId",
            );
        }
        _ => {
            kani::assert(result.is_err(), "expect_list must reject non-List");
            let Err(ExprError::TypeMismatch { expected, .. }) = result else {
                return;
            };
            kani::assert(expected == "list", "expected type is 'list'");
        }
    }
}

/// Kani harness for LEMMA-TYPE-005: expect_object iff value is Object.
#[kani::proof]
#[kani::unwind(8)]
fn kani_type_enforcer_expect_object_iff_object() {
    let value: SlotValue = kani_any_slot_value();
    let result = expect_object(value);
    match value {
        SlotValue::Object(id) => {
            kani::assert(result.is_ok(), "expect_object must accept Object");
            let Ok(recovered) = result else {
                return;
            };
            kani::assert(
                recovered == id,
                "expect_object must round-trip the ObjectId",
            );
        }
        _ => {
            kani::assert(
                result.is_err(),
                "expect_object must reject non-Object",
            );
            let Err(ExprError::TypeMismatch { expected, .. }) = result else {
                return;
            };
            kani::assert(expected == "object", "expected type is 'object'");
        }
    }
}

/// Kani harness for LEMMA-TYPE-006: SlotValue partition — at most one
/// expect_* accepts a given value (Null rejects all).
#[kani::proof]
#[kani::unwind(8)]
fn kani_type_enforcer_partition_invariant() {
    let value: SlotValue = kani_any_slot_value();
    let ok_count: u32 = u32::from(expect_bool(value).is_ok())
        + u32::from(expect_i64(value).is_ok())
        + u32::from(expect_symbol(value).is_ok())
        + u32::from(expect_list(value).is_ok())
        + u32::from(expect_object(value).is_ok());
    kani::assert(
        ok_count <= 1,
        "at most one type_enforcer accepts a given value",
    );
}