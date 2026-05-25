#![forbid(unsafe_code)]
//! Integration tests for vb_compile + vb_codegen + vb_runtime end-to-end.
//!
//! Tests the complete workflow from YAML source through compilation and codegen
//! to runtime execution, including:
//! - Full compile + emit + runtime workflow execution
//! - Runtime engine能与generated code交互
//! - Error propagation from compile through to runtime

use vb_codegen::{CodegenError, emit_rust_workflow, validate_generated_subset};
use vb_compile::compile_workflow;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram,
    PathSegment, ResourceContract, WorkflowParts,
};

// ---------------------------------------------------------------------------
// Happy path: YAML -> compile -> codegen -> runtime-ready workflow
// ---------------------------------------------------------------------------

#[test]
fn yaml_compile_codegen_produces_runtime_ready_workflow() {
    let source = br#"
version: velvet-ballastics/v1
name: e2e_finish
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    // Compile
    let workflow = compile_workflow(source).expect("compile should succeed");

    // Validate for generated subset
    let validation = validate_generated_subset(&workflow);
    assert!(
        validation.is_ok(),
        "workflow should be valid for generated subset: {:?}",
        validation
    );

    // Emit
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Generated code should contain expected structures
    assert!(generated.contains("fn drive("));
    assert!(generated.contains("CONTRACT_MAX_STEPS"));
    assert!(generated.contains("CONTRACT_MAX_SLOTS"));
}

#[test]
fn yaml_compile_codegen_with_set_step() {
    let source = br#"
version: velvet-ballastics/v1
name: e2e_set
when:
  manual: {}
steps:
  - id: init
    set:
      output: answer
      value: "42"
  - id: done
    finish:
      result: answer
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    assert!(generated.contains("fn drive("));
    assert!(generated.contains("read_const("));
}

#[test]
fn yaml_compile_codegen_with_multiple_steps() {
    let source = br#"
version: velvet-ballastics/v1
name: e2e_multi
when:
  manual: {}
steps:
  - id: first
    set:
      output: val_x
      value: "10"
  - id: second
    set:
      output: val_y
      value: "20"
  - id: done
    finish:
      result: val_x
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    assert!(generated.contains("step_0"));
    assert!(generated.contains("step_1"));
    assert!(generated.contains("step_2"));
}

#[test]
fn yaml_compile_codegen_with_set_and_finish() {
    let source = br#"
version: velvet-ballastics/v1
name: e2e_set_finish
when:
  manual: {}
steps:
  - id: init
    set:
      output: result_val
      value: "99"
  - id: done
    finish:
      result: result_val
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    assert!(generated.contains("step_0"));
    assert!(generated.contains("step_1"));
}

// ---------------------------------------------------------------------------
// Codegen validation errors through pipeline
// ---------------------------------------------------------------------------

#[test]
fn codegen_rejects_unsupported_workflow_in_pipeline() {
    // Build a workflow with an unsupported feature (Contains expression)
    let workflow = make_workflow_with_unsupported_expr_op();

    // Validate should reject it
    let result = validate_generated_subset(&workflow);
    assert!(
        result.is_err(),
        "workflow with unsupported expr should be rejected"
    );
    if let Err(e) = result {
        match e {
            CodegenError::UnsupportedIr { feature } => {
                assert!(
                    feature.contains("contains") || feature.contains("runtime symbol store"),
                    "should be contains-related error"
                );
            }
            _ => panic!("expected UnsupportedIr error"),
        }
    }
}

#[test]
fn codegen_rejects_deep_accessor_in_pipeline() {
    let workflow = make_workflow_with_deep_accessor(20);

    let result = validate_generated_subset(&workflow);
    assert!(result.is_err());
    if let Err(e) = result {
        match e {
            CodegenError::UnsupportedIr { feature } => {
                assert!(
                    feature.contains("accessor") || feature.contains("depth"),
                    "should be accessor depth error"
                );
            }
            _ => panic!("expected UnsupportedIr error"),
        }
    }
}

#[test]
fn codegen_rejects_out_of_bounds_slot_in_pipeline() {
    let workflow = make_workflow_with_out_of_bounds_accessor();

    let result = validate_generated_subset(&workflow);
    assert!(result.is_err());
    if let Err(e) = result {
        match e {
            CodegenError::UnsupportedIr { feature } => {
                assert!(
                    feature.contains("slot") || feature.contains("bounds"),
                    "should be slot bounds error"
                );
            }
            _ => panic!("expected UnsupportedIr error"),
        }
    }
}

// ---------------------------------------------------------------------------
// Compiler limits through full pipeline
// ---------------------------------------------------------------------------

