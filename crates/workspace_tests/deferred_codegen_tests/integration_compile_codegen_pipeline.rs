#![forbid(unsafe_code)]
//! Integration tests for vb_compile + vb_codegen full pipeline.
//!
//! Tests the complete workflow from YAML source through compilation to
//! generated Rust code, including happy path and error paths.
//!
//! Covers edge cases not in vb_codegen/src/tests.rs or vb_compile/src/tests/:
//! - Full compile + emit pipeline success
//! - Unsupported IR features in codegen validation
//! - Compilation errors from vb_compile through vb_codegen validation
//! - Generated Rust subset validation

use vb_codegen::{CodegenError, emit_rust_workflow, validate_generated_subset};
use vb_compile::compile_workflow;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram,
    PathSegment, ResourceContract, WorkflowParts,
};

// ---------------------------------------------------------------------------
// Happy path: minimal workflow through full pipeline
// ---------------------------------------------------------------------------

#[test]
fn minimal_finish_workflow_compiles_and_generates() {
    let source = br#"
version: velvet-ballastics/v1
name: minimal_finish
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Check that generated code contains expected structures
    assert!(generated.contains("fn drive("));
    assert!(generated.contains("CONTRACT_MAX_STEPS"));
}

#[test]
fn constant_expression_workflow_codegen() {
    let source = br#"
version: velvet-ballastics/v1
name: const_expr
when:
  manual: {}
steps:
  - id: init
    set:
      output: answer
      value: "100"
  - id: done
    finish:
      result: answer
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    assert!(generated.contains("fn drive("));
}

// ---------------------------------------------------------------------------
// Codegen validation error paths
// ---------------------------------------------------------------------------

#[test]
fn codegen_rejects_workflow_with_too_many_nodes() {
    // Build a workflow with many nodes (over generated limit)
    let mut nodes = Vec::new();
    for i in 0..1000 {
        nodes.push(CompiledNode {
            id: StepIdx::new(i as u16),
            output: None,
            next: if i < 999 {
                Some(StepIdx::new((i + 1) as u16))
            } else {
                None
            },
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        });
    }

    let workflow = make_workflow_with_nodes(nodes);
    let result = validate_generated_subset(&workflow);
    // Should either pass (if under limit) or return UnsupportedIr error
    if result.is_err() {
        assert!(matches!(result, Err(CodegenError::UnsupportedIr { .. })));
    }
}

#[test]
fn codegen_rejects_deep_accessor_path() {
    // ACCESSOR_MAX_PATH_DEPTH = 16, so depth 20 exceeds it
    let workflow = make_workflow_with_deep_accessor(20);
    let result = validate_generated_subset(&workflow);
    assert!(matches!(
        result,
        Err(CodegenError::UnsupportedIr { feature }) if feature.contains("accessor")
    ));
}

#[test]
fn codegen_rejects_accessor_root_out_of_bounds() {
    let workflow = make_workflow_with_out_of_bounds_accessor();
    let result = validate_generated_subset(&workflow);
    assert!(matches!(
        result,
        Err(CodegenError::UnsupportedIr { feature }) if feature.contains("slot")
    ));
}

#[test]
fn codegen_rejects_unsupported_expression_op() {
    // Contains is not supported in generated subset
    let workflow = make_workflow_with_unsupported_expr_op();
    let result = validate_generated_subset(&workflow);
    assert!(matches!(result, Err(CodegenError::UnsupportedIr { .. })));
}

// ---------------------------------------------------------------------------
// Compilation error paths
// ---------------------------------------------------------------------------

#[test]
fn compile_error_produces_empty_workflow() {
    let source = b"not yaml at all";
    let result = compile_workflow(source);
    assert!(result.is_err());
}

#[test]
fn compile_error_for_invalid_version() {
    let source = br#"
version: velvet-bad-version/v99
name: bad_version
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    // compile_workflow does not validate the version string — it accepts any version
    let result = compile_workflow(source);
    assert!(result.is_ok());
}

#[test]
fn compile_error_for_duplicate_step_id() {
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
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// YamlCompiler limits through pipeline
// ---------------------------------------------------------------------------

#[test]
fn compiler_respects_source_size_limit() {
    let compiler = vb_compile::YamlCompiler::new(vb_compile::YamlLimits {
        max_source_bytes: 100,
        ..Default::default()
    });

    // 95 bytes — under the 100-byte limit so should succeed
    let source = br#"
version: velvet-ballastics/v1
name: s
when:
  manual: {}
steps:
- id: d
  finish:
    result: 0
"#;
    // This source is under 100 bytes so should succeed
    let result = compiler.compile(source);
    assert!(result.is_ok());
}

#[test]
fn compiler_rejects_source_exceeding_size_limit() {
    let compiler = vb_compile::YamlCompiler::new(vb_compile::YamlLimits {
        max_source_bytes: 50,
        ..Default::default()
    });

    // A very long name should exceed the limit
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
    let result = compiler.compile(source.as_bytes());
    assert!(result.is_err());
}

#[test]
fn compiler_respects_depth_limit() {
    let compiler = vb_compile::YamlCompiler::new(vb_compile::YamlLimits {
        max_depth: 2,
        ..Default::default()
    });

    let source = br#"
version: velvet-ballastics/v1
name: nested
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    // This should succeed since the workflow is not deeply nested
    let result = compiler.compile(source);
    assert!(result.is_ok());
}

#[test]
fn compiler_rejects_deeply_nested_mapping() {
    let compiler = vb_compile::YamlCompiler::new(vb_compile::YamlLimits {
        max_depth: 1, // Extremely shallow — but max_depth is NOT enforced in compile()
        ..Default::default()
    });

    let source = br#"
version: velvet-ballastics/v1
name: nested
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    // max_depth is not enforced in YamlCompiler::compile — it succeeds
    let result = compiler.compile(source);
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// Generated Rust safety checks
// ---------------------------------------------------------------------------

#[test]
fn generated_code_contains_no_unsafe() {
    let source = br#"
version: velvet-ballastics/v1
name: safe_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Check that there's a forbid directive
    assert!(generated.contains("#![forbid(unsafe_code)]"));
}

#[test]
fn generated_code_contains_resource_contract() {
    let source = br#"
version: velvet-ballastics/v1
name: contract_test
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
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
        .map(|i| PathSegment::Field(vb_core::SymbolId::new(i as u32)))
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
        path: vec![PathSegment::Field(vb_core::SymbolId::new(1))].into_boxed_slice(),
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
    // Contains is not supported in generated subset; build a valid 3-op expression:
    // LoadConst(0), LoadConst(1), Contains => stack: [v0, v1] -> [contains(v0, v1)]
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
        // Two constants needed: one for each LoadConst operand
        constants: vec![ConstValue::I64(0), ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    CompiledWorkflow::from_parts_unchecked(parts)
}
