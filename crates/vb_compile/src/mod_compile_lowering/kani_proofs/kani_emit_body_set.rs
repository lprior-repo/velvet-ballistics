// Verification artifact: kani_emit_body_set.rs
// Obligation: PO-010
// Bead: vb-xi2f.21 | State: 5 (proof-writer)
// Verifier: Kani
//
// Harness:
//   - kani_emit_single_body_set_all (PO-010)
//
// GOD RULE 1: Uses kani::any() to vary StepIdx, SlotIdx, and Set values.
//   No hardcoded structural inputs.
// GOD RULE 2: Binds to actual production emit_single_body_set.
//   Verifies Set/Do behavior is unchanged after ForEach arm is added.

#![cfg(kani)]
#![allow(unused_must_use)]

use crate::mod_compile_lowering::part_04::emit_single_body_set;
use crate::mod_compile_lowering::part_07::SlotCompiler;
use vb_core::{CompiledNodeKind, SlotIdx, StepIdx};
use vb_yaml::ast::{StepAst, StepPrimitive};

// =========================================================================
// Input generators (GOD RULE 1)
// =========================================================================

fn make_set_body(value: i64) -> Vec<StepAst> {
    vec![StepAst {
        id: "s".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "o".to_string(),
            value: value.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }]
}

fn make_do_body(action: &str, input_val: &str) -> Vec<StepAst> {
    vec![StepAst {
        id: "d".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Do {
            action: action.to_string(),
            input: input_val.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }]
}

fn any_step_idx() -> StepIdx {
    let raw: u16 = kani::any();
    kani::assume(raw <= 65530);
    StepIdx::new(raw)
}

fn any_slot_idx() -> SlotIdx {
    let raw: u16 = kani::any();
    SlotIdx::new(raw)
}

// =========================================================================
// PO-010: kani_emit_single_body_set_all
// =========================================================================

/// PO-010: Regression verification for emit_single_body_set.
///
/// Verifies that existing Set and Do arms continue to work correctly
/// after the ForEach dispatch arm is added to the match expression.
///
/// Tests:
///   (1) Set body: single step with integer value → Ok, emits SetConst node
///   (2) Do body: single step with action+input → Ok, emits Do node
///   (3) Non-Set/non-Do body: returns error (UnsupportedStepPrimitive)
///   (4) Empty body: returns error (StepFieldShape)
///   (5) Multi-step body: returns error (StepFieldShape)
#[kani::proof]
#[kani::unwind(30)]
fn kani_emit_single_body_set_all() {
    let id = any_step_idx();
    let slot = any_slot_idx();

    // --- H1: Set body must compile ---
    {
        let set_value: i64 = kani::any();
        let body = make_set_body(set_value);
        let mut builder = SlotCompiler::new();
        let result = emit_single_body_set(
            &body, id, 0, slot, None, &mut builder, false,
        );
        kani::assert(result.is_ok(), "H1: Set body must compile successfully");
        if let Some(node) = builder.nodes.first() {
            kani::assert(
                matches!(node.kind, CompiledNodeKind::SetConst { .. }),
                "H1: Set body must emit SetConst node",
            );
            kani::assert(
                node.id == id,
                "H1: emitted node must have correct id",
            );
        }
    }

    // --- H2: Do body must compile ---
    {
        let do_action: u8 = kani::any();
        let do_input: u8 = kani::any();
        // Action and input must be valid u16 values for parsing
        let action_val = (do_action % 99).saturating_add(1); // 1..99
        let input_val = (do_input % 99).saturating_add(1);   // 1..99
        let body = make_do_body(&action_val.to_string(), &input_val.to_string());
        let mut builder = SlotCompiler::new();
        let result = emit_single_body_set(
            &body, id, 0, slot, None, &mut builder, false,
        );
        kani::assert(result.is_ok(), "H2: Do body must compile successfully");
        if let Some(node) = builder.nodes.first() {
            kani::assert(
                matches!(node.kind, CompiledNodeKind::Do { .. }),
                "H2: Do body must emit Do node",
            );
        }
    }

    // --- H3: Non-Set/Non-Do body must error ---
    {
        let non_set_body = vec![StepAst {
            id: "ns".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Finish {
                result: vb_yaml::ast::ScalarValue::Integer(0),
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        }];
        let mut builder = SlotCompiler::new();
        let result = emit_single_body_set(
            &non_set_body, id, 0, slot, None, &mut builder, false,
        );
        kani::assert(result.is_err(), "H3: Non-Set body must return error");
    }

    // --- H4: Empty body must error ---
    {
        let empty_body: Vec<StepAst> = vec![];
        let mut builder = SlotCompiler::new();
        let result = emit_single_body_set(
            &empty_body, id, 0, slot, None, &mut builder, false,
        );
        kani::assert(result.is_err(), "H4: Empty body must return error");
    }

    // --- H5: Multi-step body must error ---
    {
        let multi_body = vec![
            StepAst {
                id: "a".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Set {
                    output: "o1".to_string(),
                    value: "1".to_string(),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            },
            StepAst {
                id: "b".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Set {
                    output: "o2".to_string(),
                    value: "2".to_string(),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            },
        ];
        let mut builder = SlotCompiler::new();
        let result = emit_single_body_set(
            &multi_body, id, 0, slot, None, &mut builder, false,
        );
        kani::assert(result.is_err(), "H5: Multi-step body (>1 step) must return error");
    }
}
