//! Kani harness: accessor reference lowering verification.
//!
//! Bead: vb-core-lower-values-actions-refs
//! Workspace: /tmp/vb-ws/vb-core-lower-values-actions-refs
//! Obligation: KANI-ACCESSOR-REF-001
//!
//! Target: crates/vb_compile/src/expression_bytecode.rs accessor lowering
//! Claim: lower_accessor_reference returns ExprOp::LoadAccessor with correct
//!        AccessorProgram for numeric paths. No PathSegment::Field in v1.
//!
//! Verifier: cargo kani --package vb_compile --harness lower_accessor_reference_numeric

#![forbid(unsafe_code)]

use crate::compile_expr_to_bytecode_with_accessors;
use crate::expression::parse_expression;
use vb_core::{AccessorIdx, AccessorProgram, PathSegment, SlotIdx};

/// KANI-ACCESSOR-REF-001: accessor reference lowering is correct for numeric paths.
///
/// Strategy:
///   1. Parse "$slots.N.P.Q" expressions via public API
///   2. Verify they lower to LoadAccessor with correct AccessorProgram
///   3. Verify all path segments are PathSegment::Index (no Field in v1)
///   4. Verify sequential accessor indices
///   5. Verify non-numeric paths are rejected
#[kani::proof]
#[kani::unwind(64)]
fn lower_accessor_reference_numeric() {
    // ----------------------------------------------------------------
    // Test 1: single-level accessor $slots.N.M
    // ----------------------------------------------------------------
    let source = "$slots.2.7";
    let parsed = parse_expression(source);
    kani::assert(parsed.is_ok(, "assertion failed"), "$slots.2.7 should parse");

    if let Ok(expr) = parsed {
        let mut constants = Vec::new();
        let mut accessors: Vec<AccessorProgram> = Vec::new();
        let result = compile_expr_to_bytecode_with_accessors(
            &expr,
            &mut constants,
            &mut accessors,
        );

        kani::assert(result.is_ok(, "assertion failed"), "$slots.2.7 should compile successfully");
        if let Ok(program) = result {
            kani::assert(!accessors.is_empty(, "assertion failed"),
                "$slots.2.7 should create one accessor entry",
            );
            let ap = &accessors[0];
            kani::assert(ap.root == SlotIdx::new(2, "assertion failed"),
                "accessor root should be SlotIdx::new(2)",
            );
            kani::assert(ap.path.len(, "assertion failed") == 1,
                "$slots.2.7 should have one path segment",
            );
            let seg0 = &ap.path[0];
            let is_index = matches!(seg0, PathSegment::Index(_));
             == 1,
                "$slots.2.7 should have one path segment",
            );
            let seg0 = &ap.path[0];
            let is_index = matches!(seg0, PathSegment::Index(_));
            kani::assert(is_index, "v1 path segment must be Index, not Field");
            if let PathSegment::Index(idx) = seg0 {
                kani::assert(*idx == 7, "path index should be 7");
            }
            kani::assert(program.ops.len() == 1, "$slots.2.7 should produce exactly 1 op");
            match program.ops[0] {
                vb_core::ExprOp::LoadAccessor(aidx) => {
                    kani::assert(aidx == AccessorIdx::new(0, "assertion failed"),
                        "accessor index should be 0 (first entry)",
                    );
                }
                _ => {
                    ,
                        "accessor index should be 0 (first entry)",
                    );
                }
                _ => {
                    kani::assert(false, "expected LoadAccessor op");
                }
            }
        }
    }

    // ----------------------------------------------------------------
    // Test 2: multi-level accessor $slots.N.P.Q.R
    // ----------------------------------------------------------------
    let source2 = "$slots.1.2.3.4";
    let parsed2 = parse_expression(source2);
    kani::assert(parsed2.is_ok(), "$slots.1.2.3.4 should parse");

    if let Ok(expr2) = parsed2 {
        let mut consts2 = Vec::new();
        let mut acc2 = Vec::new();
        let res2 = compile_expr_to_bytecode_with_accessors(
            &expr2,
            &mut consts2,
            &mut acc2,
        );

        kani::assert(res2.is_ok(, "assertion failed"), "$slots.1.2.3.4 should compile successfully");
        if let Ok(program2) = res2 {
            kani::assert(acc2.len(, "assertion failed") == 1,
                "$slots.1.2.3.4 should create exactly one accessor",
            );
            let ap2 = &acc2[0];
            kani::assert(ap2.root == SlotIdx::new(1, "assertion failed"),
                "accessor root should be SlotIdx::new(1)",
            );
            kani::assert(ap2.path.len(, "assertion failed") == 3,
                "$slots.1.2.3.4 should have 3 path segments",
            );
            // All segments must be Index — unrolled for clarity
            kani::assert(matches!(&ap2.path[0], PathSegment::Index(_), "assertion failed"),
                "all v1 path segments must be Index",
            );
            kani::assert(matches!(&ap2.path[1], PathSegment::Index(_), "assertion failed"),
                "all v1 path segments must be Index",
            );
            kani::assert(matches!(&ap2.path[2], PathSegment::Index(_), "assertion failed"),
                "all v1 path segments must be Index",
            );
        }
    }

    // ----------------------------------------------------------------
    // Test 3: field accessor $slot.1.name must be rejected
    // ----------------------------------------------------------------
    let source3 = "$slot.1.name";
    let parsed3 = parse_expression(source3);
    kani::assert(parsed3.is_ok(, "assertion failed"), "$slot.1.name should parse");

    if let Ok(expr3) = parsed3 {
        let mut consts3 = Vec::new();
        let mut acc3 = Vec::new();
        let res3 = compile_expr_to_bytecode_with_accessors(
            &expr3,
            &mut consts3,
            &mut acc3,
        );

        // Must fail: field accessor requires symbol table (not available in v1)
        kani::assert(res3.is_err(, "assertion failed"),
            "$slot.1.name must be rejected (field accessor not in v1)",
        );
        kani::assert(acc3.is_empty(, "assertion failed"),
            "rejected field accessor must not mutate accessors",
        );
    }
}