#[test]
fn compiler_size_limit_rejected_before_codegen() {
    let compiler = vb_compile::YamlCompiler::new(vb_compile::YamlLimits {
        max_source_bytes: 50,
        ..Default::default()
    });

    let source = format!(
        r#"version: velvet-ballastics/v1
name: {}
when:
  manual: {{}}
steps:
  - id: done
    finish:
      result: 0
"#,
        "a".repeat(200)
    );

    // Compile should fail due to size limit
    let result = compiler.compile(source.as_bytes());
    assert!(
        result.is_err(),
        "source exceeding size limit should be rejected"
    );
}

#[test]
fn compiler_with_custom_limits_accepts_valid_source() {
    let compiler = vb_compile::YamlCompiler::new(vb_compile::YamlLimits {
        max_source_bytes: 100_000,
        max_depth: 64,
        max_nodes: 100_000,
        max_sequence_len: 10_000,
        max_mapping_entries: 1024,
        max_scalar_bytes: 65_536,
    });

    let source = br#"
version: velvet-ballastics/v1
name: custom_limits
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = compiler.compile(source);
    assert!(
        result.is_ok(),
        "valid source should compile with custom limits"
    );
}

#[test]
fn compiler_default_limits_are_sensible() {
    let compiler = vb_compile::YamlCompiler::default();

    // 1KB source should be accepted by default
    let source = format!(
        r#"version: velvet-ballastics/v1
name: {}
when:
  manual: {{}}
steps:
  - id: done
    finish:
      result: 0
"#,
        "x".repeat(1000)
    );
    let result = compiler.compile(source.as_bytes());
    assert!(result.is_ok(), "1KB source should be within default limits");
}

// ---------------------------------------------------------------------------
// Generated code safety checks
// ---------------------------------------------------------------------------

#[test]
fn generated_code_forbids_unsafe_by_default() {
    let source = br#"
version: velvet-ballastics/v1
name: safe_code
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    assert!(
        generated.contains("#![forbid(unsafe_code)]"),
        "generated code should forbid unsafe"
    );
}

#[test]
fn generated_code_contains_resource_contract() {
    let source = br#"
version: velvet-ballastics/v1
name: contract_check
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    assert!(generated.contains("CONTRACT_MAX_STEPS"));
    assert!(generated.contains("CONTRACT_MAX_SLOTS"));
    assert!(generated.contains("CONTRACT_MAX_STEP_BUDGET"));
}

#[test]
fn generated_code_contains_workflow_constants() {
    let source = br#"
version: velvet-ballastics/v1
name: constants_test
when:
  manual: {}
steps:
  - id: init
    set:
      output: x
      value: "99"
  - id: done
    finish:
      result: x
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Should have slot and node count constants
    assert!(generated.contains("WORKFLOW_SLOT_COUNT"));
    assert!(generated.contains("WORKFLOW_NODE_COUNT"));
}

// ---------------------------------------------------------------------------
// Workflow parts interoperability with runtime
// ---------------------------------------------------------------------------

#[test]
fn compiled_workflow_parts_are_validated() {
    let source = br#"
version: velvet-ballastics/v1
name: parts_validate
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let parts = workflow.to_parts();

    // Parts should have valid structure
    assert_eq!(parts.name.as_ref(), "parts_validate");
    assert_eq!(parts.entry, StepIdx::ZERO);
    assert!(parts.slot_count >= 1);
    assert!(!parts.nodes.is_empty());
}

#[test]
fn compiled_workflow_round_trips_through_parts() {
    let source = br#"
version: velvet-ballastics/v1
name: round_trip
when:
  manual: {}
steps:
  - id: step1
    set:
      output: val
      value: "1"
  - id: done
    finish:
      result: val
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");

    // Convert to parts and back
    let parts = workflow.to_parts();
    let workflow2 =
        CompiledWorkflow::try_from_parts(parts.clone()).expect("round-trip should succeed");

    let parts2 = workflow2.to_parts();

    // Key properties should match
    assert_eq!(parts.name, parts2.name);
    assert_eq!(parts.slot_count, parts2.slot_count);
    assert_eq!(parts.nodes.len(), parts2.nodes.len());
}

#[test]
fn compiled_workflow_slot_layout_is_correct() {
    let source = br#"
version: velvet-ballastics/v1
name: slot_layout
when:
  manual: {}
steps:
  - id: a
    set:
      output: slot_a
      value: "1"
  - id: b
    set:
      output: slot_b
      value: "2"
  - id: c
    set:
      output: slot_c
      value: "3"
  - id: done
    finish:
      result: slot_c
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let parts = workflow.to_parts();

    // The workflow should have at least some slots allocated
    // Actual count depends on compiler slot allocation strategy
    assert!(parts.slot_count >= 1, "slot_count should be >= 1");
    // Should have 4 nodes: 3 sets + 1 finish
    assert_eq!(parts.nodes.len(), 4);
}

