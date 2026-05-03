use saphyr::Yaml;
use saphyr_parser::Parser;
use thiserror::Error;
use vb_core::{
    AccessorProgram, ActionContract, CompiledWorkflow, CompiledNode, CompiledNodeKind,
    ConstIdx, ConstValue, ExprProgram, ResourceContract, WorkflowDigest,
};
use crate::constants::{checked_utf8, single_document, SourceMark, YamlLimits, WORKFLOW_VERSION};
use crate::errors::CompileErrors;
use crate::yaml_parse::{reject_duplicate_mapping_keys, strict_yaml_reject_unsupported};
use crate::yaml_profile::strict_profile_check;
use crate::workflow_shape::validate_workflow_document_shape;
use crate::schema::validate_input_schemas;
use crate::ast::parse_workflow_ast;
use crate::references::validate_workflow_ast as validate_references;
use crate::type_taint::validate_workflow_ast as validate_taint;
use crate::control_flow::validate_workflow_ast as validate_control_flow;
use crate::workflow_build::{build_workflow_parts};
use crate::workflow_compile::{compile_step, WorkflowBuilder};
use std::str;


// ---------------------------------------------------------------------------
// Phase 11-12 public slot compiler and IR lowering API (section 28)
// ---------------------------------------------------------------------------

/// Top-level compilation entry point producing a validated compiled workflow.
///
/// Wraps [`YamlCompiler::compile`] with the default limits for ergonomic
/// programmatic use by downstream crates.
pub fn compile_workflow(source: &[u8]) -> Result<CompiledWorkflow, CompileErrors> {
    YamlCompiler::default().compile(source)
}

/// Compiles YAML source and then verifies action contracts against the
/// idempotency gate AND gate 12 (action contract completeness).
///
/// Performs the full compilation pipeline from [`compile_workflow`], then runs
/// gate 12 to verify that every Do node has a matching contract and every
/// contract has a matching Do node, and finally runs [`check_idempotency_gates`]
/// on the supplied action contracts. Returns the compiled workflow only when
/// all three checks pass. This is the recommended entry point for runtime
/// integrations that register action contracts before workflow deployment.
pub fn compile_workflow_with_contracts(
    source: &[u8],
    contracts: &[ActionContract],
) -> Result<CompiledWorkflow, CompileErrors> {
    let workflow = compile_workflow(source)?;
    let parts = workflow.to_parts();
    vb_validate::shared::validate_with_contracts(&parts, contracts)
        .map_err(|e| CompileErrors(vec![e.into()]))?;
    check_idempotency_gates(contracts)?;
    Ok(workflow)
}

/// Builds a slot layout from workflow parts.
///
/// Returns the number of slots needed by the compiled workflow frame.
/// The slot layout is derived from the maximum slot index referenced
/// across all compiled nodes.
pub fn build_slot_layout(parts: &WorkflowParts) -> u16 {
    parts.slot_count
}

/// Builds the accessor table from workflow parts.
///
/// Returns a reference to the accessor programs table for slot-rooted
/// path traversal.
pub fn build_accessor_table(parts: &WorkflowParts) -> &[AccessorProgram] {
    &parts.accessors
}

/// Builds the constant pool from workflow parts.
///
/// Returns a reference to the constant pool containing all literal values
/// referenced by compiled nodes and expression programs.
pub fn build_constant_pool(parts: &WorkflowParts) -> &[ConstValue] {
    &parts.constants
}

/// Lowers a flat list of compiled nodes into the final IR representation.
///
/// This is the primary lowering step that converts step-level IR into the
/// compiled node array used by the hot runtime.
#[allow(clippy::too_many_arguments)]
pub fn lower_steps_to_ir(
    nodes: Vec<CompiledNode>,
    expressions: Vec<ExprProgram>,
    accessors: Vec<AccessorProgram>,
    constants: Vec<ConstValue>,
    slot_count: u16,
    symbols_count: u32,
    name: &str,
    digest: WorkflowDigest,
) -> Result<CompiledWorkflow, CompileErrors> {
    let parts = WorkflowParts {
        name: Box::from(name),
        digest,
        nodes: nodes.into_boxed_slice(),
        expressions: expressions.into_boxed_slice(),
        accessors: accessors.into_boxed_slice(),
        constants: constants.into_boxed_slice(),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))
}

