//! Tests for nested `do` primitive body lowering.
//!
//! These tests verify that `do` primitives inside scoped primitive bodies
//! (repeat, collect, for_each, reduce) are correctly lowered to final IR.

use vb_compile::{CompileError, CompileErrors, compile_workflow};
use vb_core::{CompiledNodeKind, CompiledWorkflow, StepIdx};

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: nested_do_lowering\nwhen:\n  manual: {}\nsteps:\n";

/// Tests that a `repeat` primitive with a `do` body lowers to final IR.
#[test]
fn nested_do_in_repeat_body_lowers_to_final_ir() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: action_step\n          do:\n            action: \"0\"\n            input: \"0\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.entry, vb_core::StepIdx::new(0));
    assert_eq!(parts.slot_count, 3);
    assert_eq!(parts.nodes.len(), 5);
    assert_node(
        &parts.nodes,
        0,
        vb_core::CompiledNodeKind::RepeatStart {
            max_attempts: 3,
            body: vb_core::StepIdx::new(1),
            done: vb_core::StepIdx::new(3),
        },
        Some(vb_core::SlotIdx::new(1)),
        None,
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        1,
        vb_core::CompiledNodeKind::Do {
            action: vb_core::ActionId::new(0),
            input: vb_core::SlotIdx::new(0),
        },
        Some(vb_core::SlotIdx::new(2)),
        Some(vb_core::StepIdx::new(2)),
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        2,
        vb_core::CompiledNodeKind::RepeatCheck {
            attempt_slot: vb_core::SlotIdx::new(1),
            done: vb_core::StepIdx::new(3),
        },
        Some(vb_core::SlotIdx::new(1)),
        Some(vb_core::StepIdx::new(1)),
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        3,
        vb_core::CompiledNodeKind::RepeatFinish {
            result: vb_core::SlotIdx::new(2),
        },
        Some(vb_core::SlotIdx::new(2)),
        Some(vb_core::StepIdx::new(4)),
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        4,
        vb_core::CompiledNodeKind::Finish {
            result: vb_core::SlotIdx::new(0),
        },
        None,
        None,
        None,
        None,
    )?;

    Ok(())
}

/// Tests that a `collect` primitive with a `do` body lowers to final IR.
#[test]
fn nested_do_in_collect_body_lowers_to_final_ir() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: collect_pages\n    collect:\n      variable: page\n      source: \"0\"\n      pages: 3\n      items: 5\n      steps:\n        - id: process\n          do:\n            action: \"0\"\n            input: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.entry, vb_core::StepIdx::new(0));
    assert_eq!(parts.slot_count, 4);
    assert_eq!(parts.nodes.len(), 5);
    assert_node(
        &parts.nodes,
        0,
        vb_core::CompiledNodeKind::CollectStart {
            source: vb_core::SlotIdx::new(0),
            limit: 15,
            page_size: 5,
            body: vb_core::StepIdx::new(1),
            done: vb_core::StepIdx::new(3),
        },
        Some(vb_core::SlotIdx::new(2)),
        None,
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        1,
        vb_core::CompiledNodeKind::Do {
            action: vb_core::ActionId::new(0),
            input: vb_core::SlotIdx::new(1),
        },
        Some(vb_core::SlotIdx::new(3)),
        Some(vb_core::StepIdx::new(2)),
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        2,
        vb_core::CompiledNodeKind::CollectNext {
            collector_slot: vb_core::SlotIdx::new(2),
            body: vb_core::StepIdx::new(1),
            done: vb_core::StepIdx::new(3),
        },
        None,
        None,
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        3,
        vb_core::CompiledNodeKind::CollectFinish {
            collector_slot: vb_core::SlotIdx::new(2),
        },
        Some(vb_core::SlotIdx::new(3)),
        Some(vb_core::StepIdx::new(4)),
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        4,
        vb_core::CompiledNodeKind::Finish {
            result: vb_core::SlotIdx::new(0),
        },
        None,
        None,
        None,
        None,
    )?;

    Ok(())
}

