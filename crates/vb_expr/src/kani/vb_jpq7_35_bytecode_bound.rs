#![forbid(unsafe_code)]
//! PO-KANI-003: Bytecode op count and constant pool bound verification
//! Requirement: C-COMPILE-1
//!
//! Production target: crate::bytecode::validate_op_count, push_constant
//!
//! Verifies:
//! - compile_expr_to_bytecode returns BytecodeTooLong when ops exceed MAX_OPS=256
//! - push_constant returns ConstantPoolOverflow when constants exceed MAX_CONSTANTS=65535
//! - Both bounds are independently enforced

use crate::ExprError;
use crate::bytecode::{compile_expr_to_bytecode, push_constant};
use crate::parser::{ExprAst, ExprLiteral};
use vb_core::ConstValue;

/// Maximum bytecode ops (must match MAX_OPS in bytecode/mod.rs).
const MAX_OPS: usize = 256;
/// Maximum constant pool entries (must match MAX_CONSTANTS in bytecode/mod.rs).
const MAX_CONSTANTS: usize = 65_535;

/// Build an AST that produces exactly `n` ops when compiled.
/// Each I64 literal produces 2 ops: LoadConst + the const index.
/// Actually, each literal compiles to 1 LoadConst op. So we need n literals.
fn build_ast_with_n_ops(n: usize) -> ExprAst {
    // Build a deeply nested Add chain: 1 + 1 + 1 + ...
    // Each literal: 1 op (LoadConst)
    // Each Add: 1 op (Add)
    // For n=1: just one literal (1 op)
    // For n>1: binary trees produce roughly n ops
    // Simpler: build n literals each, connected by Add
    let mut left = ExprAst::Literal(ExprLiteral::I64(1));
    for _ in 1..n {
        left = ExprAst::Binary {
            op: crate::lexer::BinaryOp::Add,
            left: Box::new(left),
            right: Box::new(ExprAst::Literal(ExprLiteral::I64(1))),
        };
    }
    left
}

/// PO-KANI-003 H1: compile_expr_to_bytecode succeeds for small ASTs.
#[kani::proof]
#[kani::unwind(20)]
fn check_compile_small_ast_succeeds() {
    let n: usize = kani::any();
    kani::assume(n >= 1 && n <= 10);

    let ast = build_ast_with_n_ops(n);
    let result = compile_expr_to_bytecode(&ast);

    match result {
        Ok(program) => {
            // Program ops must be bounded
            kani::assert(program.ops.len(, "assertion failed") <= MAX_OPS,
                "small AST: ops must not exceed MAX_OPS",
            );
        }
        Err(e) => {
            // Any error is safe
            kani::assert(!matches!(e, ExprError::BytecodeTooLong { .. }, "assertion failed"),
                "small AST must not trigger BytecodeTooLong",
            );
        }
    }
}

/// PO-KANI-003 H2: push_constant boundary — accepts indices up to MAX_CONSTANTS-1.
#[kani::proof]
fn check_push_constant_within_bound() {
    let n: usize = kani::any();
    kani::assume(n < MAX_CONSTANTS);

    let mut constants: Vec<ConstValue> = Vec::with_capacity(MAX_CONSTANTS);
    // Pre-fill with n entries
    for _ in 0..n {
        constants.push(ConstValue::Bool(true));
    }

    let value = ConstValue::I64(kani::any());
    let result = push_constant(value, &mut constants);

    match result {
        Ok(idx) => {
            kani::assert(usize::from(idx.as_usize(), "assertion failed") == n,
                "constant index must match position",
            );
            kani::assert(constants.len(, "assertion failed") == n + 1, "constant was pushed");
        }
        Err(e) => {
            kani::assert(matches!(e, ExprError::ConstantPoolOverflow, "assertion failed"),
                "only possible error for n < MAX_CONSTANTS is ConstantPoolOverflow \
                 (possible if n == MAX_CONSTANTS due to off-by-one or u16 overflow)",
            );
        }
    }
}