/// KANI-ACCESSOR-REF-001b: sequential accessor index assignment.
#[kani::proof]
#[kani::unwind(8)]
fn accessor_index_assignment() {
    // Multiple accessors get sequential indices: 0, 1, 2
    let expr1 = match parse_expression("$slots.1.0") {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    let expr2 = match parse_expression("$slots.2.1") {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    let expr3 = match parse_expression("$slots.3.2") {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };

    let mut consts = Vec::new();
    let mut acc = Vec::new();

    // First — index 0
    let res1 = compile_expr_to_bytecode_with_accessors(&expr1, &mut consts, &mut acc);
    kani::assert(res1.is_ok(, "assertion failed"), "first accessor should compile");
    if let Ok(prog1) = res1 {
        if let vb_core::ExprOp::LoadAccessor(idx1) = prog1.ops[0] {
            kani::assert(idx1 == AccessorIdx::new(0, "assertion failed"), "first accessor index should be 0");
        }
    }

    // Second — index 1
    let res2 = compile_expr_to_bytecode_with_accessors(&expr2, &mut consts, &mut acc);
    kani::assert(res2.is_ok(, "assertion failed"), "second accessor should compile");
    if let Ok(prog2) = res2 {
        if let vb_core::ExprOp::LoadAccessor(idx2) = prog2.ops[0] {
            kani::assert(idx2 == AccessorIdx::new(1, "assertion failed"), "second accessor index should be 1");
        }
    }

    // Third — index 2
    let res3 = compile_expr_to_bytecode_with_accessors(&expr3, &mut consts, &mut acc);
    kani::assert(res3.is_ok(, "assertion failed"), "third accessor should compile");
    if let Ok(prog3) = res3 {
        if let vb_core::ExprOp::LoadAccessor(idx3) = prog3.ops[0] {
            kani::assert(idx3 == AccessorIdx::new(2, "assertion failed"), "third accessor index should be 2");
        }
    }

    kani::assert(acc.len(, "assertion failed") == 3, "three accessors should be accumulated");
}

/// KANI-ACCESSOR-REF-001c: non-numeric path segments are rejected.
#[kani::proof]
#[kani::unwind(6)]
fn rejects_non_numeric_accessor_path() {
    // $slots.1.abc — non-numeric second segment
    let expr = match parse_expression("$slots.1.abc") {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    let mut acc: Vec<AccessorProgram> = Vec::new();
    let res = compile_expr_to_bytecode_with_accessors(
        &expr,
        &mut Vec::new(),
        &mut acc,
    );
    kani::assert(res.is_err(, "assertion failed"), "non-numeric path segment should be rejected");
    kani::assert(acc.is_empty(, "assertion failed"), "rejected path should not create accessor");
}
