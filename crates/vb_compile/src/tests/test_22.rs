#![forbid(unsafe_code)]
//! RED-PHASE tests for vb-yd5x: shared validated IR usage
//!
//! These tests prove the contract gap between compile and validate paths.
//! Tests compile but fail before implementation, demonstrating that
//! `lower_steps_to_ir` bypasses `vb_validate::shared` validation.
//!
//! After fixing `lower_steps_to_ir` to call shared validation first,
//! these tests should pass.

use vb_core::ids::{SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ExprProgram, WorkflowParts};
use vb_core::{CompiledWorkflow, ResourceContract};

use crate::{lower_steps_to_ir, validate_ir, CompileError, CompileErrors, ValidationError};

const MINIMAL_DIGEST: WorkflowDigest = WorkflowDigest::from_bytes([0u8; 32]);

fn make_parts(
    nodes: Vec<CompiledNode>,
    expressions: Vec<ExprProgram>,
    slot_count: u16,
) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("test_lower_steps"),
        digest: MINIMAL_DIGEST,
        nodes: nodes.into_boxed_slice(),
        expressions: expressions.into_boxed_slice(),
        accessors: Box::new([]),
        constants: Box::new([vb_core::ConstValue::I64(0)]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result_slot),
        },
    }
}

fn do_node(index: u16, action: vb_core::ActionId, input: SlotIdx) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do { action, input },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RED-PHASE: These tests PROVE the contract gap
// lower_steps_to_ir currently BYPASSES vb_validate::shared validation
// ─────────────────────────────────────────────────────────────────────────────

/// RED-PHASE: `lower_steps_to_ir` with Do.input == slot_count should return
/// `CompileError::Validation(ValidationError::SlotReferenceOutOfRange)`.
///
/// CURRENT BEHAVIOR (bug): `lower_steps_to_ir` bypasses Gate 9 and calls
/// `CompiledWorkflow::try_from_parts` directly, which does NOT validate Do.input.
/// This test FAILS before implementation.
///
/// AFTER FIX: `lower_steps_to_ir` should call `vb_validate::shared::validate`
/// before core construction, catching Do.input >= slot_count.
#[test]
fn lower_steps_to_ir_returns_slot_reference_out_of_range_when_do_input_exceeds_slot_count() {
    let nodes = vec![do_node(0, vb_core::ActionId::new(7), SlotIdx::new(1))];
    let parts = make_parts(nodes, vec![], 1);

    let result = lower_steps_to_ir(
        parts.nodes.into_vec(),
        vec![],
        vec![],
        parts.constants.into_vec(),
        parts.slot_count,
        parts.symbols_count,
        &parts.name,
        parts.digest,
    );

    assert!(
        matches!(
            result,
            Err(CompileErrors(ref errors))
            if errors.len() == 1
            && matches!(
                errors.first(),
                Some(CompileError::Validation(ValidationError::SlotReferenceOutOfRange {
                    slot: 1,
                    slot_count: 1,
                    context
                }) if context.contains("Do.input"))
            )
        ),
        "lower_steps_to_ir should return SlotReferenceOutOfRange for Do.input >= slot_count, got: {:?}",
        result
    );
}

/// RED-PHASE: `lower_steps_to_ir` with empty nodes should return
/// `CompileError::Workflow(WorkflowError::EmptyNodes)`.
///
/// This tests that core construction errors are preserved as Workflow errors.
#[test]
fn lower_steps_to_ir_returns_empty_nodes_when_node_vector_is_empty() {
    let parts = make_parts(vec![], vec![], 0);

    let result = lower_steps_to_ir(
        vec![],
        vec![],
        vec![],
        parts.constants.into_vec(),
        0,
        0,
        &parts.name,
        parts.digest,
    );

    assert!(
        matches!(
            result,
            Err(CompileErrors(ref errors))
            if errors.len() == 1
            && matches!(
                errors.first(),
                Some(CompileError::Workflow(vb_core::WorkflowError::EmptyNodes))
            )
        ),
        "lower_steps_to_ir should return EmptyNodes for empty node vector, got: {:?}",
        result
    );
}

