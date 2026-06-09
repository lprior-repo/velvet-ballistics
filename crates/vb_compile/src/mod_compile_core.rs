//! Private compile facade/orchestration module.

pub use crate::limits::YamlLimits;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_validation::{
    canonical_yaml_error, checked_utf8, reject_duplicate_mapping_keys,
    reject_known_canonical_text_gaps, single_document, validate_strict_profile,
    validate_workflow_document_shape,
};
use crate::{ast, control_flow, references, schema, strict_yaml, type_taint};
use saphyr::{LoadableYamlNode, Yaml};
use vb_core::{
    AccessorProgram, ActionContract, CompiledWorkflow, ConstValue, Idempotency, RetrySafety,
    SideEffect, WorkflowDigest, WorkflowParts,
};

/// Cold compiler facade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlCompiler {
    limits: YamlLimits,
}
impl YamlCompiler {
    /// Creates a compiler with explicit strict-profile limits.
    #[must_use]
    pub const fn new(limits: YamlLimits) -> Self {
        Self { limits }
    }

    /// Parses and validates YAML, then emits compiled workflow IR.
    pub fn compile(&self, source: &[u8]) -> Result<CompiledWorkflow, CompileErrors> {
        let text = checked_utf8(source, self.limits).map_err(|e| CompileErrors(vec![e]))?;
        reject_known_canonical_text_gaps(text).map_err(|e| CompileErrors(vec![e]))?;
        let source = vb_yaml::parse_workflow_source(text)
            .map_err(|e| CompileErrors(vec![canonical_yaml_error(e)]))?;
        crate::mod_compile_lowering::compile_source(&source)
    }

    /// Parses strict YAML into the cold typed AST without emitting runtime IR.
    pub fn parse_ast(&self, source: &[u8]) -> Result<ast::WorkflowAst, CompileErrors> {
        let text = checked_utf8(source, self.limits).map_err(|e| CompileErrors(vec![e]))?;
        strict_yaml::reject_unsupported_profile_events(text).map_err(|e| CompileErrors(vec![e]))?;
        reject_duplicate_mapping_keys(text).map_err(|e| CompileErrors(vec![e]))?;
        let docs =
            Yaml::load_from_str(text).map_err(|e| CompileErrors(vec![CompileError::Parse(e)]))?;
        let doc = single_document(&docs).map_err(|e| CompileErrors(vec![e]))?;
        validate_strict_profile(doc, self.limits).map_err(|e| CompileErrors(vec![e]))?;
        validate_workflow_document_shape(doc).map_err(|e| CompileErrors(vec![e]))?;
        schema::validate_input_schemas(doc)?;
        let ast = ast::parse_workflow_ast(text, doc).map_err(|e| CompileErrors(vec![e]))?;
        references::validate_workflow_ast(&ast)?;
        type_taint::validate_workflow_ast(&ast)?;
        control_flow::validate_workflow_ast(&ast)?;
        Ok(ast)
    }
}
impl Default for YamlCompiler {
    fn default() -> Self {
        Self::new(YamlLimits::default())
    }
}
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
/// Computes the blake3 digest of a compiled workflow artifact.
pub fn compute_compiled_digest(source: &[u8]) -> WorkflowDigest {
    WorkflowDigest::from_bytes(blake3::hash(source).into())
}

/// Emits a postcard-serialized compiled workflow artifact.
///
/// The serialized artifact can be loaded by the hot runtime without
/// re-parsing YAML source.
pub fn emit_compiled_artifact(workflow: &CompiledWorkflow) -> Result<Box<[u8]>, CompileErrors> {
    let parts = workflow.to_parts();
    postcard::to_allocvec(&parts)
        .map(std::vec::Vec::into_boxed_slice)
        .map_err(|error| {
            CompileErrors(vec![CompileError::ExpressionLoweringUnsupported {
                feature: format!("postcard serialization failed: {error}").into_boxed_str(),
            }])
        })
}

/// Validates that all action contracts satisfy idempotency safety requirements.
///
/// Rejects any action whose static contract declares side effects combined with
/// retry-unsafe or non-idempotent semantics. This gate runs at compile time so
/// that workflows with unsafe action configurations are rejected before deployment.
///
/// Rules:
/// - `SideEffect::Pure` always passes (pure computation).
/// - `side_effect != None` AND `RetrySafety::Unsafe` is rejected.
/// - `side_effect != None` AND `Idempotency::AtLeastOnceExternal` is rejected.
/// - `side_effect != None` AND `Idempotency::DeterministicPure` is rejected.
/// - `side_effect != None` AND `RetrySafety::Safe` with `Idempotency::IdempotentExternal` passes.
/// - `side_effect != None` AND `RetrySafety::KeyRequired` with `Idempotency::IdempotentExternal` passes.
pub fn is_compile_idempotency_gate_accepted(contract: &ActionContract) -> bool {
    matches!(
        (
            contract.side_effect,
            contract.retry_safety,
            contract.idempotency,
        ),
        (SideEffect::Pure, _, _)
            | (
                _,
                RetrySafety::Safe | RetrySafety::KeyRequired,
                Idempotency::IdempotentExternal,
            )
    )
}

pub fn check_idempotency_gates(contracts: &[ActionContract]) -> Result<(), CompileErrors> {
    let mut errors = Vec::new();
    let mut i = 0;
    while i < contracts.len() {
        let Some(contract) = contracts.get(i) else {
            break;
        };
        if is_compile_idempotency_gate_accepted(contract) {
            i = match i.checked_add(1) {
                Some(next) => next,
                None => break,
            };
            continue;
        }
        if contract.retry_safety == RetrySafety::Unsafe {
            errors.push(CompileError::IdempotencyViolation {
                action: contract.id,
                side_effect: contract.side_effect,
                reason: Box::from("side-effecting action declares RetrySafety::Unsafe"),
            });
            i = match i.checked_add(1) {
                Some(next) => next,
                None => break,
            };
            continue;
        }
        if contract.idempotency == Idempotency::AtLeastOnceExternal {
            errors.push(CompileError::IdempotencyViolation {
                action: contract.id,
                side_effect: contract.side_effect,
                reason: Box::from(
                    "side-effecting action declares Idempotency::AtLeastOnceExternal \
                     without guaranteed idempotent retry",
                ),
            });
        }
        if contract.idempotency == Idempotency::DeterministicPure {
            errors.push(CompileError::IdempotencyViolation {
                action: contract.id,
                side_effect: contract.side_effect,
                reason: Box::from("side-effecting action declares Idempotency::DeterministicPure"),
            });
        }
        i = match i.checked_add(1) {
            Some(next) => next,
            None => break,
        };
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}
