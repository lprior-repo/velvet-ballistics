use super::*;

// ─────────────────────────────────────────────────────────────────
// Test 9: lower_canonical_choose emits no YAML strings in IR
// Plan N12 / RRO-B013
// ─────────────────────────────────────────────────────────────────
/// Verifies that `lower_canonical_choose` stores branch conditions as
/// `SlotIdx` values, never as raw YAML strings. The type system enforces
/// this (`SlotBranch.condition: SlotIdx`), but this test provides
/// behavioral evidence.
#[test]
fn lower_canonical_choose_emits_no_yaml_strings() {
    // Given: a choose with when="5" and one Set body step
    let branches = vec![crate::ChooseBranch {
        when: "5".to_string(),
        steps: vec![choose_body_set_step("body_a", "42")],
    }];
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    // When: lower_canonical_choose compiles
    lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(2)),
        &step_names,
        &mut builder,
    )
    .expect("choose must lower");
    // Then: all conditions are SlotIdx (type-guaranteed; verified by value)
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot {
            branches: slot_branches,
            ..
        } => {
            assert!(!slot_branches.is_empty(), "must have at least one branch");
            assert_eq!(
                slot_branches[0].condition,
                vb_core::ids::SlotIdx::new(5),
                "condition must be SlotIdx(5), parsed from when string '5'"
            );
        }
        other => panic!("expected ChooseSlot, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────
// Test 10: slot compiler records unique indices
// Plan N15 / RRO-B006
// ─────────────────────────────────────────────────────────────────
/// Verifies that the `SlotCompiler` tracks slot indices as body steps
/// are lowered. The body Set node must have an assigned output slot,
/// and the condition slot referenced in the ChooseSlot branches must
/// match the parsed `when` value.
#[test]
fn slot_compiler_records_unique_indices() {
    // Given: a choose with one Set body step
    let branches = vec![crate::ChooseBranch {
        when: "1".to_string(),
        steps: vec![choose_body_set_step("body_a", "7")],
    }];
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    // When: lower_canonical_choose compiles the body
    lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(2)),
        &step_names,
        &mut builder,
    )
    .expect("choose must lower");
    // Then: the body Set node has an output slot (observable behavior)
    assert!(
        builder.nodes[1].output.slot().is_some(),
        "body Set node must have an output slot assigned"
    );
    // Verify the ChooseSlot condition is correctly parsed from when="1"
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot {
            branches: slot_branches,
            ..
        } => {
            assert_eq!(
                slot_branches[0].condition,
                vb_core::ids::SlotIdx::new(1),
                "condition must be SlotIdx(1) from when string '1'"
            );
        }
        other => panic!("expected ChooseSlot, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────
// Test 11: condition slots disjoint from body output slots
// ─────────────────────────────────────────────────────────────────
/// Verifies that condition slots referenced in ChooseSlot nodes are
/// distinct from the output slots allocated during body step lowering.
/// The output slot is extracted from the compiled body node (not hardcoded),
/// so the test does not couple to the internal slot-allocation scheme.
#[test]
fn lower_canonical_choose_slots_disjoint_from_conditions() {
    // Given: a choose with when="99" and one Set body step
    let branches = vec![crate::ChooseBranch {
        when: "99".to_string(),
        steps: vec![choose_body_set_step("body_a", "42")],
    }];
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    // When: lower_canonical_choose compiles
    lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(2)),
        &step_names,
        &mut builder,
    )
    .expect("choose must lower");
    // Then: condition slots are disjoint from body output slots
    let condition_slot = vb_core::ids::SlotIdx::new(99);
    // Extract the body node's actual output slot from the compiled IR
    let body_output_slot = builder.nodes[1]
        .output
        .slot()
        .expect("body Set node must have an output slot");
    // Verify that the condition slot does NOT match the body output slot
    assert_ne!(
        condition_slot, body_output_slot,
        "condition slot ({condition_slot:?}) must be disjoint from body output slot ({body_output_slot:?})"
    );
    // Verify the ChooseSlot node has the correct condition
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot {
            branches: slot_branches,
            ..
        } => {
            assert_eq!(
                slot_branches[0].condition, condition_slot,
                "condition must be SlotIdx(99)"
            );
        }
        other => panic!("expected ChooseSlot, got {other:?}"),
    }
    // Verify the body Set node has a different output
    assert_eq!(builder.nodes[1].id, vb_core::ids::StepIdx::new(1));
    assert!(
        matches!(
            &builder.nodes[1].kind,
            vb_core::CompiledNodeKind::SetConst { .. }
        ),
        "body node must be SetConst"
    );
}

// ─────────────────────────────────────────────────────────────────
// Test 12: lower_canonical_choose otherwise target past body span
// Plan N13 / RRO-B003
// ─────────────────────────────────────────────────────────────────
/// Verifies that the `otherwise` target StepIdx points past the body
/// span (i.e., it is not one of the body step nodes).
#[test]
fn lower_canonical_choose_otherwise_target_past_body() {
    // Given: a choose with 2 body steps and an otherwise label pointing past
    // the body. step_names has 4 entries so "done" maps to StepIdx(3),
    // which is beyond the body span (StepIdx 0 ChooseSlot + body at 1,2).
    let branches = vec![crate::ChooseBranch {
        when: "0".to_string(),
        steps: vec![
            choose_body_set_step("body_a", "7"),
            choose_body_set_step("body_b", "8"),
        ],
    }];
    let step_names: [Box<str>; 4] = [
        Box::from("pick"),
        Box::from("body_a"),
        Box::from("body_b"),
        Box::from("done"),
    ];
    let mut builder = crate::SlotCompiler::new();
    // When: lower_canonical_choose compiles with otherwise="done"
    lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(3)),
        &step_names,
        &mut builder,
    )
    .expect("choose with otherwise must lower");
    // Then: otherwise target is not a body node AND exactly hits the done label
    // Body spans StepIdx(1) through StepIdx(2).
    // "done" at index 3 in step_names → StepIdx(3) → past body span.
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot { otherwise, .. } => {
            let target = otherwise
                .step()
                .expect("otherwise must be set when label is provided");
            assert_eq!(
                target,
                vb_core::ids::StepIdx::new(3),
                "otherwise must target node at StepIdx(3) past body span, got {target:?}"
            );
            assert_ne!(
                target,
                vb_core::ids::StepIdx::new(1),
                "otherwise must not target first body node"
            );
            assert_ne!(
                target,
                vb_core::ids::StepIdx::new(2),
                "otherwise must not target last body node"
            );
        }
        other => panic!("expected ChooseSlot, got {other:?}"),
    }
}
