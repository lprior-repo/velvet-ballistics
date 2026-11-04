//! Comprehensive property-based tests for expression operator evaluation.
//!
//! These tests verify mathematical properties of expression operators:
//! commutativity, associativity, identity, distributivity, and boundary behavior.

use crate::eval::{BinaryOp, UnaryOp, eval_binary_op, eval_unary_op};
use vb_core::{FiniteF64, SlotValue};

use proptest::prelude::*;

// Property: Addition is commutative for i64.
proptest! {
    #[test]
    fn addition_is_commutative(a in -1000000i64..=1000000i64, b in -1000000i64..=1000000i64) {
        let result_ab = eval_binary_op(BinaryOp::Add, SlotValue::I64(a), SlotValue::I64(b));
        let result_ba = eval_binary_op(BinaryOp::Add, SlotValue::I64(b), SlotValue::I64(a));
        assert_eq!(result_ab, result_ba);
    }
}

// Property: Multiplication is commutative for i64.
proptest! {
    #[test]
    fn multiplication_is_commutative(a in -1000000i64..=1000000i64, b in -1000000i64..=1000000i64) {
        let result_ab = eval_binary_op(BinaryOp::Mul, SlotValue::I64(a), SlotValue::I64(b));
        let result_ba = eval_binary_op(BinaryOp::Mul, SlotValue::I64(b), SlotValue::I64(a));
        assert_eq!(result_ab, result_ba);
    }
}

// Property: Addition is associative for i64.
proptest! {
    #[test]
    fn addition_is_associative(a in -1000000i64..=1000000i64, b in -1000000i64..=1000000i64, c in -1000000i64..=1000000i64) {
        let left = eval_binary_op(BinaryOp::Add, SlotValue::I64(a), SlotValue::I64(b)).unwrap_or(SlotValue::Null);
        let result_left = eval_binary_op(BinaryOp::Add, left, SlotValue::I64(c));

        let right = eval_binary_op(BinaryOp::Add, SlotValue::I64(b), SlotValue::I64(c)).unwrap_or(SlotValue::Null);
        let result_right = eval_binary_op(BinaryOp::Add, SlotValue::I64(a), right);

        assert_eq!(result_left, result_right);
    }
}

// Property: Addition identity: a + 0 == a.
proptest! {
    #[test]
    fn addition_identity(a in -1000000i64..=1000000i64) {
        let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(a), SlotValue::I64(0));
        assert_eq!(result, Ok(SlotValue::I64(a)));
    }
}

// Property: Multiplication identity: a * 1 == a.
proptest! {
    #[test]
    fn multiplication_identity(a in -1000000i64..=1000000i64) {
        let result = eval_binary_op(BinaryOp::Mul, SlotValue::I64(a), SlotValue::I64(1));
        assert_eq!(result, Ok(SlotValue::I64(a)));
    }
}

// Property: Multiplication by zero: a * 0 == 0.
proptest! {
    #[test]
    fn multiplication_by_zero(a in -1000000i64..=1000000i64) {
        let result = eval_binary_op(BinaryOp::Mul, SlotValue::I64(a), SlotValue::I64(0));
        assert_eq!(result, Ok(SlotValue::I64(0)));
    }
}

// Property: Subtraction of self: a - a == 0.
proptest! {
    #[test]
    fn subtraction_self_is_zero(a in -1000000i64..=1000000i64) {
        let result = eval_binary_op(BinaryOp::Sub, SlotValue::I64(a), SlotValue::I64(a));
        assert_eq!(result, Ok(SlotValue::I64(0)));
    }
}

// Property: Division of self: a / a == 1 (for non-zero a, all signs).
proptest! {
    #[test]
    fn division_self_is_one(a in any::<i64>()) {
        prop_assume!(a != 0);
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(a), SlotValue::I64(a));
        assert_eq!(result, Ok(SlotValue::I64(1)));
    }
}

// Property: Multiplication distributes over addition.
proptest! {
    #[test]
    fn multiplication_distributes_over_addition(a in -1000000i64..=1000000i64, b in -1000000i64..=1000000i64, c in -1000000i64..=1000000i64) {
        let left_sum = eval_binary_op(BinaryOp::Add, SlotValue::I64(b), SlotValue::I64(c)).unwrap_or(SlotValue::Null);
        let result_left = eval_binary_op(BinaryOp::Mul, SlotValue::I64(a), left_sum);

        let term1 = eval_binary_op(BinaryOp::Mul, SlotValue::I64(a), SlotValue::I64(b)).unwrap_or(SlotValue::Null);
        let term2 = eval_binary_op(BinaryOp::Mul, SlotValue::I64(a), SlotValue::I64(c)).unwrap_or(SlotValue::Null);
        let result_right = eval_binary_op(BinaryOp::Add, term1, term2);

        assert_eq!(result_left, result_right);
    }
}

