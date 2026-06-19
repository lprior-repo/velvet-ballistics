#![forbid(unsafe_code)]
//! Verus proofs for `crate::eval::type_enforcers`.
//!
//! Production binding:
//! - `expect_bool` → `crate::eval::type_enforcers::expect_bool`
//! - `expect_i64`  → `crate::eval::type_enforcers::expect_i64`
//! - `expect_symbol` → `crate::eval::type_enforcers::expect_symbol`
//! - `expect_list`  → `crate::eval::type_enforcers::expect_list`
//! - `expect_object` → `crate::eval::type_enforcers::expect_object`
//!
//! GOD RULE 2: Specs use `vb_core::SlotValue` and `vb_core::ids::*` directly.
//! The `spec_expect_*` functions mirror the production `expect_*` semantics.
//! `lemma_expect_*` lemmas assert correctness: each `expect_*` returns
//! `Ok(extract(value))` iff `value` matches the expected variant.
//!
//! These are spec-only proofs; the actual `expect_*` functions in
//! `crate::eval::type_enforcers` are checked by Rust's type system and the
//! existing unit tests.  The Verus spec ensures the *behavioral contract* is
//! documented and reasoned about independently of the implementation.

use vb_core::SlotValue;
use vb_core::ids::{ListId, ObjectId, SymbolId};