/// Tests that a `for_each` primitive with a `do` body lowers to final IR.
#[test]
fn nested_do_in_for_each_body_lowers_to_final_ir() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: loop\n    for_each:\n      variable: item\n      input: \"0\"\n      at_once: 2\n      steps:\n        - id: process\n          do:\n            action: \"0\"\n            input: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.entry, vb_core::StepIdx::new(0));
    assert_eq!(parts.slot_count, 5);
    assert_eq!(parts.nodes.len(), 4);
    assert_node(
        &parts.nodes,
        0,
        vb_core::CompiledNodeKind::ForEachStart {
            input: vb_core::SlotIdx::new(0),
            item_slot: vb_core::SlotIdx::new(3),
            limit: 2,
            body: vb_core::StepIdx::new(1),
            done: vb_core::StepIdx::new(3),
        },
        Some(vb_core::SlotIdx::new(2)),
        None,
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        1,
        vb_core::CompiledNodeKind::Do {
            action: vb_core::ActionId::new(0),
            input: vb_core::SlotIdx::new(1),
        },
        Some(vb_core::SlotIdx::new(4)),
        Some(vb_core::StepIdx::new(2)),
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        2,
        vb_core::CompiledNodeKind::ForEachNext {
            iterator_slot: vb_core::SlotIdx::new(2),
            body: vb_core::StepIdx::new(1),
            done: vb_core::StepIdx::new(3),
        },
        Some(vb_core::SlotIdx::new(3)),
        None,
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        3,
        vb_core::CompiledNodeKind::Finish {
            result: vb_core::SlotIdx::new(0),
        },
        None,
        None,
        None,
        None,
    )?;

    Ok(())
}

/// Tests that a `reduce` primitive with a `do` body lowers to final IR.
#[test]
fn nested_do_in_reduce_body_lowers_to_final_ir() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: fold\n    reduce:\n      variable: acc\n      input: \"0\"\n      initial: \"10\"\n      steps:\n        - id: add\n          do:\n            action: \"0\"\n            input: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.entry, vb_core::StepIdx::new(0));
    assert_eq!(parts.slot_count, 6);
    assert_eq!(parts.nodes.len(), 6);
    assert_eq!(parts.constants.get(0), Some(&vb_core::ConstValue::I64(10)));
    assert_node(
        &parts.nodes,
        0,
        vb_core::CompiledNodeKind::Copy {
            source: vb_core::SlotIdx::new(0),
        },
        Some(vb_core::SlotIdx::new(2)),
        Some(vb_core::StepIdx::new(1)),
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        1,
        vb_core::CompiledNodeKind::ReduceStart {
            input: vb_core::SlotIdx::new(2),
            accumulator: vb_core::SlotIdx::new(5),
            initial: vb_core::ConstIdx::new(0),
            body: vb_core::StepIdx::new(2),
            done: vb_core::StepIdx::new(4),
        },
        Some(vb_core::SlotIdx::new(4)),
        None,
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        2,
        vb_core::CompiledNodeKind::Do {
            action: vb_core::ActionId::new(0),
            input: vb_core::SlotIdx::new(1),
        },
        Some(vb_core::SlotIdx::new(5)),
        Some(vb_core::StepIdx::new(3)),
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        3,
        vb_core::CompiledNodeKind::ReduceNext {
            iterator_slot: vb_core::SlotIdx::new(4),
            accumulator: vb_core::SlotIdx::new(5),
            body: vb_core::StepIdx::new(2),
            done: vb_core::StepIdx::new(4),
        },
        Some(vb_core::SlotIdx::new(3)),
        None,
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        4,
        vb_core::CompiledNodeKind::ReduceFinish {
            accumulator: vb_core::SlotIdx::new(5),
        },
        Some(vb_core::SlotIdx::new(5)),
        Some(vb_core::StepIdx::new(5)),
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        5,
        vb_core::CompiledNodeKind::Finish {
            result: vb_core::SlotIdx::new(0),
        },
        None,
        None,
        None,
        None,
    )?;

    Ok(())
}