// Property: Division by one is identity.
proptest! {
    #[test]
    fn division_by_one_is_identity(a in -1000000i64..=1000000i64) {
        let result = eval_binary_op(BinaryOp::Div, SlotValue::I64(a), SlotValue::I64(1));
        assert_eq!(result, Ok(SlotValue::I64(a)));
    }
}

// Property: Double negation is identity.
proptest! {
    #[test]
    fn double_negation_is_identity(a in -1000000i64..=1000000i64) {
        let neg = eval_unary_op(UnaryOp::Neg, SlotValue::I64(a)).unwrap_or(SlotValue::Null);
        let result = eval_unary_op(UnaryOp::Neg, neg);
        assert_eq!(result, Ok(SlotValue::I64(a)));
    }
}

// Property: Negation is inverse: a + (-a) == 0.
proptest! {
    #[test]
    fn negation_is_inverse(a in -1000000i64..=1000000i64) {
        let neg = eval_unary_op(UnaryOp::Neg, SlotValue::I64(a)).unwrap_or(SlotValue::Null);
        let result = eval_binary_op(BinaryOp::Add, SlotValue::I64(a), neg);
        assert_eq!(result, Ok(SlotValue::I64(0)));
    }
}

// Property: F64 addition is commutative.
proptest! {
    #[test]
    fn f64_addition_is_commutative(a in -1000000f64..=1000000f64, b in -1000000f64..=1000000f64) {
        let va = FiniteF64::new(a).unwrap_or(FiniteF64::new(0.0).unwrap());
        let vb = FiniteF64::new(b).unwrap_or(FiniteF64::new(0.0).unwrap());
        let result_ab = eval_binary_op(BinaryOp::Add, SlotValue::F64(va), SlotValue::F64(vb));
        let result_ba = eval_binary_op(BinaryOp::Add, SlotValue::F64(vb), SlotValue::F64(va));
        assert_eq!(result_ab, result_ba);
    }
}

// Property: F64 multiplication is commutative.
proptest! {
    #[test]
    fn f64_multiplication_is_commutative(a in -1000000f64..=1000000f64, b in -1000000f64..=1000000f64) {
        let va = FiniteF64::new(a).unwrap_or(FiniteF64::new(0.0).unwrap());
        let vb = FiniteF64::new(b).unwrap_or(FiniteF64::new(0.0).unwrap());
        let result_ab = eval_binary_op(BinaryOp::Mul, SlotValue::F64(va), SlotValue::F64(vb));
        let result_ba = eval_binary_op(BinaryOp::Mul, SlotValue::F64(vb), SlotValue::F64(va));
        assert_eq!(result_ab, result_ba);
    }
}

// Property: F64 addition identity.
proptest! {
    #[test]
    fn f64_addition_identity(a in -1000000f64..=1000000f64) {
        let va = FiniteF64::new(a).unwrap_or(FiniteF64::new(0.0).unwrap());
        let zero = FiniteF64::new(0.0).unwrap();
        let result = eval_binary_op(BinaryOp::Add, SlotValue::F64(va), SlotValue::F64(zero));
        assert_eq!(result, Ok(SlotValue::F64(va)));
    }
}

// Property: F64 multiplication identity.
proptest! {
    #[test]
    fn f64_multiplication_identity(a in -1000000f64..=1000000f64) {
        let va = FiniteF64::new(a).unwrap_or(FiniteF64::new(0.0).unwrap());
        let one = FiniteF64::new(1.0).unwrap();
        let result = eval_binary_op(BinaryOp::Mul, SlotValue::F64(va), SlotValue::F64(one));
        assert_eq!(result, Ok(SlotValue::F64(va)));
    }
}

// Property: F64 double negation is identity.
proptest! {
    #[test]
    fn f64_double_negation_is_identity(a in -1000000f64..=1000000f64) {
        let va = FiniteF64::new(a).unwrap_or(FiniteF64::new(0.0).unwrap());
        let neg = eval_unary_op(UnaryOp::Neg, SlotValue::F64(va)).unwrap_or(SlotValue::Null);
        let result = eval_unary_op(UnaryOp::Neg, neg);
        assert_eq!(result, Ok(SlotValue::F64(va)));
    }
}

// Property: Boolean AND is commutative.
proptest! {
    #[test]
    fn bool_and_commutative(a in any::<bool>(), b in any::<bool>()) {
        let result_ab = eval_binary_op(BinaryOp::And, SlotValue::Bool(a), SlotValue::Bool(b));
        let result_ba = eval_binary_op(BinaryOp::And, SlotValue::Bool(b), SlotValue::Bool(a));
        assert_eq!(result_ab, result_ba);
    }
}