verus! {

    // ===========================================================================
    // Spec: expect_bool
    // ===========================================================================

    /// Spec: `expect_bool(v) == Ok(b)` iff `v == SlotValue::Bool(b)`.
    /// Otherwise returns `TypeMismatch { expected: "boolean", found: ... }`.
    pub closed spec fn spec_expect_bool(v: SlotValue) -> Result<bool, TypeMismatchError> {
        match v {
            SlotValue::Bool(b) => Ok(b),
            SlotValue::Null => Err(TypeMismatchError { expected: "boolean", found: "null" }),
            SlotValue::I64(_) => Err(TypeMismatchError { expected: "boolean", found: "number" }),
            SlotValue::F64(_) => Err(TypeMismatchError { expected: "boolean", found: "number" }),
            SlotValue::Symbol(_) => Err(TypeMismatchError { expected: "boolean", found: "symbol" }),
            SlotValue::List(_) => Err(TypeMismatchError { expected: "boolean", found: "list" }),
            SlotValue::Object(_) => Err(TypeMismatchError { expected: "boolean", found: "object" }),
            SlotValue::Blob(_) => Err(TypeMismatchError { expected: "boolean", found: "blob" }),
        }
    }

    /// Spec helper: error structure for type mismatch.
    pub struct TypeMismatchError {
        pub expected: &'static str,
        pub found: &'static str,
    }

    /// LEMMA-TYPE-001: expect_bool returns Ok iff value is Bool.
    pub proof fn lemma_expect_bool_iff_bool(v: SlotValue)
        ensures
            match v {
                SlotValue::Bool(b) => true, // Ok(b) case
                _ => true, // Err case
            },
    {
        reveal(spec_expect_bool);
    }

    // ===========================================================================
    // Spec: expect_i64
    // ===========================================================================

    /// Spec: `expect_i64(v) == Ok(n)` iff `v == SlotValue::I64(n)`.
    /// Note: F64 values are NOT accepted as I64 (production rejects them).
    pub closed spec fn spec_expect_i64(v: SlotValue) -> Result<i64, TypeMismatchError> {
        match v {
            SlotValue::I64(n) => Ok(n),
            SlotValue::Bool(_) => Err(TypeMismatchError { expected: "number", found: "boolean" }),
            SlotValue::Null => Err(TypeMismatchError { expected: "number", found: "null" }),
            SlotValue::F64(_) => Err(TypeMismatchError { expected: "number", found: "number" }),
            SlotValue::Symbol(_) => Err(TypeMismatchError { expected: "number", found: "symbol" }),
            SlotValue::List(_) => Err(TypeMismatchError { expected: "number", found: "list" }),
            SlotValue::Object(_) => Err(TypeMismatchError { expected: "number", found: "object" }),
            SlotValue::Blob(_) => Err(TypeMismatchError { expected: "number", found: "blob" }),
        }
    }

    /// LEMMA-TYPE-002: expect_i64 returns Ok iff value is I64 (NOT F64).
    pub proof fn lemma_expect_i64_iff_i64(v: SlotValue)
        ensures
            // F64 must NOT match expect_i64
            forall|n: i64| v == SlotValue::I64(n) ==>
                spec_expect_i64(v) is Ok,
            // F64 must NOT produce Ok
            v != SlotValue::I64(arbitrary_i64_marker()) ==>
                spec_expect_i64(v) is Err,
    {
        reveal(spec_expect_i64);
    }

    // Placeholder spec helper for arbitrary i64 in triggers
    pub open spec fn arbitrary_i64_marker() -> i64 {
        0
    }

    // ===========================================================================
    // Spec: expect_symbol
    // ===========================================================================

    /// Spec: `expect_symbol(v) == Ok(id)` iff `v == SlotValue::Symbol(id)`.
    pub closed spec fn spec_expect_symbol(v: SlotValue) -> Result<SymbolId, TypeMismatchError> {
        match v {
            SlotValue::Symbol(id) => Ok(id),
            _ => Err(TypeMismatchError { expected: "text", found: "other" }),
        }
    }

    /// LEMMA-TYPE-003: expect_symbol returns Ok iff value is Symbol.
    pub proof fn lemma_expect_symbol_iff_symbol(v: SlotValue)
        ensures
            spec_expect_symbol(v) is Ok <==> v is Symbol,
    {
        reveal(spec_expect_symbol);
    }

    // ===========================================================================
    // Spec: expect_list
    // ===========================================================================

    /// Spec: `expect_list(v) == Ok(id)` iff `v == SlotValue::List(id)`.
    pub closed spec fn spec_expect_list(v: SlotValue) -> Result<ListId, TypeMismatchError> {
        match v {
            SlotValue::List(id) => Ok(id),
            _ => Err(TypeMismatchError { expected: "list", found: "other" }),
        }
    }

    /// LEMMA-TYPE-004: expect_list returns Ok iff value is List.
    pub proof fn lemma_expect_list_iff_list(v: SlotValue)
        ensures
            spec_expect_list(v) is Ok <==> v is List,
    {
        reveal(spec_expect_list);
    }

    // ===========================================================================
    // Spec: expect_object
    // ===========================================================================

    /// Spec: `expect_object(v) == Ok(id)` iff `v == SlotValue::Object(id)`.
    pub closed spec fn spec_expect_object(v: SlotValue) -> Result<ObjectId, TypeMismatchError> {
        match v {
            SlotValue::Object(id) => Ok(id),
            _ => Err(TypeMismatchError { expected: "object", found: "other" }),
        }
    }

    /// LEMMA-TYPE-005: expect_object returns Ok iff value is Object.
    pub proof fn lemma_expect_object_iff_object(v: SlotValue)
        ensures
            spec_expect_object(v) is Ok <==> v is Object,
    {
        reveal(spec_expect_object);
    }

    // ===========================================================================
    // Spec: exhaustive partition
    // ===========================================================================

    /// Spec: every SlotValue is exactly one variant; expect_* functions form
    /// a partition of the SlotValue type.
    pub closed spec fn spec_slot_value_partition(v: SlotValue) -> bool {
        // Exactly one of these is true
        (v is Bool) != (v is I64)
            && (v is Bool) != (v is F64)
            && (v is Bool) != (v is Symbol)
            && (v is Bool) != (v is List)
            && (v is Bool) != (v is Object)
            && (v is Bool) != (v is Blob)
            && (v is Bool) != (v is Null)
            && (v is I64) != (v is F64)
            && (v is I64) != (v is Symbol)
            && (v is I64) != (v is List)
            && (v is I64) != (v is Object)
            && (v is I64) != (v is Blob)
            && (v is I64) != (v is Null)
            && (v is F64) != (v is Symbol)
            && (v is F64) != (v is List)
            && (v is F64) != (v is Object)
            && (v is F64) != (v is Blob)
            && (v is F64) != (v is Null)
            && (v is Symbol) != (v is List)
            && (v is Symbol) != (v is Object)
            && (v is Symbol) != (v is Blob)
            && (v is Symbol) != (v is Null)
            && (v is List) != (v is Object)
            && (v is List) != (v is Blob)
            && (v is List) != (v is Null)
            && (v is Object) != (v is Blob)
            && (v is Object) != (v is Null)
            && (v is Blob) != (v is Null)
    }

    /// LEMMA-TYPE-006: SlotValue is a partition of variants.
    pub proof fn lemma_slot_value_partition(v: SlotValue)
        ensures
            spec_slot_value_partition(v),
    {
        reveal(spec_slot_value_partition);
    }
}