/// RED-PHASE: `lower_steps_to_ir` with wrong node id should return
/// `CompileError::Workflow(WorkflowError::NodeIdMismatch)`.
///
/// CURRENT BEHAVIOR (bug): If shared validation is bypassed, node id mismatch
/// might not be caught at the right stage.
///
/// AFTER FIX: Shared validation first, then core construction, with proper error typing.
#[test]
fn lower_steps_to_ir_returns_node_id_mismatch_when_first_node_id_is_one() {
    let mut node = do_node(1, vb_core::ActionId::new(7), SlotIdx::new(0));
    node.id = StepIdx::new(1);
    let parts = make_parts(vec![node], vec![], 1);

    let result = lower_steps_to_ir(
        parts.nodes.into_vec(),
        vec![],
        vec![],
        parts.constants.into_vec(),
        parts.slot_count,
        parts.symbols_count,
        &parts.name,
        parts.digest,
    );

    assert!(
        matches!(
            result,
            Err(CompileErrors(ref errors))
            if errors.len() == 1
            && matches!(
                errors.first(),
                Some(CompileError::Workflow(vb_core::WorkflowError::NodeIdMismatch {
                    expected: StepIdx::new(0),
                    actual: StepIdx::new(1)
                }))
            )
        ),
        "lower_steps_to_ir should return NodeIdMismatch when first node id != 0, got: {:?}",
        result
    );
}

/// RED-PHASE: `validate_ir` correctly orders shared validation before core.
///
/// This test proves `validate_ir` returns Validation error for shared gate violation
/// BEFORE core construction would run.
#[test]
fn validate_ir_returns_slot_reference_out_of_range_before_core_acceptance() {
    let nodes = vec![do_node(0, vb_core::ActionId::new(7), SlotIdx::new(1))];
    let parts = make_parts(nodes, vec![], 1);

    let result = validate_ir(parts);

    assert!(
        matches!(
            result,
            Err(CompileErrors(ref errors))
            if errors.len() == 1
            && matches!(
                errors.first(),
                Some(CompileError::Validation(ValidationError::SlotReferenceOutOfRange {
                    slot: 1,
                    slot_count: 1,
                    context
                }) if context.contains("Do.input"))
            )
        ),
        "validate_ir should return SlotReferenceOutOfRange before core acceptance, got: {:?}",
        result
    );
}

/// RED-PHASE: `validate_ir` returns Workflow error when shared validation passes
/// but core construction fails.
#[test]
fn validate_ir_returns_workflow_error_when_shared_validation_passes_but_core_fails() {
    let parts = make_parts(vec![], vec![], 0);

    let result = validate_ir(parts);

    assert!(
        matches!(
            result,
            Err(CompileErrors(ref errors))
            if errors.len() == 1
            && matches!(
                errors.first(),
                Some(CompileError::Workflow(vb_core::WorkflowError::EmptyNodes))
            )
        ),
        "validate_ir should return Workflow error when core fails after shared validation, got: {:?}",
        result
    );
}

/// RED-PHASE: `validate_ir` accepts parts that pass both shared and core validation.
#[test]
fn validate_ir_returns_valid_workflow_when_parts_pass_all_validation() {
    let nodes = vec![finish_node(0, 0)];
    let parts = make_parts(nodes, vec![], 1);

    let result = validate_ir(parts);

    assert!(
        result.is_ok(),
        "validate_ir should return Ok for valid parts, got: {:?}",
        result
    );
}

