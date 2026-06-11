//! Tests for nested `do` primitive body lowering.
//!
//! These tests verify that `do` primitives inside scoped primitive bodies
//! (repeat, collect, for_each, reduce) are correctly lowered to final IR.

use vb_compile::{CompileError, CompileErrors, compile_workflow};
use vb_core::{CompiledNodeKind, CompiledWorkflow, StepIdx};

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: nested-do-lowering\nwhen:\n  manual: {}\nsteps:\n";

/// Tests that a `repeat` primitive with a `do` body lowers to final IR.
#[test]
fn nested_do_in_repeat_body_lowers_to_final_ir() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: action_step\n          do:\n            action: \"0\"\n            input: \"0\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml)?;
    let parts = workflow.to_parts();

    // The expected structure:
    // 0 = RepeatStart { max_attempts: 3, body: 1, done: 3 }
    // 1 = Do { action: test_action, input: 0 } (the body step)
    // 2 = RepeatAttempt { attempt_slot: 1, body: 1, done: 3 }
    // 3 = RepeatFinish { result: 1 }
    // 4 = Finish { result: 0 }

    assert_eq!(
        parts.nodes.len(),
        5,
        "repeat with do body should produce 5 nodes"
    );
    assert_eq!(parts.entry, StepIdx::new(0), "entry must be dense zero");

    // Verify RepeatStart at node 0
    match &parts.nodes[0].kind {
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => {
            assert_eq!(*max_attempts, 3, "RepeatStart max_attempts");
            assert_eq!(body.get(), 1, "RepeatStart body");
            assert_eq!(done.get(), 3, "RepeatStart done");
        }
        other => return Err(format!("expected RepeatStart at node 0, got {other:?}")),
    }

    // Verify Do at node 1
    match &parts.nodes[1].kind {
        CompiledNodeKind::Do { action, input } => {
            assert_eq!(action.get(), 0, "Do action id"); // First registered action
            assert_eq!(input.get(), 0, "Do input slot");
        }
        other => return Err(format!("expected Do at node 1, got {other:?}")),
    }

    // Verify RepeatAttempt at node 2
    match &parts.nodes[2].kind {
        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => {
            assert_eq!(attempt_slot.get(), 1, "RepeatAttempt attempt_slot");
            assert_eq!(body.get(), 1, "RepeatAttempt body");
            assert_eq!(done.get(), 3, "RepeatAttempt done");
        }
        other => return Err(format!("expected RepeatAttempt at node 2, got {other:?}")),
    }

    // Verify RepeatFinish at node 3
    match &parts.nodes[3].kind {
        CompiledNodeKind::RepeatFinish { result } => {
            assert_eq!(result.get(), 1, "RepeatFinish result");
        }
        other => return Err(format!("expected RepeatFinish at node 3, got {other:?}")),
    }

    // Verify Finish at node 4
    match &parts.nodes[4].kind {
        CompiledNodeKind::Finish { result } => {
            assert_eq!(result.get(), 0, "Finish result slot");
        }
        other => return Err(format!("expected Finish at node 4, got {other:?}")),
    }

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

    // Expected structure:
    // 0 = CollectStart { source: 0, body: 1, done: 3 }
    // 1 = Do { action: process_page, input: 1 }
    // 2 = CollectPage { collector_slot: 0, body: 1, done: 3 }
    // 3 = CollectFinish { collector_slot: 0 }
    // 4 = Finish { result: 0 }

    assert_eq!(
        parts.nodes.len(),
        5,
        "collect with do body should produce 5 nodes"
    );

    // Verify CollectStart at node 0
    match &parts.nodes[0].kind {
        CompiledNodeKind::CollectStart {
            source, body, done, ..
        } => {
            assert_eq!(source.get(), 0, "CollectStart source");
            assert_eq!(body.get(), 1, "CollectStart body");
            assert_eq!(done.get(), 3, "CollectStart done");
        }
        other => return Err(format!("expected CollectStart at node 0, got {other:?}")),
    }

    // Verify Do at node 1
    match &parts.nodes[1].kind {
        CompiledNodeKind::Do { action: _, input } => {
            assert_eq!(input.get(), 1, "Do input slot");
        }
        other => return Err(format!("expected Do at node 1, got {other:?}")),
    }

    // Verify CollectPage at node 2
    match &parts.nodes[2].kind {
        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            done,
        } => {
            assert_eq!(collector_slot.get(), 0, "CollectPage collector_slot");
            assert_eq!(body.get(), 1, "CollectPage body");
            assert_eq!(done.get(), 3, "CollectPage done");
        }
        other => return Err(format!("expected CollectPage at node 2, got {other:?}")),
    }

    // Verify CollectFinish at node 3
    match &parts.nodes[3].kind {
        CompiledNodeKind::CollectFinish { collector_slot } => {
            assert_eq!(collector_slot.get(), 0, "CollectFinish collector_slot");
        }
        other => return Err(format!("expected CollectFinish at node 3, got {other:?}")),
    }

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

    // Expected structure:
    // 0 = ForEachStart { input: 0, item_slot: 1, body: 1, done: 3 }
    // 1 = Do { action: 0, input: 1 } (body step)
    // 2 = ForEachNext { iterator_slot: 1, body: 1, done: 3 }
    // 3 = Finish { result: 0 }

    assert_eq!(
        parts.nodes.len(),
        4,
        "for_each with do body should produce 4 nodes"
    );

    // Verify ForEachStart at node 0
    match &parts.nodes[0].kind {
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            body,
            done,
            ..
        } => {
            assert_eq!(input.get(), 0, "ForEachStart input");
            assert_eq!(item_slot.get(), 1, "ForEachStart item_slot");
            assert_eq!(body.get(), 1, "ForEachStart body");
            assert_eq!(done.get(), 3, "ForEachStart done");
        }
        other => return Err(format!("expected ForEachStart at node 0, got {other:?}")),
    }

    // Verify Do at node 1 (body step)
    match &parts.nodes[1].kind {
        CompiledNodeKind::Do { action, input } => {
            let _ = action; // suppress unused warning in test
            assert_eq!(action.get(), 0, "Do action id");
            assert_eq!(input.get(), 1, "Do input slot");
        }
        other => return Err(format!("expected Do at node 1, got {other:?}")),
    }

    // Verify ForEachNext at node 2
    match &parts.nodes[2].kind {
        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => {
            assert_eq!(iterator_slot.get(), 1, "ForEachNext iterator_slot");
            assert_eq!(body.get(), 1, "ForEachNext body");
            assert_eq!(done.get(), 3, "ForEachNext done");
        }
        other => return Err(format!("expected ForEachNext at node 2, got {other:?}")),
    }

    Ok(())
}

