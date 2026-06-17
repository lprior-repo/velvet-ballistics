#![allow(clippy::expect_used)]
//! Integration tests for compiling workflows with Choose body steps.
//!
//! Bead: vb-xi2f.13
//! These tests exercise `compile_workflow` (the public API) end-to-end,
//! verifying that YAML workflows with `choose` primitives containing
//! `Set` and `Do` body steps produce correct compiled IR.

use vb_compile::compile_workflow;
use vb_core::{CompiledNodeKind, StepIdx};

// ─────────────────────────────────────────────────────────────────
// Helper: build YAML source string
// ─────────────────────────────────────────────────────────────────

fn make_choose_yaml(body_yaml: &str) -> Vec<u8> {
    format!(
        r#"version: velvet-ballistics/v1
name: test_choose_body
when:
  manual: {{}}
steps:
  - id: setup
    set:
      output: result
      value: "1"
  - id: pick
    choose:
      branches:
        - when: "0"
          steps:
{body_yaml}
      otherwise: done
  - id: done
    finish:
      result: result
"#,
    )
    .into_bytes()
}

// ─────────────────────────────────────────────────────────────────
// Test 13: compile_workflow with choose + Set body
// Plan N16 / RRO-B012
// ─────────────────────────────────────────────────────────────────

/// Verifies that a YAML workflow with a `choose` containing a `Set`
/// body step compiles successfully and produces correct IR with a
/// ChooseSlot node and a body SetConst node.
#[test]
fn compile_workflow_choose_body_set_success() {
    // Given: YAML with choose + Set body
    let yaml = make_choose_yaml(
        r#"            - id: body_s
              set:
                output: out_0
                value: "42""#,
    );
    // When: compile_workflow is called
    let wf = compile_workflow(&yaml).expect("must compile valid choose with Set body");
    // Then: workflow has correct structure
    let node_count = wf.node_count();
    assert!(
        node_count >= 4,
        "workflow must have at least 4 nodes (setup + ChooseSlot + body + finish), got {node_count}"
    );
    // Verify the ChooseSlot node exists at index 1
    let choose_node = wf
        .node(StepIdx::new(1))
        .expect("node at index 1 must exist");
    assert!(
        matches!(choose_node.kind, CompiledNodeKind::ChooseSlot { .. }),
        "node at index 1 must be ChooseSlot, got {:?}",
        choose_node.kind
    );
    // Verify the body node exists at index 2
    let body_node = wf.node(StepIdx::new(2)).expect("body node must exist");
    assert!(
        matches!(body_node.kind, CompiledNodeKind::SetConst { .. }),
        "body node must be SetConst, got {:?}",
        body_node.kind
    );
    // Verify the body node chains to the finish node
    assert_eq!(
        body_node.next,
        Some(StepIdx::new(3)),
        "body node must chain to finish node at index 3"
    );
    // Verify ChooseSlot has branch target pointing to the body
    match &choose_node.kind {
        CompiledNodeKind::ChooseSlot { branches, .. } => {
            assert!(!branches.is_empty(), "must have at least one branch");
            let first_branch = branches
                .first()
                .expect("branches is non-empty (asserted above)");
            assert_eq!(
                first_branch.target,
                StepIdx::new(2),
                "branch target must point to body node"
            );
            assert_eq!(
                first_branch.condition.get(),
                0,
                "condition must be slot 0 (setup output)"
            );
        }
        other => assert!(
            matches!(other, CompiledNodeKind::ChooseSlot { .. }),
            "node 1 must be ChooseSlot (already asserted above), got {other:?}"
        ),
    }
}

// ─────────────────────────────────────────────────────────────────
// Test 14: compile_workflow with choose + Do body
// Plan N17 / RRO-B012
// ─────────────────────────────────────────────────────────────────

/// Verifies that a YAML workflow with a `choose` containing a `Do`
/// body step compiles successfully and produces a Do node with correct
/// chain links, branch target, and node positioning in the compiled IR.
#[test]
fn compile_workflow_choose_body_do_success() {
    // Given: YAML with choose + Do body
    let yaml = make_choose_yaml(
        r#"            - id: body_do
              do:
                action: "1"
                input: "0""#,
    );
    // When: compile_workflow is called
    let wf = compile_workflow(&yaml).expect("must compile valid choose with Do body");
    // Then: workflow has correct structure
    let node_count = wf.node_count();
    assert!(
        node_count >= 4,
        "workflow must have at least 4 nodes (setup + ChooseSlot + body + finish), got {node_count}"
    );
    // Verify the ChooseSlot node exists at index 1
    let choose_node = wf
        .node(StepIdx::new(1))
        .expect("node at index 1 must exist");
    assert!(
        matches!(choose_node.kind, CompiledNodeKind::ChooseSlot { .. }),
        "node at index 1 must be ChooseSlot, got {:?}",
        choose_node.kind
    );
    // Verify the Do body node exists at index 2
    let body_node = wf.node(StepIdx::new(2)).expect("body node must exist");
    assert!(
        matches!(body_node.kind, CompiledNodeKind::Do { .. }),
        "body node must be Do, got {:?}",
        body_node.kind
    );
    // Verify the Do body node chains to the finish node
    assert_eq!(
        body_node.next,
        Some(StepIdx::new(3)),
        "Do body node must chain to finish node at index 3"
    );
    // Verify ChooseSlot has branch target pointing to the Do body
    match &choose_node.kind {
        CompiledNodeKind::ChooseSlot { branches, .. } => {
            assert!(!branches.is_empty(), "must have at least one branch");
            let first_branch = branches
                .first()
                .expect("branches is non-empty (asserted above)");
            assert_eq!(
                first_branch.target,
                StepIdx::new(2),
                "branch target must point to Do body node"
            );
            assert_eq!(
                first_branch.condition.get(),
                0,
                "condition must be slot 0 (setup output)"
            );
        }
        other => assert!(
            matches!(other, CompiledNodeKind::ChooseSlot { .. }),
            "node 1 must be ChooseSlot (already asserted above), got {other:?}"
        ),
    }
}