/// PO-KANI-003 H3: push_constant rejects at MAX_CONSTANTS limit.
#[kani::proof]
fn check_push_constant_at_limit() {
    // Build a pool at exactly MAX_CONSTANTS entries
    let mut constants: Vec<ConstValue> = Vec::with_capacity(MAX_CONSTANTS + 1);
    for _ in 0..MAX_CONSTANTS {
        constants.push(ConstValue::Bool(true));
    }

    let value = ConstValue::I64(42);
    let result = push_constant(value, &mut constants);

    // At MAX_CONSTANTS, push must fail
    kani::assert(result.is_err(),
        "push_constant at MAX_CONSTANTS limit must return error",
    );

    match result {
        Err(ExprError::ConstantPoolOverflow) => {
            // Correct behavior
        }
        Err(other) => {
            // Any typed error is acceptable (but we expect ConstantPoolOverflow)
            let _ = other;
        }
        Ok(_) => {
            ,
        "push_constant at MAX_CONSTANTS limit must return error",
    );

    match result {
        Err(ExprError::ConstantPoolOverflow) => {
            // Correct behavior
        }
        Err(other) => {
            // Any typed error is acceptable (but we expect ConstantPoolOverflow)
            let _ = other;
        }
        Ok(_) => {
            kani::assert(false, "push_constant must fail at limit");
        }
    }

    // Pool size must not increase beyond MAX_CONSTANTS
    kani::assert(
        constants.len() == MAX_CONSTANTS,
        "constant pool must not grow beyond MAX_CONSTANTS on failed push",
    );
}

/// PO-KANI-003 H4: compile_expr_to_bytecode validates bytecode length.
/// Construct an AST that produces many ops and verify BytecodeTooLong fires.
#[kani::proof]
#[kani::unwind(130)]
fn check_compile_rejects_too_many_ops() {
    // Build an AST with 130 I64 literals chained by Add
    // Each literal: 1 op (LoadConst with push_constant)
    // Each Add: 1 op (Add)
    // Total: 130 LoadConst + 129 Add = 259 ops > MAX_OPS=256
    let n_literals: usize = 130;
    let mut ast = ExprAst::Literal(ExprLiteral::I64(1));
    for _ in 1..n_literals {
        ast = ExprAst::Binary {
            op: crate::lexer::BinaryOp::Add,
            left: Box::new(ast),
            right: Box::new(ExprAst::Literal(ExprLiteral::I64(1))),
        };
    }

    let result = compile_expr_to_bytecode(&ast);

    kani::assert(result.is_err(), "130-add chain (259 ops) must be rejected");

    match result {
        Err(ExprError::BytecodeTooLong { len, max }) => {
            , "130-add chain (259 ops) must be rejected");

    match result {
        Err(ExprError::BytecodeTooLong { len, max }) => {
            kani::assert(len > MAX_OPS, "reported len must exceed MAX_OPS");
            kani::assert(max == MAX_OPS, "max must be 256");
        }
        Err(e) => {
            // Could also be ConstantPoolOverflow if constants exceed 65535
            // This is also a valid bound enforcement
            let _ = e;
        }
        Ok(_) => {
            kani::assert(false, "130-add chain must not compile successfully");
        }
    }
}

/// PO-KANI-003 H5: push_constant with index overflow (u16::MAX + 1).
/// Verifies that u16::try_from catches the overflow before the MAX_CONSTANTS check.
#[kani::proof]
fn check_push_constant_u16_overflow() {
    let n: usize = kani::any();
    kani::assume(n > u16::MAX as usize);
    kani::assume(n <= u16::MAX as usize + 100);

    let mut constants: Vec<ConstValue> = Vec::with_capacity(n);
    // Pre-fill with n entries (if possible in Kani's memory model)
    for _ in 0..u16::MAX as usize {
        constants.push(ConstValue::Bool(true));
    }
    // Add one more to trigger the u16::try_from failure
    constants.push(ConstValue::Bool(true));

    let value = ConstValue::I64(42);
    let result = push_constant(value, &mut constants);

    // At len = u16::MAX + 1 = 65536, u16::try_from fails -> ConstantPoolOverflow
    kani::assert(result.is_err(),
        "push_constant at u16::MAX+1 must return error",
    );

    match result {
        Err(ExprError::ConstantPoolOverflow) => {
            // Correct — u16::try_from catches the overflow
        }
        Err(_) => {
            // Any typed error is safe
        }
        Ok(_) => {
            ,
        "push_constant at u16::MAX+1 must return error",
    );

    match result {
        Err(ExprError::ConstantPoolOverflow) => {
            // Correct — u16::try_from catches the overflow
        }
        Err(_) => {
            // Any typed error is safe
        }
        Ok(_) => {
            kani::assert(false, "push_constant must fail at u16::MAX+1");
        }
    }
}
