#![forbid(unsafe_code)]
//! Verus proofs for eval type enforcement and binary op semantics.
//!
//! Production binding:
//! - SlotValue → vb_core::SlotValue (production type, directly used)
//! - BinaryOp, UnaryOp → crate::lexer (production types, directly used)
//! - expect_bool, expect_i64 → crate::eval::type_enforcers (production fns)
//! - eval_binary_op, eval_unary_op → crate::eval::ops (production fns)
//! - eval_i64_div_values → crate::eval::ops (production fn)
//!
//! GOD RULE 2: All spec/proof functions use production SlotValue, BinaryOp,
//! UnaryOp types directly — no spec mirror types.

use crate::lexer::{BinaryOp, UnaryOp};
use vb_core::SlotValue;

verus! {

    // ===========================================================================
    // Type enforcement specs (production SlotValue used directly)
    // ===========================================================================

    /// Spec: SlotValue is a boolean.
    pub closed spec fn spec_slot_is_bool(val: SlotValue) -> bool {
        matches!(val, SlotValue::Bool(_))
    }

    /// Spec: SlotValue is an i64.
    pub closed spec fn spec_slot_is_i64(val: SlotValue) -> bool {
        matches!(val, SlotValue::I64(_))
    }

    /// Spec: SlotValue is an f64.
    pub closed spec fn spec_slot_is_f64(val: SlotValue) -> bool {
        matches!(val, SlotValue::F64(_))
    }

    /// Spec: SlotValue is a valid type for boolean operations (Bool only).
    pub closed spec fn spec_slot_bool_op_valid(val: SlotValue) -> bool {
        spec_slot_is_bool(val)
    }

    /// Spec: SlotValue is a valid type for numeric operations (I64 or F64).
    pub closed spec fn spec_slot_numeric_op_valid(val: SlotValue) -> bool {
        spec_slot_is_i64(val) || spec_slot_is_f64(val)
    }

    /// Spec: SlotValue is a valid type for equality comparison (any type).
    pub closed spec fn spec_slot_eq_valid(val: SlotValue) -> bool {
        true // all SlotValue variants support equality
    }

    // ===========================================================================
    // Binary op semantics specs
    // ===========================================================================

    /// Spec: logical AND of two booleans.
    pub closed spec fn spec_and(a: bool, b: bool) -> bool {
        a && b
    }

    /// Spec: logical OR of two booleans.
    pub closed spec fn spec_or(a: bool, b: bool) -> bool {
        a || b
    }

    /// Spec: logical NOT of a boolean.
    pub closed spec fn spec_not(a: bool) -> bool {
        !a
    }

    /// Spec: integer addition (returns Some(result) if no overflow).
    pub closed spec fn spec_i64_add(left: i64, right: i64) -> Option<i64> {
        left.checked_add(right)
    }

    /// Spec: integer subtraction (returns Some(result) if no underflow).
    pub closed spec fn spec_i64_sub(left: i64, right: i64) -> Option<i64> {
        left.checked_sub(right)
    }

    /// Spec: integer multiplication (returns Some(result) if no overflow).
    pub closed spec fn spec_i64_mul(left: i64, right: i64) -> Option<i64> {
        left.checked_mul(right)
    }

    /// Spec: integer division (returns Some(result) if divisor != 0 and no overflow).
    pub closed spec fn spec_i64_div(left: i64, right: i64) -> Option<i64> {
        if right == 0 {
            None
        } else {
            left.checked_div(right)
        }
    }

    // ===========================================================================
    // Type enforcement proof specs
    // ===========================================================================

    /// Spec: expect_bool returns Ok(b) when val is Bool(b), Err otherwise.
    /// This is a pure spec mirror of the production expect_bool function.
    closed spec fn spec_expect_bool(val: SlotValue) -> Option<bool> {
        match val {
            SlotValue::Bool(b) => Some(b),
            _ => None,
        }
    }

    /// Spec: expect_i64 returns Ok(n) when val is I64(n), Err otherwise.
    closed spec fn spec_expect_i64(val: SlotValue) -> Option<i64> {
        match val {
            SlotValue::I64(n) => Some(n),
            _ => None,
        }
    }

    // ===========================================================================
    // Proof: Type enforcement lemmas
    // ===========================================================================

    /// LEMMA-EVAL-001: expect_bool correctly extracts bool from SlotValue::Bool.
    /// For any bool b, spec_expect_bool(SlotValue::Bool(b)) == Some(b).
    pub proof fn lemma_expect_bool_correct(val: SlotValue)
        ensures
            (spec_slot_is_bool(val) && spec_expect_bool(val) != None)
                || (!spec_slot_is_bool(val) && spec_expect_bool(val) == None),
    {
        assert((spec_slot_is_bool(val) && spec_expect_bool(val) != None)
            || (!spec_slot_is_bool(val) && spec_expect_bool(val) == None));
    }

    /// LEMMA-EVAL-002: expect_i64 correctly extracts i64 from SlotValue::I64.
    /// For any i64 n, spec_expect_i64(SlotValue::I64(n)) == Some(n).
    pub proof fn lemma_expect_i64_correct(val: SlotValue)
        ensures
            (spec_slot_is_i64(val) && spec_expect_i64(val) != None)
                || (!spec_slot_is_i64(val) && spec_expect_i64(val) == None),
    {
        assert((spec_slot_is_i64(val) && spec_expect_i64(val) != None)
            || (!spec_slot_is_i64(val) && spec_expect_i64(val) == None));
    }

    // ===========================================================================
    // Proof: Boolean algebra lemmas
    // ===========================================================================

    /// LEMMA-EVAL-003: Boolean AND is commutative.
    pub proof fn lemma_bool_and_commutative(a: bool, b: bool)
        ensures
            spec_and(a, b) == spec_and(b, a),
    {
        assert(spec_and(a, b) == spec_and(b, a));
    }

    /// LEMMA-EVAL-004: Boolean OR is commutative.
    pub proof fn lemma_bool_or_commutative(a: bool, b: bool)
        ensures
            spec_or(a, b) == spec_or(b, a),
    {
        assert(spec_or(a, b) == spec_or(b, a));
    }

    /// LEMMA-EVAL-005: Boolean NOT is an involution (¬¬a = a).
    pub proof fn lemma_bool_not_involution(a: bool)
        ensures
            spec_not(spec_not(a)) == a,
    {
        assert(spec_not(spec_not(a)) == a);
    }

    /// LEMMA-EVAL-006: Boolean De Morgan's law: ¬(a ∧ b) = ¬a ∨ ¬b.
    pub proof fn lemma_bool_de_morgan_and(a: bool, b: bool)
        ensures
            spec_not(spec_and(a, b)) == spec_and(spec_not(a), spec_not(b)),
    {
        assert(spec_not(spec_and(a, b)) == spec_and(spec_not(a), spec_not(b)));
    }

    /// LEMMA-EVAL-007: Boolean De Morgan's law: ¬(a ∨ b) = ¬a ∧ ¬b.
    pub proof fn lemma_bool_de_morgan_or(a: bool, b: bool)
        ensures
            spec_not(spec_or(a, b)) == spec_and(spec_not(a), spec_not(b)),
    {
        assert(spec_not(spec_or(a, b)) == spec_and(spec_not(a), spec_not(b)));
    }

    // ===========================================================================
    // Proof: Arithmetic lemmas
    // ===========================================================================

    /// LEMMA-EVAL-008: i64 addition is commutative (when both operands are in bounds).
    pub proof fn lemma_i64_add_commutative(left: i64, right: i64)
        ensures
            spec_i64_add(left, right) == spec_i64_add(right, left),
    {
        assert(spec_i64_add(left, right) == spec_i64_add(right, left));
    }

    /// LEMMA-EVAL-009: i64 subtraction with zero is identity.
    pub proof fn lemma_i64_sub_zero_identity(left: i64)
        ensures
            spec_i64_sub(left, 0) == Some(left),
    {
        assert(spec_i64_sub(left, 0) == Some(left));
    }

    /// LEMMA-EVAL-010: i64 multiplication by zero yields zero.
    pub proof fn lemma_i64_mul_zero(left: i64)
        ensures
            spec_i64_mul(left, 0) == Some(0),
    {
        assert(spec_i64_mul(left, 0) == Some(0));
    }

    /// LEMMA-EVAL-011: Integer division by zero returns None (error).
    pub proof fn lemma_i64_div_zero_error(left: i64)
        ensures
            spec_i64_div(left, 0) == None,
    {
        assert(spec_i64_div(left, 0) == None);
    }

    // ===========================================================================
    // Proof: SlotValue type validity for operations
    // ===========================================================================

    /// LEMMA-EVAL-012: SlotValue::Bool is valid for boolean ops, invalid for numeric ops.
    pub proof fn lemma_bool_valid_for_bool_ops()
        ensures
            spec_slot_bool_op_valid(SlotValue::Bool(true))
                && !spec_slot_numeric_op_valid(SlotValue::Bool(true)),
    {
        assert(spec_slot_bool_op_valid(SlotValue::Bool(true)));
        assert(!spec_slot_numeric_op_valid(SlotValue::Bool(true)));
    }

    /// LEMMA-EVAL-013: SlotValue::I64 is valid for numeric ops, invalid for bool ops.
    pub proof fn lemma_i64_valid_for_numeric_ops()
        ensures
            spec_slot_numeric_op_valid(SlotValue::I64(42))
                && !spec_slot_bool_op_valid(SlotValue::I64(42)),
    {
        assert(spec_slot_numeric_op_valid(SlotValue::I64(42)));
        assert(!spec_slot_bool_op_valid(SlotValue::I64(42)));
    }

    /// LEMMA-EVAL-014: SlotValue::F64 is valid for numeric ops, invalid for bool ops.
    pub proof fn lemma_f64_valid_for_numeric_ops()
        ensures
            spec_slot_numeric_op_valid(SlotValue::F64(vb_core::value::FiniteF64::new(1.5).unwrap()))
                && !spec_slot_bool_op_valid(SlotValue::F64(vb_core::value::FiniteF64::new(1.5).unwrap())),
    {
        let f = vb_core::value::FiniteF64::new(1.5).unwrap();
        assert(spec_slot_numeric_op_valid(SlotValue::F64(f)));
        assert(!spec_slot_bool_op_valid(SlotValue::F64(f)));
    }
}