/// Tests that a `reduce` primitive with a `do` body lowers to final IR.
/// Re-enabled by vb-em8xu (vb-budget-reduce).
#[test]
fn nested_do_in_reduce_body_lowers_to_final_ir() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: fold\n    reduce:\n      variable: acc\n      input: \"0\"\n      initial: \"10\"\n      steps:\n        - id: add\n          do:\n            action: \"0\"\n            input: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml)?;
    let parts = workflow.to_parts();

    // Expected structure:
    // 0 = ReduceStart { input: 0, accumulator: 1, initial: const, body: 1, done: 3 }
    // 1 = Do { action: add_one, input: 1 }
    // 2 = ReduceNext { iterator_slot: 1, accumulator: 1, body: 1, done: 3 }
    // 3 = ReduceFinish { accumulator: 1 }
    // 4 = Finish { result: 0 }

    assert_eq!(
        parts.nodes.len(),
        5,
        "reduce with do body should produce 5 nodes"
    );

    // Verify ReduceStart at node 0
    match &parts.nodes[0].kind {
        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            body,
            done,
            ..
        } => {
            assert_eq!(input.get(), 0, "ReduceStart input");
            assert_eq!(accumulator.get(), 1, "ReduceStart accumulator");
            assert_eq!(body.get(), 1, "ReduceStart body");
            assert_eq!(done.get(), 3, "ReduceStart done");
        }
        other => return Err(format!("expected ReduceStart at node 0, got {other:?}")),
    }

    // Verify Do at node 1
    match &parts.nodes[1].kind {
        CompiledNodeKind::Do { action: _, input } => {
            assert_eq!(input.get(), 1, "Do input slot");
        }
        other => return Err(format!("expected Do at node 1, got {other:?}")),
    }

    // Verify ReduceNext at node 2
    match &parts.nodes[2].kind {
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body,
            done,
        } => {
            assert_eq!(iterator_slot.get(), 1, "ReduceNext iterator_slot");
            assert_eq!(accumulator.get(), 1, "ReduceNext accumulator");
            assert_eq!(body.get(), 1, "ReduceNext body");
            assert_eq!(done.get(), 3, "ReduceNext done");
        }
        other => return Err(format!("expected ReduceNext at node 2, got {other:?}")),
    }

    // Verify ReduceFinish at node 3
    match &parts.nodes[3].kind {
        CompiledNodeKind::ReduceFinish { accumulator } => {
            assert_eq!(accumulator.get(), 1, "ReduceFinish accumulator");
        }
        other => return Err(format!("expected ReduceFinish at node 3, got {other:?}")),
    }

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

    // Expected structure:
    // 0 = TogetherStart { branches: [1, 3], join: 5 }
    // 1 = TogetherBranch { branch: 0, entry: 2, join: 5 }
    // 2 = Do { action: left_action, input: 0 }
    // 3 = TogetherBranch { branch: 1, entry: 4, join: 5 }
    // 4 = Do { action: right_action, input: 1 }
    // 5 = TogetherJoin { branch_count: 2, accumulator: 0 }
    // 6 = Finish { result: 0 }

    assert_eq!(
        parts.nodes.len(),
        7,
        "together with do branches should produce 7 nodes"
    );

    // Verify TogetherStart at node 0
    match &parts.nodes[0].kind {
        CompiledNodeKind::TogetherStart { branches, join } => {
            let actual: Vec<u16> = branches.iter().map(|b| b.get()).collect();
            assert_eq!(actual, [1, 3], "TogetherStart branches");
            assert_eq!(join.get(), 5, "TogetherStart join");
        }
        other => return Err(format!("expected TogetherStart at node 0, got {other:?}")),
    }

    // Verify first TogetherBranch at node 1
    match &parts.nodes[1].kind {
        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            join,
            ..
        } => {
            assert_eq!(*branch, 0, "first TogetherBranch branch index");
            assert_eq!(entry.get(), 2, "first TogetherBranch entry");
            assert_eq!(join.get(), 5, "first TogetherBranch join");
        }
        other => {
            return Err(format!(
                "expected first TogetherBranch at node 1, got {other:?}"
            ));
        }
    }

    // Verify Do at node 2 (left action)
    match &parts.nodes[2].kind {
        CompiledNodeKind::Do { action: _, input } => {
            assert_eq!(input.get(), 0, "left Do input slot");
        }
        other => return Err(format!("expected left Do at node 2, got {other:?}")),
    }

    // Verify second TogetherBranch at node 3
    match &parts.nodes[3].kind {
        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            join,
            ..
        } => {
            assert_eq!(*branch, 1, "second TogetherBranch branch index");
            assert_eq!(entry.get(), 4, "second TogetherBranch entry");
            assert_eq!(join.get(), 5, "second TogetherBranch join");
        }
        other => {
            return Err(format!(
                "expected second TogetherBranch at node 3, got {other:?}"
            ));
        }
    }

    // Verify Do at node 4 (right action)
    match &parts.nodes[4].kind {
        CompiledNodeKind::Do { action: _, input } => {
            assert_eq!(input.get(), 1, "right Do input slot");
        }
        other => return Err(format!("expected right Do at node 4, got {other:?}")),
    }

    // Verify TogetherJoin at node 5
    match &parts.nodes[5].kind {
        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => {
            assert_eq!(*branch_count, 2, "TogetherJoin branch_count");
            assert_eq!(accumulator.get(), 0, "TogetherJoin accumulator");
        }
        other => return Err(format!("expected TogetherJoin at node 5, got {other:?}")),
    }

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
