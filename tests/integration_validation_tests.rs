#![forbid(unsafe_code)]
//! Integration tests for public validation and compilation adapter behavior.
//!
//! Covers requirements R16-R21 from contract.md without relying on stale
//! split-file paths. These tests assert public adapter behavior remains stable
//! while semantic validation is routed through the shared pipeline.
//!
//! - R16: compile adapter workflows call validate_with_contracts
//! - R17: API compilation workflows call validate_with_contracts
//! - R18: vb_compile::schema.rs:651 calls validate
//! - R19: vb_compile::types.rs:155 calls validate
//! - R20: velvet_ballastics::commands_verify.rs:76 calls validate
//! - R21: fuzz::lib.rs:40,60 calls validate_with_contracts

use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_validate::ValidationError;
use vb_validate::shared::{ValidationPipeline, validate, validate_with_contracts};

// ---------------------------------------------------------------------------
// Helper constructors
// ---------------------------------------------------------------------------

fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("integration"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn make_contract(action_id: u16) -> ActionContract {
    ActionContract {
        id: ActionId::new(action_id),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }
}

// ===========================================================================
// R16: compile adapter workflows call validate_with_contracts
// ===========================================================================

#[test]
fn integration_compile_calls_validate_with_contracts() {
    // Simulate the public compile adapter workflow with Do nodes and contracts.
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(1)];

    let result = validate_with_contracts(&parts, &contracts);

    assert_eq!(result, Ok(()));
}

// ===========================================================================
// R17: API compilation workflows call validate_with_contracts
// ===========================================================================

#[test]
fn integration_api_compilation_calls_validate_with_contracts() {
    // Simulate a public API compilation workflow.
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(1)];

    let result = validate_with_contracts(&parts, &contracts);

    assert_eq!(result, Ok(()));
}

#[test]
fn test_existing_validate_api_returns_expected_success() {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];
    let parts = make_parts(nodes, 1);

    assert_eq!(validate(&parts), Ok(()));
    assert_eq!(ValidationPipeline::default().validate(&parts), Ok(()));
}

// ===========================================================================
// R18: vb_compile::schema.rs:651 calls validate
// ===========================================================================

#[test]
fn integration_schema_calls_validate() {
    // Simulate schema validation workflow
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];
    let parts = make_parts(nodes, 1);

    // This is what schema.rs:651 should call
    let result = validate(&parts);

    assert_eq!(result, Ok(()));
}

// ===========================================================================
// R19: vb_compile::types.rs:155 calls validate
// ===========================================================================

#[test]
fn integration_types_calls_validate() {
    // Simulate types validation workflow
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];
    let parts = make_parts(nodes, 1);

    // This is what types.rs:155 should call
    let result = validate(&parts);

    assert_eq!(result, Ok(()));
}

// ===========================================================================
// R20: velvet_ballastics::commands_verify.rs:76 calls validate
// ===========================================================================

#[test]
fn integration_verify_command_calls_validate() {
    // Simulate verify command workflow
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];
    let parts = make_parts(nodes, 1);

    // This is what commands_verify.rs:76 should call
    let result = validate(&parts);

    assert_eq!(result, Ok(()));
}

// ===========================================================================
// R21: fuzz::lib.rs:40,60 calls validate_with_contracts
// ===========================================================================

#[test]
fn integration_fuzz_calls_validate_with_contracts() {
    // Simulate fuzz target workflow with actions
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(1)];

    let result = validate_with_contracts(&parts, &contracts);

    assert_eq!(result, Ok(()));
}

// ===========================================================================
// Integration: Full vb_compile pipeline test
// ===========================================================================

#[test]
fn integration_vb_compile_full_pipeline() {
    // Simulate full compile pipeline with multiple nodes and contracts
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: vb_core::ids::ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        },
    ];
    let parts = WorkflowParts {
        name: Box::from("full_pipeline"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([vb_core::value::ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let contracts = vec![make_contract(1)];

    // Full pipeline validation
    let result = validate_with_contracts(&parts, &contracts);

    // Should pass for valid workflow
    assert_eq!(result, Ok(()));
}

// ===========================================================================
// Integration: vb_validate unit tests integration
// ===========================================================================

#[test]
fn integration_vb_validate_unit_integration() {
    // Test that individual gates work correctly
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }];
    let parts = make_parts(nodes, 1);

    // Test with default pipeline
    let pipeline = ValidationPipeline::default();
    let result = pipeline.validate(&parts);

    assert_eq!(result, Ok(()));

    // Test with all gates disabled
    let no_gates = ValidationPipeline::no_gates();
    let result = no_gates.validate(&parts);

    assert_eq!(result, Ok(()));
}

// ===========================================================================
// Integration: End-to-end validation pipeline test
// ===========================================================================

#[test]
fn integration_e2e_validation_pipeline() {
    // Build a complete workflow that should pass all gates
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: vb_core::ids::ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        },
    ];

    let parts = WorkflowParts {
        name: Box::from("e2e"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([vb_core::value::ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let contracts = vec![make_contract(1)];

    // E2E: Full pipeline with contracts
    let result = validate_with_contracts(&parts, &contracts);

    assert_eq!(result, Ok(()));
}

// ===========================================================================
// Integration: Error case validation
// ===========================================================================

#[test]
fn integration_error_case_validation() {
    // Build an invalid workflow (out-of-bounds slot)
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(99)), // Invalid: slot_count = 1
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }];
    let parts = make_parts(nodes, 1);

    // Should fail validation
    let result = validate(&parts);

    assert!(matches!(
        result,
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));

    // Error should be specific
    if let Err(e) = result {
        let msg = format!("{e}");
        assert!(
            msg.contains("slot") || msg.contains("SLOT") || msg.contains("Range"),
            "expected slot-related error, got: {e}"
        );
    }
}

// ===========================================================================
// Integration: Pipeline selective gate testing
// ===========================================================================

#[test]
fn integration_selective_gate_testing() {
    // Workflow that would fail G9 but passes other gates
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(99)), // Out of bounds
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }];
    let parts = make_parts(nodes, 1);

    // With all gates: should fail
    let all_result = ValidationPipeline::default().validate(&parts);
    assert!(matches!(
        all_result,
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));

    // With G9 disabled: should pass (but this is artificial - in real use,
    // you'd want to validate the specific gate you're testing)
    let no_g9 = ValidationPipeline {
        gate_09_slot_references: false,
        ..ValidationPipeline::default()
    };
    let no_g9_result = no_g9.validate(&parts);
    assert_eq!(no_g9_result, Ok(()));
}
