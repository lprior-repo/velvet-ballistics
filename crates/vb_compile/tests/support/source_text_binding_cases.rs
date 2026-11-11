#[path = "source_text_binding_fixtures.rs"]
mod fixtures;

use fixtures::*;
use vb_compile::{
    ArtifactVerificationError, CompilationBinding, CompileError, CompileErrors, compile_workflow,
    encode_compiled_ir, verify_compiled_artifact,
};
use vb_core::workflow::source_metadata::SourceMetadataError;
use vb_core::workflow::{VersionedWorkflowParts, WorkflowIrV2};
use vb_core::{CompiledWorkflow, ConstValue, SymbolId, WorkflowDigest, WorkflowError};

#[test]
fn present_literal_has_runs_after_real_input_symbol_remapping() -> TestResult {
    execute_has(
        "key",
        &["key".into(), "unused".into(), "input-only".into()],
        SymbolId::new(0),
        true,
    )
}

#[test]
fn missing_literal_has_runs_after_real_input_symbol_remapping() -> TestResult {
    execute_has(
        "missing",
        &[
            "missing".into(),
            "unused".into(),
            "key".into(),
            "input-only".into(),
        ],
        SymbolId::new(2),
        false,
    )
}

#[test]
fn public_declarations_and_steps_share_first_definition_ids() -> TestResult {
    let workflow = compile_workflow(declarations_source().as_bytes())?;
    assert_eq!(
        workflow.symbol_payloads(),
        Some(["z".into(), "a".into()].as_slice())
    );
    assert_eq!(
        workflow.to_parts().constants.as_ref(),
        &[
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(1)),
            ConstValue::Symbol(SymbolId::new(1)),
            ConstValue::Symbol(SymbolId::new(0)),
        ]
    );
    Ok(())
}

#[test]
fn public_nested_bodies_share_the_compiler_prefix_depth_first() -> TestResult {
    let workflow = compile_workflow(nested_source().as_bytes())?;
    assert_eq!(
        workflow.symbol_payloads(),
        Some(["decl".into(), "outer".into(), "inner".into(), "last".into(),].as_slice())
    );
    assert_eq!(
        symbol_constants(&workflow),
        vec![
            SymbolId::new(0),
            SymbolId::new(1),
            SymbolId::new(2),
            SymbolId::new(1),
            SymbolId::new(3)
        ]
    );
    Ok(())
}

fn symbol_constants(workflow: &CompiledWorkflow) -> Vec<SymbolId> {
    workflow
        .to_parts()
        .constants
        .iter()
        .filter_map(|value| match value {
            ConstValue::Symbol(id) => Some(*id),
            _ => None,
        })
        .collect()
}

#[test]
fn public_utf8_and_whitespace_payloads_keep_exact_bytes() -> TestResult {
    let workflow = compile_expression("(\"é\" == \"é\") and (\" key\" == \"key \")")?;
    assert_eq!(
        workflow.symbol_payloads(),
        Some(["é".into(), "é".into(), " key".into(), "key ".into(),].as_slice())
    );
    assert_eq!(workflow.symbols_count(), 4);
    Ok(())
}

#[test]
fn public_empty_payload_is_bound_even_with_zero_payload_bytes() -> TestResult {
    let workflow = compile_expression("\"\"")?;
    assert_eq!(workflow.symbol_payloads(), Some(["".into()].as_slice()));
    assert_eq!(workflow.symbols_count(), 1);
    assert!(matches!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V2(_)
    ));
    Ok(())
}

#[test]
fn public_text_free_compilation_keeps_v1_raw_bytes_and_digest() -> TestResult {
    let workflow = compile_expression("1 + 2")?;
    assert_eq!(workflow.symbol_payloads(), None);
    assert!(matches!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V1(_)
    ));
    let mut parts = workflow.to_parts();
    assert_eq!(
        encode_compiled_ir(&workflow)?.as_ref(),
        postcard::to_allocvec(&parts)?
    );
    parts.digest = WorkflowDigest::from_bytes([0; 32]);
    let expected = WorkflowDigest::from_bytes(blake3::hash(&postcard::to_allocvec(&parts)?).into());
    assert_eq!(workflow.digest(), expected);
    Ok(())
}

#[test]
fn public_literal_digest_binds_payload_bytes_with_fixed_parts() -> TestResult {
    let workflow = compile_expression("has($input.obj, \"key\")")?;
    let mut ir = WorkflowIrV2 {
        wire_version: 2,
        parts: workflow.to_parts(),
        symbol_payloads: Box::from(["key".into()]),
    };
    assert_eq!(workflow.digest(), independent_v2_digest(ir.clone())?);
    ir.symbol_payloads = Box::from(["kez".into()]);
    let expected = independent_v2_digest(ir.clone())?;
    assert_ne!(workflow.digest(), expected);
    let changed = CompiledWorkflow::try_from_parts_with_symbols(ir.parts, ir.symbol_payloads)?;
    assert_stale_payload(&changed, workflow.digest(), expected)?;
    Ok(())
}

fn assert_stale_payload(
    workflow: &CompiledWorkflow,
    expected_claimed: WorkflowDigest,
    expected_computed: WorkflowDigest,
) -> TestResult {
    match verify_compiled_artifact(workflow, &[]) {
        Err(ArtifactVerificationError::DigestMismatch {
            binding,
            claimed,
            computed,
        }) => {
            assert_eq!(
                (binding, claimed, computed),
                (
                    CompilationBinding::Workflow,
                    expected_claimed,
                    expected_computed
                )
            );
            Ok(())
        }
        other => Err(format!("wrong digest result: {other:?}").into()),
    }
}

#[test]
fn public_text_variable_accepts_exact_symbol_byte_limit() -> TestResult {
    let payload = "x".repeat(4096);
    let workflow = compile_variable(&payload)?;
    assert_eq!(
        workflow.symbol_payloads(),
        Some([payload.into_boxed_str()].as_slice())
    );
    assert_eq!(workflow.symbols_count(), 1);
    Ok(())
}

#[test]
fn public_overlong_text_variable_retains_exact_metadata_error() {
    assert_eq!(
        compile_variable(&"x".repeat(4097)),
        Err(CompileErrors(vec![CompileError::Workflow(
            WorkflowError::InvalidSymbolMetadata {
                source: SourceMetadataError::SymbolTooLong {
                    id: SymbolId::new(0),
                    bytes: 4097,
                    max: 4096
                },
            }
        ),]))
    );
}
