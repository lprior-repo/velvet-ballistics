//! Kani harness: slot reference lowering verification.
//!
//! Bead: vb-core-lower-values-actions-refs
//! Workspace: /tmp/vb-ws/vb-core-lower-values-actions-refs
//! Obligation: KANI-SLOT-REF-001
//!
//! Target: crates/vb_compile/src/expression_bytecode.rs slot/accessor lowering
//! Claim: $slot.N lowers to LoadSlot (no accessor entry).
//!        $slots.N.P lowers to LoadAccessor with correct AccessorProgram.
//!        Non-numeric paths are rejected.
//!
//! Verifier: cargo kani --package vb_compile --harness lower_slot_reference_valid
//!
//! F-001 Fix: uses public compile_expr_to_bytecode_with_accessors API.
//! F-004 Fix: second harness has #[kani::proof] attribute.
//! F-005 Fix: all while loops removed; edge cases inlined with concrete values.

#![forbid(unsafe_code)]

use crate::compile_expr_to_bytecode_with_accessors;
use crate::expression::parse_expression;
use vb_core::{AccessorIdx, AccessorProgram, ExprOp, PathSegment, SlotIdx};

/// KANI-SLOT-REF-001: slot reference lowering is correct for all valid u16 indices.
///
/// Strategy:
///   - Use public API: compile_expr_to_bytecode_with_accessors on ParsedExpression
///   - Symbolic slot index via kani::any<u16> for broad coverage
///   - Concrete edge cases (0, 1, 255, 65535) explicitly verified
#[kani::proof]
#[kani::unwind(8)]
fn lower_slot_reference_valid() {
    // ----------------------------------------------------------------
    // Test 1: arbitrary valid u16 slot index — direct slot reference
    // $slot.N -> LoadSlot, no accessor entry
    // ----------------------------------------------------------------
    let slot_idx = kani::any::<u16>();
    let reference = format!("$slot.{slot_idx}");
    let parsed = parse_expression(&reference);

    kani::assert(parsed.is_ok(), "valid $slot.N reference should parse");
    if let Ok(expr) = parsed {
        let mut constants = Vec::new();
        let mut accessors: Vec<AccessorProgram> = Vec::new();
        let result = compile_expr_to_bytecode_with_accessors(
            &expr,
            &mut constants,
            &mut accessors,
        );

        kani::assert(result.is_ok(), "valid $slot.N should compile successfully");
        if let Ok(program) = result {
            kani::assert(program.ops.len() == 1, "$slot.N should produce exactly 1 op");
            match program.ops[0] {
                ExprOp::LoadSlot(s) => {
                    kani::assert(s == SlotIdx::new(slot_idx),
                        "LoadSlot should contain the parsed slot index",
                    );
                }
                other => {
                    ,
                        "LoadSlot should contain the parsed slot index",
                    );
                }
                other => {
                    kani::assert(false, "expected LoadSlot, got other variant");
                }
            }
            kani::assert(
                accessors.is_empty(),
                "direct $slot.N reference must not create accessor entry",
            );
        }
    }

    // ----------------------------------------------------------------
    // Test 2: concrete edge cases — $slot.0
    // ----------------------------------------------------------------
    let expr0 = parse_expression("$slot.0");
    kani::assert(expr0.is_ok(), "$slot.0 should parse");
    if let Ok(e) = expr0 {
        let mut acc: Vec<AccessorProgram> = Vec::new();
        let res = compile_expr_to_bytecode_with_accessors(
            &e, &mut Vec::new(), &mut acc,
        );
        kani::assert(res.is_ok(), "$slot.0 should succeed");
        if let Ok(prog) = res {
            if let ExprOp::LoadSlot(s) = prog.ops[0] {
                kani::assert(s == SlotIdx::new(0), "$slot.0 index should be 0");
            } else {
                , "$slot.0 index should be 0");
            } else {
                kani::assert(false, "$slot.0 should produce LoadSlot");
            }
        }
        kani::assert(acc.is_empty(), "$slot.0 should not create accessor");
    }

    // ----------------------------------------------------------------
    // Test 3: concrete edge cases — $slot.1
    // ----------------------------------------------------------------
    let expr1 = parse_expression("$slot.1");
    kani::assert(expr1.is_ok(), "$slot.1 should parse");
    if let Ok(e) = expr1 {
        let mut acc: Vec<AccessorProgram> = Vec::new();
        let res = compile_expr_to_bytecode_with_accessors(
            &e, &mut Vec::new(), &mut acc,
        );
        kani::assert(res.is_ok(), "$slot.1 should succeed");
        if let Ok(prog) = res {
            if let ExprOp::LoadSlot(s) = prog.ops[0] {
                kani::assert(s == SlotIdx::new(1), "$slot.1 index should be 1");
            } else {
                , "$slot.1 index should be 1");
            } else {
                kani::assert(false, "$slot.1 should produce LoadSlot");
            }
        }
    }

    // ----------------------------------------------------------------
    // Test 4: concrete edge cases — $slot.255
    // ----------------------------------------------------------------
    let expr255 = parse_expression("$slot.255");
    kani::assert(expr255.is_ok(), "$slot.255 should parse");
    if let Ok(e) = expr255 {
        let mut acc: Vec<AccessorProgram> = Vec::new();
        let res = compile_expr_to_bytecode_with_accessors(
            &e, &mut Vec::new(), &mut acc,
        );
        kani::assert(res.is_ok(), "$slot.255 should succeed");
        if let Ok(prog) = res {
            if let ExprOp::LoadSlot(s) = prog.ops[0] {
                kani::assert(s == SlotIdx::new(255), "$slot.255 index should be 255");
            } else {
                , "$slot.255 index should be 255");
            } else {
                kani::assert(false, "$slot.255 should produce LoadSlot");
            }
        }
    }

    // ----------------------------------------------------------------
    // Test 5: concrete edge cases — $slot.65535 (u16::MAX)
    // ----------------------------------------------------------------
    let expr_max = parse_expression("$slot.65535");
    kani::assert(expr_max.is_ok(), "$slot.65535 should parse");
    if let Ok(e) = expr_max {
        let mut acc: Vec<AccessorProgram> = Vec::new();
        let res = compile_expr_to_bytecode_with_accessors(
            &e, &mut Vec::new(), &mut acc,
        );
        kani::assert(res.is_ok(), "$slot.65535 should succeed");
        if let Ok(prog) = res {
            if let ExprOp::LoadSlot(s) = prog.ops[0] {
                kani::assert(s == SlotIdx::new(65535), "$slot.65535 index should be 65535");
            } else {
                , "$slot.65535 index should be 65535");
            } else {
                kani::assert(false, "$slot.65535 should produce LoadSlot");
            }
        }
    }

    // ----------------------------------------------------------------
    // Test 6: $slots.N (plural, no path) — also LoadSlot
    // ----------------------------------------------------------------
    let slot_idx_plural = kani::any::<u16>();
    let ref_plural = format!("$slots.{slot_idx_plural}");
    let parsed_plural = parse_expression(&ref_plural);
    kani::assert(parsed_plural.is_ok(), "valid $slots.N should parse");
    if let Ok(expr) = parsed_plural {
        let mut acc: Vec<AccessorProgram> = Vec::new();
        let res = compile_expr_to_bytecode_with_accessors(
            &expr, &mut Vec::new(), &mut acc,
        );
        kani::assert(res.is_ok(), "$slots.N should compile successfully");
        kani::assert(acc.is_empty(), "$slots.N (no path) should not create accessor");
    }
}