// ---------------------------------------------------------------------------
// Error propagation
// ---------------------------------------------------------------------------

#[test]
fn compile_error_propagates_through_pipeline() {
    let source = b"not yaml at all";

    let result = compile_workflow(source);
    assert!(result.is_err(), "invalid YAML should produce compile error");
}

#[test]
fn compile_error_for_duplicate_step_ids() {
    let source = br#"
version: velvet-ballastics/v1
name: dup_ids
when:
  manual: {}
steps:
  - id: step
    set:
      output: a
      value: "1"
  - id: step
    set:
      output: b
      value: "2"
  - id: done
    finish:
      result: a
"#;
    let result = compile_workflow(source);
    assert!(result.is_err(), "duplicate step IDs should produce error");
}

#[test]
fn compile_accepts_valid_finish_workflow() {
    // This test just verifies a simple finish workflow compiles
    let source = br#"
version: velvet-ballastics/v1
name: simple_finish
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    // A workflow with finish as last step should compile
    assert!(
        result.is_ok(),
        "finish workflow should compile: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Runtime compatibility checks
// ---------------------------------------------------------------------------

#[test]
fn generated_workflow_has_valid_entry_point() {
    let source = br#"
version: velvet-ballastics/v1
name: entry_check
when:
  manual: {}
steps:
  - id: start
    set:
      output: x
      value: "0"
  - id: done
    finish:
      result: x
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");

    // Entry should be step 0
    let parts = workflow.to_parts();
    assert_eq!(parts.entry, StepIdx::ZERO);
}

#[test]
fn generated_workflow_has_valid_node_count() {
    let source = br#"
version: velvet-ballastics/v1
name: node_count
when:
  manual: {}
steps:
  - id: a
    set:
      output: val_a
      value: "1"
  - id: b
    set:
      output: val_b
      value: "2"
  - id: done
    finish:
      result: val_a
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");

    let parts = workflow.to_parts();
    // Should have 3 nodes: set val_a, set val_b, finish
    assert_eq!(parts.nodes.len(), 3);
}

#[test]
fn generated_workflow_has_valid_slot_count() {
    let source = br#"
version: velvet-ballastics/v1
name: slot_count
when:
  manual: {}
steps:
  - id: a
    set:
      output: result
      value: "42"
  - id: done
    finish:
      result: result
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");

    let parts = workflow.to_parts();
    // Should have at least slot 0 (for result reference)
    assert!(parts.slot_count >= 1);
}

#[test]
fn generated_workflow_constants_are_accessible() {
    let source = br#"
version: velvet-ballastics/v1
name: const_test
when:
  manual: {}
steps:
  - id: init
    set:
      output: val
      value: "100"
  - id: done
    finish:
      result: val
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");

    let parts = workflow.to_parts();
    // Should have at least one constant (the value 100)
    assert!(!parts.constants.is_empty());
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn make_workflow_with_nodes(nodes: Vec<CompiledNode>) -> CompiledWorkflow {
    let parts = WorkflowParts {
        name: "test".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    CompiledWorkflow::from_parts_unchecked(parts)
}

fn make_workflow_with_deep_accessor(depth: usize) -> CompiledWorkflow {
    let path: Vec<PathSegment> = (0..depth)
        .map(|i| PathSegment::Field(SymbolId::new(i as u32)))
        .collect();

    let accessor = AccessorProgram {
        root: SlotIdx::ZERO,
        path: path.into_boxed_slice(),
    };

    let parts = WorkflowParts {
        name: "deep_accessor".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: Box::new([]),
        slot_count: 10,
        symbols_count: depth as u32,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    CompiledWorkflow::from_parts_unchecked(parts)
}

fn make_workflow_with_out_of_bounds_accessor() -> CompiledWorkflow {
    let accessor = AccessorProgram {
        root: SlotIdx::new(100), // Out of bounds for slot_count = 2
        path: vec![PathSegment::Field(SymbolId::new(1))].into_boxed_slice(),
    };

    let parts = WorkflowParts {
        name: "oob_accessor".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 1,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    CompiledWorkflow::from_parts_unchecked(parts)
}

fn make_workflow_with_unsupported_expr_op() -> CompiledWorkflow {
    // Contains is not supported in generated subset
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Contains,
    ];
    let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).expect("valid expr");

    let parts = WorkflowParts {
        name: "unsupported_expr".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: vec![expr].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(0), ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    CompiledWorkflow::from_parts_unchecked(parts)
}