/// RED-PHASE: `lower_steps_to_ir` with expression stack mismatch should return
/// `CompileError::Validation(ValidationError::ExpressionStackMismatch)`.
///
/// Gate 7 checks that declared max_stack matches computed stack depth.
/// Core's ExprProgram::try_from_parts checks bytecode validity but not max_stack accuracy.
///
/// CURRENT BEHAVIOR (bug): `lower_steps_to_ir` bypasses Gate 7.
///
/// AFTER FIX: Shared validation catches ExpressionStackMismatch.
#[test]
fn lower_steps_to_ir_returns_expression_stack_mismatch_when_declared_stack_is_wrong() {
    let nodes = vec![finish_node(0, 0)];
    let mut expr = ExprProgram {
        ops: Box::new([vb_core::ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 0,
    };
    let parts = make_parts(nodes, vec![expr], 1);

    let result = lower_steps_to_ir(
        parts.nodes.into_vec(),
        vec![expr.clone()],
        vec![],
        parts.constants.into_vec(),
        parts.slot_count,
        parts.symbols_count,
        &parts.name,
        parts.digest,
    );

    assert!(
        matches!(
            result,
            Err(CompileErrors(ref errors))
            if errors.len() == 1
            && matches!(
                errors.first(),
                Some(CompileError::Validation(ValidationError::ExpressionStackMismatch {
                    expr_index: 0,
                    declared: 0,
                    computed: 1
                }))
            )
        ),
        "lower_steps_to_ir should return ExpressionStackMismatch for wrong max_stack, got: {:?}",
        result
    );
}

/// RED-PHASE: `validate_ir` catches expression stack mismatch via Gate 7.
#[test]
fn validate_ir_returns_expression_stack_mismatch_via_gate_7() {
    let nodes = vec![finish_node(0, 0)];
    let expr = ExprProgram {
        ops: Box::new([vb_core::ExprOp::LoadSlot(SlotIdx::new(0))]),
        max_stack: 0,
    };
    let parts = make_parts(nodes, vec![expr], 1);

    let result = validate_ir(parts);

    assert!(
        matches!(
            result,
            Err(CompileErrors(ref errors))
            if errors.len() == 1
            && matches!(
                errors.first(),
                Some(CompileError::Validation(ValidationError::ExpressionStackMismatch {
                    expr_index: 0,
                    declared: 0,
                    computed: 1
                }))
            )
        ),
        "validate_ir should catch ExpressionStackMismatch via Gate 7, got: {:?}",
        result
    );
}

/// RED-PHASE: Prove `lower_steps_to_ir` output passes shared validation.
///
/// After fixing the bypass bug, the workflow returned by `lower_steps_to_ir`
/// should have parts that pass `vb_validate::shared::validate`.
#[test]
fn lower_steps_to_ir_output_passes_shared_validation() {
    let nodes = vec![finish_node(0, 0)];
    let parts = make_parts(nodes, vec![], 1);

    let result = lower_steps_to_ir(
        parts.nodes.into_vec(),
        vec![],
        vec![],
        parts.constants.into_vec(),
        parts.slot_count,
        parts.symbols_count,
        &parts.name,
        parts.digest,
    );

    assert!(
        result.is_ok(),
        "lower_steps_to_ir should return Ok for valid input, got: {:?}",
        result
    );

    let workflow = result.unwrap();
    let output_parts = workflow.to_parts();
    let validate_result = vb_validate::shared::validate(&output_parts);
    assert!(
        validate_result.is_ok(),
        "lower_steps_to_ir output should pass shared validation, got: {:?}",
        validate_result
    );
}

/// RED-PHASE: `validate_ir` output passes shared validation (round-trip proof).
#[test]
fn validate_ir_output_passes_shared_validation() {
    let nodes = vec![finish_node(0, 0)];
    let parts = make_parts(nodes, vec![], 1);

    let result = validate_ir(parts);
    assert!(result.is_ok(), "validate_ir should succeed for valid parts");

    let workflow = result.unwrap();
    let output_parts = workflow.to_parts();

    let validate_result = vb_validate::shared::validate(&output_parts);
    assert!(
        validate_result.is_ok(),
        "validate_ir output should pass shared validation, got: {:?}",
        validate_result
    );
}

/// RED-PHASE: `compile_workflow_with_contracts` rejects missing action contract.
#[test]
fn compile_workflow_with_contracts_rejects_missing_action_contract() {
    let source = br#"version: velvet-ballastics/v1
name: test_do
when:
  manual: {}
steps:
  - id: do_it
    do:
      action: 7
      input: {}
  - id: done
    finish:
      result: 0
"#;

    let result = crate::compile_workflow_with_contracts(source, &[]);

    assert!(
        matches!(
            result,
            Err(CompileErrors(ref errors))
            if errors.len() == 1
            && matches!(
                errors.first(),
                Some(CompileError::Validation(ValidationError::ActionContractMissing {
                    action_id,
                    node_index: 0
                }) if action_id.get() == 7)
            )
        ),
        "compile_workflow_with_contracts should reject missing action contract, got: {:?}",
        result
    );
}

/// RED-PHASE: `compile_workflow_with_contracts` rejects orphan action contract.
#[test]
fn compile_workflow_with_contracts_rejects_orphan_action_contract() {
    let source = br#"version: velvet-ballastics/v1
name: test_no_do
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let orphan_contract = vb_core::ActionContract {
        id: vb_core::ActionId::new(99),
        side_effect: vb_core::SideEffect::None,
        retry_safety: vb_core::RetrySafety::Safe,
        idempotency: vb_core::Idempotency::DeterministicPure,
    };

    let result = crate::compile_workflow_with_contracts(source, &[orphan_contract]);

    assert!(
        matches!(
            result,
            Err(CompileErrors(ref errors))
            if errors.len() == 1
            && matches!(
                errors.first(),
                Some(CompileError::Validation(ValidationError::ActionContractOrphan {
                    action_id
                }) if action_id.get() == 99)
            )
        ),
        "compile_workflow_with_contracts should reject orphan action contract, got: {:?}",
        result
    );
}

/// RED-PHASE: `compile_workflow_with_contracts` accepts valid action contract.
#[test]
fn compile_workflow_with_contracts_accepts_valid_action_contract() {
    let source = br#"version: velvet-ballastics/v1
name: test_do
when:
  manual: {}
steps:
  - id: do_it
    do:
      action: 7
      input: {}
  - id: done
    finish:
      result: 0
"#;

    let valid_contract = vb_core::ActionContract {
        id: vb_core::ActionId::new(7),
        side_effect: vb_core::SideEffect::None,
        retry_safety: vb_core::RetrySafety::Safe,
        idempotency: vb_core::Idempotency::DeterministicPure,
    };

    let result = crate::compile_workflow_with_contracts(source, &[valid_contract]);

    assert!(
        result.is_ok(),
        "compile_workflow_with_contracts should accept valid action contract, got: {:?}",
        result
    );
}

/// RED-PHASE: Plain `vb_validate::shared::validate` does NOT claim gate 12.
///
/// Even when action contracts are missing, plain validation should succeed
/// because gate 12 requires contracts and is not part of plain validation.
#[test]
fn plain_validate_does_not_claim_gate_12_for_missing_contracts() {
    let nodes = vec![do_node(0, vb_core::ActionId::new(7), SlotIdx::new(0))];
    let parts = make_parts(nodes, vec![], 1);

    let result = vb_validate::shared::validate(&parts);

    assert!(
        result.is_ok(),
        "plain validate should NOT check gate 12 for Do with action 7, got: {:?}",
        result
    );
}

/// RED-PHASE: `vb_validate::shared::validate_with_contracts` catches missing contracts.
#[test]
fn validate_with_contracts_returns_action_contract_missing() {
    let nodes = vec![do_node(0, vb_core::ActionId::new(7), SlotIdx::new(0))];
    let parts = make_parts(nodes, vec![], 1);

    let result = vb_validate::shared::validate_with_contracts(&parts, &[]);

    assert!(
        matches!(
            result,
            Err(ValidationError::ActionContractMissing {
                action_id,
                node_index: 0
            }) if action_id.get() == 7
        ),
        "validate_with_contracts should return ActionContractMissing, got: {:?}",
        result
    );
}

/// RED-PHASE: `vb_validate::shared::validate_with_contracts` catches orphan contracts.
#[test]
fn validate_with_contracts_returns_action_contract_orphan() {
    let nodes = vec![finish_node(0, 0)];
    let parts = make_parts(nodes, vec![], 1);

    let orphan_contract = vb_core::ActionContract {
        id: vb_core::ActionId::new(99),
        side_effect: vb_core::SideEffect::None,
        retry_safety: vb_core::RetrySafety::Safe,
        idempotency: vb_core::Idempotency::DeterministicPure,
    };

    let result = vb_validate::shared::validate_with_contracts(&parts, &[orphan_contract]);

    assert!(
        matches!(
            result,
            Err(ValidationError::ActionContractOrphan {
                action_id
            }) if action_id.get() == 99
        ),
        "validate_with_contracts should return ActionContractOrphan, got: {:?}",
        result
    );
}

/// RED-PHASE: Error preservation - ValidationError::SlotReferenceOutOfRange
/// preserves exact payload through CompileError::Validation conversion.
#[test]
fn compile_error_preserves_slot_reference_out_of_range_variant() {
    let inner = ValidationError::SlotReferenceOutOfRange {
        slot: 5,
        slot_count: 3,
        context: Box::from("Do.input"),
    };
    let compile_err = CompileError::Validation(inner.clone());

    match compile_err {
        CompileError::Validation(ValidationError::SlotReferenceOutOfRange {
            slot,
            slot_count,
            context,
        }) => {
            assert_eq!(slot, 5);
            assert_eq!(slot_count, 3);
            assert_eq!(&*context, "Do.input");
        }
        other => panic!("Expected SlotReferenceOutOfRange, got: {:?}", other),
    }
}

/// RED-PHASE: Error preservation - ValidationError::ExpressionStackMismatch
/// preserves exact payload.
#[test]
fn compile_error_preserves_expression_stack_mismatch_variant() {
    let inner = ValidationError::ExpressionStackMismatch {
        expr_index: 2,
        declared: 10,
        computed: 3,
    };
    let compile_err = CompileError::Validation(inner.clone());

    match compile_err {
        CompileError::Validation(ValidationError::ExpressionStackMismatch {
            expr_index,
            declared,
            computed,
        }) => {
            assert_eq!(expr_index, 2);
            assert_eq!(declared, 10);
            assert_eq!(computed, 3);
        }
        other => panic!("Expected ExpressionStackMismatch, got: {:?}", other),
    }
}

/// RED-PHASE: Error preservation - WorkflowError variants preserve through
/// CompileError::Workflow conversion.
#[test]
fn compile_error_preserves_workflow_error_variant() {
    let inner = vb_core::WorkflowError::EmptyNodes;
    let compile_err = CompileError::Workflow(inner);

    match &compile_err {
        CompileError::Workflow(vb_core::WorkflowError::EmptyNodes) => {}
        other => panic!("Expected EmptyNodes, got: {:?}", other),
    }
}

/// RED-PHASE: Error preservation - ActionContractMissing preserves.
#[test]
fn compile_error_preserves_action_contract_missing_variant() {
    let inner = ValidationError::ActionContractMissing {
        action_id: vb_core::ActionId::new(42),
        node_index: 5,
    };
    let compile_err = CompileError::Validation(inner);

    match compile_err {
        CompileError::Validation(ValidationError::ActionContractMissing {
            action_id,
            node_index,
        }) => {
            assert_eq!(action_id.get(), 42);
            assert_eq!(node_index, 5);
        }
        other => panic!("Expected ActionContractMissing, got: {:?}", other),
    }
}

/// RED-PHASE: Error preservation - ActionContractOrphan preserves.
#[test]
fn compile_error_preserves_action_contract_orphan_variant() {
    let inner = ValidationError::ActionContractOrphan {
        action_id: vb_core::ActionId::new(99),
    };
    let compile_err = CompileError::Validation(inner);

    match compile_err {
        CompileError::Validation(ValidationError::ActionContractOrphan { action_id }) => {
            assert_eq!(action_id.get(), 99);
        }
        other => panic!("Expected ActionContractOrphan, got: {:?}", other),
    }
}

/// RED-PHASE: CompileErrors contains exactly one error for isolated failures.
#[test]
fn compile_errors_contains_exactly_one_error_for_isolated_validation_failure() {
    let nodes = vec![do_node(0, vb_core::ActionId::new(7), SlotIdx::new(1))];
    let parts = make_parts(nodes, vec![], 1);

    let result = lower_steps_to_ir(
        parts.nodes.into_vec(),
        vec![],
        vec![],
        parts.constants.into_vec(),
        parts.slot_count,
        parts.symbols_count,
        &parts.name,
        parts.digest,
    );

    match result {
        Err(CompileErrors(ref errors)) => {
            assert_eq!(
                errors.len(),
                1,
                "Expected exactly 1 error for isolated validation failure, got {}",
                errors.len()
            );
        }
        Ok(_) => panic!("Expected error, got Ok"),
        other => panic!("Expected CompileErrors, got: {:?}", other),
    }
}