/// KANI-SLOT-REF-001b: slot path creates accessor entry.
///
/// $slots.N.P -> LoadAccessor with correct AccessorProgram.
/// No while loops — concrete test cases only.
/// F-004 Fix: #[kani::proof] attribute present on this harness.
#[kani::proof]
#[kani::unwind(8)]
fn lower_slot_reference_with_path_creates_accessor() {
    // ----------------------------------------------------------------
    // Test 1: single-level accessor $slots.N.M
    // ----------------------------------------------------------------
    let source = "$slots.2.7";
    let parsed = parse_expression(source);
    kani::assert(parsed.is_ok(), "$slots.2.7 should parse");

    if let Ok(expr) = parsed {
        let mut constants = Vec::new();
        let mut accessors: Vec<AccessorProgram> = Vec::new();
        let result = compile_expr_to_bytecode_with_accessors(
            &expr, &mut constants, &mut accessors,
        );

        kani::assert(result.is_ok(), "$slots.2.7 should compile successfully");
        if let Ok(program) = result {
            kani::assert(!accessors.is_empty(), "$slots.2.7 should create one accessor entry");
            let ap = &accessors[0];
            kani::assert(ap.root == SlotIdx::new(2), "accessor root should be SlotIdx::new(2)");
            kani::assert(ap.path.len() == 1, "$slots.2.7 should have one path segment");
            let seg0 = &ap.path[0];
            let is_index = matches!(seg0, PathSegment::Index(_));
             == 1, "$slots.2.7 should have one path segment");
            let seg0 = &ap.path[0];
            let is_index = matches!(seg0, PathSegment::Index(_));
            kani::assert(is_index, "v1 path segment must be Index, not Field");
            if let PathSegment::Index(idx) = seg0 {
                kani::assert(*idx == 7, "path index should be 7");
            }
            kani::assert(program.ops.len() == 1, "$slots.2.7 should produce exactly 1 op");
            match program.ops[0] {
                ExprOp::LoadAccessor(aidx) => {
                    kani::assert(aidx == AccessorIdx::new(0),
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
            &expr2, &mut consts2, &mut acc2,
        );

        kani::assert(res2.is_ok(), "$slots.1.2.3.4 should compile successfully");
        if let Ok(program2) = res2 {
            kani::assert(acc2.len() == 1, "$slots.1.2.3.4 should create exactly one accessor");
            let ap2 = &acc2[0];
            kani::assert(ap2.root == SlotIdx::new(1), "accessor root should be SlotIdx::new(1)");
            kani::assert(ap2.path.len() == 3, "$slots.1.2.3.4 should have 3 path segments");
            // Unrolled segment checks — no loops
            kani::assert(matches!(&ap2.path[0], PathSegment::Index(_)),
                "segment 0 must be Index",
            );
            kani::assert(matches!(&ap2.path[1], PathSegment::Index(_)),
                "segment 1 must be Index",
            );
            kani::assert(matches!(&ap2.path[2], PathSegment::Index(_)),
                "segment 2 must be Index",
            );
        }
    }

    // ----------------------------------------------------------------
    // Test 3: field accessor $slot.1.name must be rejected
    // ----------------------------------------------------------------
    let source3 = "$slot.1.name";
    let parsed3 = parse_expression(source3);
    kani::assert(parsed3.is_ok(), "$slot.1.name should parse");

    if let Ok(expr3) = parsed3 {
        let mut consts3 = Vec::new();
        let mut acc3 = Vec::new();
        let res3 = compile_expr_to_bytecode_with_accessors(
            &expr3, &mut consts3, &mut acc3,
        );

        // Must fail: field accessor requires symbol table (not available in v1)
        kani::assert(res3.is_err(), "$slot.1.name must be rejected (field accessor not in v1)");
        kani::assert(acc3.is_empty(), "rejected field accessor must not mutate accessors");
    }
}