// Property: Boolean OR is commutative.
proptest! {
    #[test]
    fn bool_or_commutative(a in any::<bool>(), b in any::<bool>()) {
        let result_ab = eval_binary_op(BinaryOp::Or, SlotValue::Bool(a), SlotValue::Bool(b));
        let result_ba = eval_binary_op(BinaryOp::Or, SlotValue::Bool(b), SlotValue::Bool(a));
        assert_eq!(result_ab, result_ba);
    }
}

// Property: Boolean NOT double is identity.
proptest! {
    #[test]
    fn bool_not_double_is_identity(a in any::<bool>()) {
        let neg = eval_unary_op(UnaryOp::Not, SlotValue::Bool(a)).unwrap_or(SlotValue::Null);
        let result = eval_unary_op(UnaryOp::Not, neg);
        assert_eq!(result, Ok(SlotValue::Bool(a)));
    }
}

// Property: Boolean AND identity is true.
proptest! {
    #[test]
    fn bool_and_identity_true(a in any::<bool>()) {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(a), SlotValue::Bool(true));
        assert_eq!(result, Ok(SlotValue::Bool(a)));
    }
}

// Property: Boolean OR identity is false.
proptest! {
    #[test]
    fn bool_or_identity_false(a in any::<bool>()) {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(a), SlotValue::Bool(false));
        assert_eq!(result, Ok(SlotValue::Bool(a)));
    }
}

// Property: Boolean AND absorbing is false.
proptest! {
    #[test]
    fn bool_and_absorbing_false(a in any::<bool>()) {
        let result = eval_binary_op(BinaryOp::And, SlotValue::Bool(a), SlotValue::Bool(false));
        assert_eq!(result, Ok(SlotValue::Bool(false)));
    }
}

// Property: Boolean OR absorbing is true.
proptest! {
    #[test]
    fn bool_or_absorbing_true(a in any::<bool>()) {
        let result = eval_binary_op(BinaryOp::Or, SlotValue::Bool(a), SlotValue::Bool(true));
        assert_eq!(result, Ok(SlotValue::Bool(true)));
    }
}

// Property: Equality is reflexive.
proptest! {
    #[test]
    fn eq_is_reflexive(a in -1000000i64..=1000000i64) {
        let result = eval_binary_op(BinaryOp::Eq, SlotValue::I64(a), SlotValue::I64(a));
        assert_eq!(result, Ok(SlotValue::Bool(true)));
    }
}

// Property: Equality is symmetric.
proptest! {
    #[test]
    fn eq_is_symmetric(a in -1000000i64..=1000000i64, b in -1000000i64..=1000000i64) {
        let result_ab = eval_binary_op(BinaryOp::Eq, SlotValue::I64(a), SlotValue::I64(b));
        let result_ba = eval_binary_op(BinaryOp::Eq, SlotValue::I64(b), SlotValue::I64(a));
        assert_eq!(result_ab, result_ba);
    }
}

// Property: Less-than is antisymmetric.
proptest! {
    #[test]
    fn lt_is_antisymmetric(b in -1000000i64..=1000000i64, a_gap in 0i64..=1000000i64) {
        // Ordered by construction: a < b always, zero rejects at any case
        // count. Values stay within ±3e6, far from i64 overflow.
        let a = b - a_gap - 1;
        let result_ab = eval_binary_op(BinaryOp::Lt, SlotValue::I64(a), SlotValue::I64(b));
        let result_ba = eval_binary_op(BinaryOp::Lt, SlotValue::I64(b), SlotValue::I64(a));
        prop_assert_eq!(result_ab, Ok(SlotValue::Bool(true)));
        prop_assert_eq!(result_ba, Ok(SlotValue::Bool(false)));
    }
}

// Property: Less-than is transitive.
proptest! {
    #[test]
    fn lt_is_transitive(
        a in -1000000i64..=1000000i64,
        b_gap in 0i64..=1000000i64,
        c_gap in 0i64..=1000000i64,
    ) {
        // Ordered by construction, so every case executes the assertions
        // below with zero rejects. All sums stay within ±4e6, far from
        // i64 overflow.
        let b = a + b_gap + 1;
        let c = b + c_gap + 1;
        prop_assert_eq!(
            eval_binary_op(BinaryOp::Lt, SlotValue::I64(a), SlotValue::I64(b)),
            Ok(SlotValue::Bool(true))
        );
        prop_assert_eq!(
            eval_binary_op(BinaryOp::Lt, SlotValue::I64(b), SlotValue::I64(c)),
            Ok(SlotValue::Bool(true))
        );
        prop_assert_eq!(
            eval_binary_op(BinaryOp::Lt, SlotValue::I64(a), SlotValue::I64(c)),
            Ok(SlotValue::Bool(true))
        );
    }
}