/// Tests that nested do body with invalid input slot reference returns appropriate error.
#[test]
fn nested_do_with_invalid_input_slot_returns_error() -> Result<(), String> {
    // The input "99999" is out of range for slot index
    let yaml = workflow_yaml(
        "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: action_step\n          do:\n            action: \"0\"\n            input: \"99999\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let result = compile_workflow(yaml.as_bytes());
    assert!(
        result.is_err(),
        "nested do with out-of-range input slot should fail"
    );
    let errors = result.err().unwrap();
    let first = errors
        .first()
        .ok_or_else(|| String::from("expected at least one error"))?;
    match first {
        CompileError::SlotIndexOutOfRange { value } => {
            assert_eq!(*value, 99999, "should report exact out-of-range value");
        }
        other => {
            return Err(format!(
                "expected SlotIndexOutOfRange error for invalid input, got {other:?}"
            ));
        }
    }
    Ok(())
}

/// Tests that together branches can contain do primitives.
#[test]
fn nested_do_in_together_branch_lowers_to_final_ir() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: fanout\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_action\n              do:\n                action: \"0\"\n                input: \"0\"\n        - label: right\n          steps:\n            - id: right_action\n              do:\n                action: \"1\"\n                input: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml)?;
    let parts = workflow.to_parts();
    assert_eq!(parts.entry, vb_core::StepIdx::new(0));
    assert_eq!(parts.slot_count, 4);
    assert_eq!(parts.nodes.len(), 7);
    assert_node(
        &parts.nodes,
        0,
        vb_core::CompiledNodeKind::TogetherStart {
            branches: vec![vb_core::StepIdx::new(1), vb_core::StepIdx::new(3)],
            join: vb_core::StepIdx::new(5),
        },
        Some(vb_core::SlotIdx::new(2)),
        None,
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        1,
        vb_core::CompiledNodeKind::TogetherBranch {
            branch: 0,
            accumulator: vb_core::SlotIdx::new(2),
            entry: vb_core::StepIdx::new(2),
            join: vb_core::StepIdx::new(5),
        },
        None,
        None,
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        2,
        vb_core::CompiledNodeKind::Do {
            action: vb_core::ActionId::new(0),
            input: vb_core::SlotIdx::new(0),
        },
        Some(vb_core::SlotIdx::new(3)),
        Some(vb_core::StepIdx::new(3)),
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        3,
        vb_core::CompiledNodeKind::TogetherBranch {
            branch: 1,
            accumulator: vb_core::SlotIdx::new(2),
            entry: vb_core::StepIdx::new(4),
            join: vb_core::StepIdx::new(5),
        },
        Some(vb_core::SlotIdx::new(3)),
        None,
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        4,
        vb_core::CompiledNodeKind::Do {
            action: vb_core::ActionId::new(1),
            input: vb_core::SlotIdx::new(1),
        },
        Some(vb_core::SlotIdx::new(3)),
        Some(vb_core::StepIdx::new(5)),
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        5,
        vb_core::CompiledNodeKind::TogetherJoin {
            branch_count: 2,
            accumulator: vb_core::SlotIdx::new(2),
        },
        Some(vb_core::SlotIdx::new(3)),
        Some(vb_core::StepIdx::new(6)),
        None,
        None,
    )?;
    assert_node(
        &parts.nodes,
        6,
        vb_core::CompiledNodeKind::Finish {
            result: vb_core::SlotIdx::new(0),
        },
        None,
        None,
        None,
        None,
    )?;

    Ok(())
}

// =============================================================================
// Helper functions
// =============================================================================

fn compile_yaml(yaml: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(yaml.as_bytes()).map_err(|errors| format_compile_errors(&errors))
}

fn format_compile_errors(errors: &CompileErrors) -> String {
    let mut message = String::new();
    for error in errors.iter() {
        if !message.is_empty() {
            message.push_str("; ");
        }
        message.push_str(error.code().as_str());
        message.push_str(": ");
        message.push_str(&error.to_string());
    }
    message
}

fn workflow_yaml(steps: &str) -> String {
    let mut yaml = String::from(HEADER);
    yaml.push_str(steps);
    yaml
}

fn assert_node(
    nodes: &[vb_core::CompiledNode],
    id: u16,
    kind: vb_core::CompiledNodeKind,
    output: Option<vb_core::SlotIdx>,
    next: Option<vb_core::StepIdx>,
    error_slot: Option<vb_core::SlotIdx>,
    on_error: Option<vb_core::StepIdx>,
) -> Result<(), String> {
    let actual = nodes
        .get(usize::from(id))
        .ok_or_else(|| format!("node {} not found", id))?;
    let expected = vb_core::CompiledNode {
        id: vb_core::StepIdx::new(id),
        kind,
        output,
        next,
        error_slot,
        on_error,
    };
    assert_eq!(*actual, expected);
    Ok(())
}
