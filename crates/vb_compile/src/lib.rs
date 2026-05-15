#![forbid(unsafe_code)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::return_self_not_must_use)]
//! Cold-path YAML compiler boundary.
//!
//! YAML enters the system only through this crate. The hot engine consumes only
//! `vb_core::CompiledWorkflow` values built from native Rust `saphyr` parsing.

// NOTE: Validation deduplication with `vb_validate` (DRIFT-5)
// -----------------------------------------------
// Reference validation is shared: this crate builds a `RefTables` from its AST
// and calls `vb_validate::references::validate_single_reference` for each
// reference, avoiding duplicate validation logic.
//
// Control-flow and type/taint validation remain compile-local because they
// need structured step/target indices and AST-specific type inference that the
// standalone validator's string-based error model cannot represent. These
// modules perform the same *logical* checks as `vb_validate` but on different
// input types.

pub mod ast;
mod control_flow;
pub mod expression;
mod expression_bytecode;
mod references;
mod schema;
pub mod strict_yaml;
mod type_taint;

// Kani harnesses for idempotency gate parity verification (State 5 proof-writer).
#[cfg(kani)]
pub mod kani_idempotency_parity;

pub use expression_bytecode::{compile_expr_to_bytecode, compile_expr_to_bytecode_with_accessors};

// Re-export the shared validation error types from `vb_validate` so that
// downstream consumers of this crate can optionally use the standalone
// validator's error domain without depending on `vb_validate` directly.
pub use vb_validate::{ValidationError, ValidationResult};

use saphyr::{LoadableYamlNode, Yaml};
use saphyr_parser::{Event, Parser, Span, StrInput};
use std::collections::{HashMap, HashSet};
use std::str;
use thiserror::Error;
use vb_core::{
    AccessorProgram, ActionContract, ActionId, CompiledNode, CompiledNodeKind, CompiledWorkflow,
    ConstIdx, ConstValue, ExprIdx, ExprProgram, Idempotency, ResourceContract, RetrySafety,
    SideEffect, SlotBranch, SlotIdx, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};

const DEFAULT_MAX_SOURCE_BYTES: usize = 1_048_576;
const DEFAULT_MAX_DEPTH: u16 = 64;
const DEFAULT_MAX_NODES: u32 = 100_000;
const DEFAULT_MAX_SEQUENCE_LEN: usize = 10_000;
const DEFAULT_MAX_MAPPING_ENTRIES: usize = 1_024;
const DEFAULT_MAX_SCALAR_BYTES: usize = 65_536;
const WORKFLOW_VERSION: &str = "velvet-ballastics/v1";

/// Strict YAML resource limits for cold compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlLimits {
    /// Maximum workflow source size in bytes.
    pub max_source_bytes: usize,
    /// Maximum YAML nesting depth.
    pub max_depth: u16,
    /// Maximum total YAML nodes visited by validation.
    pub max_nodes: u32,
    /// Maximum sequence length.
    pub max_sequence_len: usize,
    /// Maximum mapping entry count.
    pub max_mapping_entries: usize,
    /// Maximum UTF-8 scalar length in bytes.
    pub max_scalar_bytes: usize,
}

impl Default for YamlLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            max_depth: DEFAULT_MAX_DEPTH,
            max_nodes: DEFAULT_MAX_NODES,
            max_sequence_len: DEFAULT_MAX_SEQUENCE_LEN,
            max_mapping_entries: DEFAULT_MAX_MAPPING_ENTRIES,
            max_scalar_bytes: DEFAULT_MAX_SCALAR_BYTES,
        }
    }
}

/// Cold compiler facade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlCompiler {
    limits: YamlLimits,
}

/// Source location exposed by `saphyr-parser`.
///
/// `index` is the parser-provided byte offset into the UTF-8 source. `line` and
/// `column` are one-indexed parser marks. Tree-only validation paths use an
/// unavailable mark because `saphyr::Yaml` nodes do not retain marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceMark {
    /// Parser-provided byte offset.
    pub index: usize,
    /// Parser-provided exclusive byte offset where the event span ends.
    pub end_index: usize,
    /// One-indexed source line.
    pub line: usize,
    /// One-indexed source column.
    pub column: usize,
    /// Whether this mark came from `saphyr-parser` event data.
    pub available: bool,
}

impl SourceMark {
    #[must_use]
    pub(crate) fn from_parser_span(span: Span) -> Self {
        Self {
            index: span.start.index(),
            end_index: span.end.index(),
            line: span.start.line(),
            column: span.start.col(),
            available: true,
        }
    }

    #[must_use]
    pub(crate) const fn unavailable() -> Self {
        Self {
            index: 0,
            end_index: 0,
            line: 0,
            column: 0,
            available: false,
        }
    }
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
        let source = vb_yaml::parse_workflow_source(text)
            .map_err(|e| CompileErrors(vec![canonical_yaml_error(e)]))?;
        compile_source(&source)
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

fn canonical_yaml_error(error: vb_yaml::YamlError) -> CompileError {
    CompileError::CanonicalYaml {
        category: yaml_error_category(&error),
        message: error.to_string().into_boxed_str(),
    }
}

fn yaml_error_category(error: &vb_yaml::YamlError) -> &'static str {
    match error {
        vb_yaml::YamlError::UnsupportedFeature { .. }
        | vb_yaml::YamlError::AnchorAliasMerge
        | vb_yaml::YamlError::CustomTag { .. }
        | vb_yaml::YamlError::BinaryScalar
        | vb_yaml::YamlError::AmbiguousScalar { .. }
        | vb_yaml::YamlError::ForbiddenFeature { .. } => "forbidden_feature",
        vb_yaml::YamlError::DuplicateKey { .. } => "duplicate_key",
        vb_yaml::YamlError::MultipleDocuments { .. } => "document_count",
        vb_yaml::YamlError::SourceTooLarge { .. }
        | vb_yaml::YamlError::NestingTooDeep { .. }
        | vb_yaml::YamlError::NodeLimitExceeded { .. }
        | vb_yaml::YamlError::ScalarTooLong { .. }
        | vb_yaml::YamlError::SequenceTooLong { .. }
        | vb_yaml::YamlError::MappingTooLarge { .. } => "limit_exceeded",
        vb_yaml::YamlError::UnknownField { .. } => "unknown_field",
        vb_yaml::YamlError::EmptySource => "empty_source",
        vb_yaml::YamlError::MissingField { .. } => "missing_field",
        vb_yaml::YamlError::FieldShape { .. } => "field_shape",
        vb_yaml::YamlError::ParseError { .. } => "parse_error",
    }
}

impl Default for YamlCompiler {
    fn default() -> Self {
        Self::new(YamlLimits::default())
    }
}

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

/// Compile the canonical cold YAML authoring AST into numeric runtime IR.
pub fn compile_source(
    source: &vb_yaml::ast::WorkflowSource,
) -> Result<CompiledWorkflow, CompileErrors> {
    validate_canonical_compile_scope(source)?;
    let mut builder = SlotCompiler::new();
    let mut outputs: HashMap<&str, SlotIdx> = HashMap::new();
    let steps = source.steps();
    let last = steps
        .len()
        .checked_sub(1)
        .ok_or(CompileErrors(vec![CompileError::EmptySteps]))?;
    let mut step_names: Vec<Box<str>> = Vec::with_capacity(steps.len());
    for step in steps {
        step_names.push(Box::from(step.id.as_str()));
    }
    for (index, step) in steps.iter().enumerate() {
        let id = step_idx(index).map_err(|e| CompileErrors(vec![e]))?;
        let next = if index == last {
            None
        } else {
            Some(
                step_idx(index.checked_add(1).ok_or_else(|| {
                    CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }])
                })?)
                .map_err(|e| CompileErrors(vec![e]))?,
            )
        };
        match &step.primitive {
            vb_yaml::ast::StepPrimitive::Set { output, value } => {
                if outputs.contains_key(output.as_str()) {
                    return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
                        name: output.clone().into_boxed_str(),
                    }]));
                }
                let parsed = value.parse::<i64>().map_err(|_| {
                    CompileErrors(vec![CompileError::StepFieldShape {
                        step: index,
                        field: "set.value",
                        expected: "integer string",
                    }])
                })?;
                let const_idx = builder
                    .push_constant(ConstValue::I64(parsed))
                    .map_err(|e| CompileErrors(vec![e]))?;
                let slot = slot_idx_for_step(index).map_err(|e| CompileErrors(vec![e]))?;
                outputs.insert(output.as_str(), slot);
                builder.push_node(lower_set(id, slot, const_idx, next));
            }
            vb_yaml::ast::StepPrimitive::Finish { result } => {
                if index != last {
                    return Err(CompileErrors(vec![CompileError::StepFieldShape {
                        step: index,
                        field: "finish",
                        expected: "the last step",
                    }]));
                }
                let slot = canonical_finish_slot(result, &outputs)?;
                let node = lower_finish(id, slot, &mut builder);
                builder.push_node(node);
            }
            other => {
                return Err(CompileErrors(vec![
                    CompileError::UnsupportedStepPrimitive {
                        step: index,
                        primitive: canonical_primitive_name(other),
                    },
                ]));
            }
        }
    }
    let parts = WorkflowParts {
        name: Box::from(source.name()),
        digest: canonical_digest(source),
        slot_count: builder.slot_count().map_err(|e| CompileErrors(vec![e]))?,
        symbols_count: 0,
        nodes: builder.nodes.into_boxed_slice(),
        expressions: builder.expressions.into_boxed_slice(),
        accessors: builder.accessors.into_boxed_slice(),
        constants: builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: step_names.into_boxed_slice(),
    };
    vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))?;
    CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))
}

fn validate_canonical_compile_scope(
    source: &vb_yaml::ast::WorkflowSource,
) -> Result<(), CompileErrors> {
    let mut errors = Vec::new();
    if !source.inputs().is_empty() {
        errors.push(CompileError::UnsupportedTopLevelDeclaration { field: "inputs" });
    }
    if !source.vars().is_empty() {
        errors.push(CompileError::UnsupportedTopLevelDeclaration { field: "vars" });
    }
    if !source.secrets().is_empty() {
        errors.push(CompileError::UnsupportedTopLevelDeclaration { field: "secrets" });
    }
    if !source.examples().is_empty() {
        errors.push(CompileError::UnsupportedTopLevelDeclaration { field: "examples" });
    }
    if source.result().is_some() {
        errors.push(CompileError::UnsupportedTopLevelResult);
    }
    let mut step_ids = HashSet::with_capacity(source.steps().len());
    for (index, step) in source.steps().iter().enumerate() {
        if !step_ids.insert(step.id.as_str()) {
            errors.push(CompileError::DuplicateStepId {
                id: Box::from(step.id.as_str()),
            });
        }
        if step.name.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("name"),
            });
        }
        if step.condition.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("if"),
            });
        }
        if step.with.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("with"),
            });
        }
        if step.retry.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("try_again"),
            });
        }
        if step.on_error.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("on_error"),
            });
        }
        if step.then.is_some() {
            errors.push(CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from("then"),
            });
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}

fn canonical_finish_slot(
    result: &vb_yaml::ast::ScalarValue,
    outputs: &HashMap<&str, SlotIdx>,
) -> Result<SlotIdx, CompileErrors> {
    match result {
        vb_yaml::ast::ScalarValue::String(name) => {
            outputs.get(name.as_str()).copied().ok_or_else(|| {
                CompileErrors(vec![CompileError::UnknownOutputName {
                    name: name.clone().into_boxed_str(),
                }])
            })
        }
        vb_yaml::ast::ScalarValue::Integer(value) => {
            let raw = u16::try_from(*value).map_err(|_| {
                CompileErrors(vec![CompileError::SlotIndexOutOfRange { value: *value }])
            })?;
            Ok(SlotIdx::new(raw))
        }
    }
}

fn canonical_primitive_name(primitive: &vb_yaml::ast::StepPrimitive) -> &'static str {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { .. } => "set",
        vb_yaml::ast::StepPrimitive::Save { .. } => "save",
        vb_yaml::ast::StepPrimitive::Do { .. } => "do",
        vb_yaml::ast::StepPrimitive::Choose { .. } => "choose",
        vb_yaml::ast::StepPrimitive::ForEach { .. } => "for_each",
        vb_yaml::ast::StepPrimitive::Together { .. } => "together",
        vb_yaml::ast::StepPrimitive::Collect { .. } => "collect",
        vb_yaml::ast::StepPrimitive::Reduce { .. } => "reduce",
        vb_yaml::ast::StepPrimitive::Repeat { .. } => "repeat",
        vb_yaml::ast::StepPrimitive::Wait { .. } => "wait",
        vb_yaml::ast::StepPrimitive::Ask { .. } => "ask",
        vb_yaml::ast::StepPrimitive::Finish { .. } => "finish",
    }
}

fn canonical_digest(source: &vb_yaml::ast::WorkflowSource) -> WorkflowDigest {
    let mut hasher = blake3::Hasher::new();
    hasher.update(source.version().as_bytes());
    hasher.update(source.name().as_bytes());
    match source.trigger() {
        vb_yaml::ast::TriggerAst::Manual => hasher.update(b"manual"),
        vb_yaml::ast::TriggerAst::Schedule { cron } => {
            hasher.update(b"schedule");
            hasher.update(cron.as_bytes())
        }
        vb_yaml::ast::TriggerAst::Event { event_type } => {
            hasher.update(b"event");
            hasher.update(event_type.as_bytes())
        }
        vb_yaml::ast::TriggerAst::Webhook => hasher.update(b"webhook"),
    };
    for step in source.steps() {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(&mut hasher, &step.primitive);
    }
    WorkflowDigest::from_bytes(hasher.finalize().into())
}

fn digest_step_primitive(hasher: &mut blake3::Hasher, primitive: &vb_yaml::ast::StepPrimitive) {
    match primitive {
        vb_yaml::ast::StepPrimitive::Set { output, value } => {
            hasher.update(b"set");
            hasher.update(output.as_bytes());
            hasher.update(value.as_bytes());
        }
        vb_yaml::ast::StepPrimitive::Finish { result } => {
            hasher.update(b"finish");
            match result {
                vb_yaml::ast::ScalarValue::String(value) => hasher.update(value.as_bytes()),
                vb_yaml::ast::ScalarValue::Integer(value) => hasher.update(&value.to_le_bytes()),
            };
        }
        other => {
            hasher.update(canonical_primitive_name(other).as_bytes());
        }
    }
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
        step_names: Box::new([]),
    };
    vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))?;
    CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))
}

/// Lowers a `set` (save) primitive into a `SetConst` or `Copy` node.
pub fn lower_set(
    id: StepIdx,
    output: SlotIdx,
    value: ConstIdx,
    next: Option<StepIdx>,
) -> CompiledNode {
    CompiledNode {
        id,
        output: Some(output),
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::SetConst { value },
    }
}

/// Lowers a `do` (action) primitive into a `Do` node.
pub fn lower_do(
    id: StepIdx,
    action: vb_core::ActionId,
    input: SlotIdx,
    output: Option<SlotIdx>,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> CompiledNode {
    builder.record_slot(input);
    CompiledNode {
        id,
        output,
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::Do { action, input },
    }
}

/// Lowers a `choose` primitive into a `ChooseSlot` node.
///
/// Follows the critical choose lowering rule: conditions are
/// pre-materialized boolean slots, not raw YAML condition strings.
pub fn lower_choose(
    id: StepIdx,
    branches: Vec<SlotBranch>,
    otherwise: Option<StepIdx>,
    builder: &mut SlotCompiler,
) -> Result<CompiledNode, CompileError> {
    for branch in &branches {
        builder.record_slot(branch.condition);
    }
    let branches = branches.into_boxed_slice();
    validate_branch_route(&branches, otherwise)?;
    Ok(CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        },
    })
}

/// Lowers a `for_each` primitive into `ForEachStart`, body, and `ForEachJoin` nodes.
pub fn lower_for_each(
    id: StepIdx,
    input: SlotIdx,
    item_slot: SlotIdx,
    limit: u32,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    builder.record_slot(input);
    builder.record_slot(item_slot);
    let iterator_slot = item_slot;
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::ForEachStart {
                input,
                item_slot,
                limit,
                body,
                done,
            },
        },
        CompiledNode {
            id: body,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::ForEachNext {
                iterator_slot,
                body,
                done,
            },
        },
    ])
}

/// Lowers a `together` (parallel) primitive into `TogetherStart`, branch, and `TogetherJoin` nodes.
pub fn lower_together(
    id: StepIdx,
    branches: Vec<StepIdx>,
    join: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    let branch_count = u16::try_from(branches.len()).map_err(|_| {
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "together",
            field: "branches",
            value: branches.len(),
            limit: usize::from(u16::MAX),
        }
    })?;
    let accumulator = alloc_accumulator_slot(builder)?;
    let mut nodes = vec![CompiledNode {
        id,
        output: Some(accumulator),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::TogetherStart {
            branches: branches.into_boxed_slice(),
            join,
        },
    }];
    nodes.push(CompiledNode {
        id: join,
        output: Some(accumulator),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        },
    });
    Ok(nodes)
}

/// Allocates a fresh accumulator slot for the together primitive.
fn alloc_accumulator_slot(builder: &mut SlotCompiler) -> Result<SlotIdx, CompileError> {
    let next = builder.slot_count()?;
    let slot = SlotIdx::new(next);
    builder.record_slot(slot);
    Ok(slot)
}

/// Lowers a `collect` (gather) primitive into collection IR nodes.
pub fn lower_collect(
    id: StepIdx,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    builder.record_slot(source);
    let collector_slot = source;
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::CollectStart {
                source,
                limit,
                page_size,
                body,
                done,
            },
        },
        CompiledNode {
            id: body,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::CollectPage {
                collector_slot,
                body,
                done,
            },
        },
        CompiledNode {
            id: done,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::CollectFinish { collector_slot },
        },
    ])
}

/// Lowers a `reduce` (summarize) primitive into reduction IR nodes.
pub fn lower_reduce(
    id: StepIdx,
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    builder.record_slot(input);
    builder.record_slot(accumulator);
    let iterator_slot = accumulator;
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::ReduceStart {
                input,
                accumulator,
                initial,
                body,
                done,
            },
        },
        CompiledNode {
            id: body,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::ReduceNext {
                iterator_slot,
                accumulator,
                body,
                done,
            },
        },
        CompiledNode {
            id: done,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::ReduceFinish { accumulator },
        },
    ])
}

/// Lowers a `repeat` primitive into retry IR nodes.
pub fn lower_repeat(
    id: StepIdx,
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    let attempt_slot = slot_idx_for_step(
        id.as_usize()
            .checked_add(1)
            .ok_or(CompileError::SlotIndexOutOfRange { value: i64::MAX })?,
    )?;
    builder.record_slot(attempt_slot);
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts,
                body,
                done,
            },
        },
        CompiledNode {
            id: body,
            output: Some(attempt_slot),
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::RepeatAttempt {
                attempt_slot,
                body,
                done,
            },
        },
        CompiledNode {
            id: done,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::RepeatFinish {
                result: attempt_slot,
            },
        },
    ])
}

/// Type-safe discriminator for the two legal `wait` shapes.
///
/// Replaces the previous `is_event: bool` parameter, which allowed invalid
/// combinations such as passing `is_event = false` with a `timeout_slot`,
/// which would be silently discarded.
pub enum WaitKind {
    /// `wait.until` — waits until a deadline slot is reached; no timeout.
    Until { deadline: SlotIdx },
    /// `wait.event` — waits for an event slot, with an optional timeout.
    Event {
        event: SlotIdx,
        timeout: Option<SlotIdx>,
    },
}

/// Lowers a `wait` primitive into `WaitUntil` or `WaitEvent` IR nodes.
pub fn lower_wait(id: StepIdx, kind: WaitKind, builder: &mut SlotCompiler) -> CompiledNode {
    let compiled_kind = match kind {
        WaitKind::Until { deadline } => {
            builder.record_slot(deadline);
            CompiledNodeKind::WaitUntil {
                deadline_slot: deadline,
            }
        }
        WaitKind::Event { event, timeout } => {
            builder.record_slot(event);
            if let Some(slot) = timeout {
                builder.record_slot(slot);
            }
            CompiledNodeKind::WaitEvent {
                event,
                timeout_slot: timeout,
            }
        }
    };
    CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: compiled_kind,
    }
}

/// Lowers an `ask` primitive into `Ask` and `AskResume` IR nodes.
pub fn lower_ask(
    id: StepIdx,
    prompt: SlotIdx,
    answer: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    builder: &mut SlotCompiler,
) -> Result<Vec<CompiledNode>, CompileError> {
    builder.record_slot(prompt);
    builder.record_slot(answer);
    let resume = id
        .checked_add(1)
        .ok_or(CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "ask",
            field: "resume_step",
            value: id.as_usize(),
            limit: usize::from(u16::MAX),
        })?;
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Ask {
                prompt,
                timeout_slot,
            },
        },
        CompiledNode {
            id: resume,
            output: Some(answer),
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::AskResume { answer },
        },
    ])
}

/// Lowers a `finish` primitive into a terminal `Finish` node.
pub fn lower_finish(id: StepIdx, result: SlotIdx, builder: &mut SlotCompiler) -> CompiledNode {
    builder.record_slot(result);
    CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::Finish { result },
    }
}

/// Validates compiled workflow IR against structural and resource invariants.
///
/// Runs the shared validation pipeline (gates 7-15) via
/// [`vb_validate::shared::validate`], then delegates to
/// [`CompiledWorkflow::try_from_parts`] for core structural and budget checks.
///
/// Returns the specific validation error so callers can distinguish gate
/// failures from structural errors.
pub fn validate_ir(parts: WorkflowParts) -> Result<CompiledWorkflow, CompileErrors> {
    vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))?;
    CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))
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

/// Generates a Rust source file from a compiled workflow.
///
/// The generated Rust backend is a supported subset, not a catch-all lowering
/// path for every valid [`CompiledWorkflow`]. Unsupported IR is rejected by
/// `vb_codegen` before source emission and is surfaced here as a compile error,
/// so callers can fall back to the interpreter/runtime path without compiling
/// partial generated Rust.
pub fn compile_to_generated_rust(workflow: &CompiledWorkflow) -> Result<String, CompileErrors> {
    vb_codegen::emit_rust_workflow(workflow).map_err(|error| {
        CompileErrors(vec![CompileError::ExpressionLoweringUnsupported {
            feature: error.to_string().into_boxed_str(),
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
/// - `SideEffect::None` always passes (pure computation).
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
        (SideEffect::None, _, _)
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

/// Mutable slot compiler state for building node arrays.
///
/// Tracks slot allocation, constant pool, expression programs, and accessor
/// programs during step lowering.
#[derive(Debug, Default)]
pub struct SlotCompiler {
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
    accessors: Vec<AccessorProgram>,
    max_slot: Option<usize>,
}

impl SlotCompiler {
    /// Creates a new empty slot compiler.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes a constant value and returns its index.
    pub fn push_constant(&mut self, value: ConstValue) -> Result<ConstIdx, CompileError> {
        let index = u16::try_from(self.constants.len()).map_err(|_| {
            CompileError::Workflow(WorkflowError::ConstOutOfBounds {
                constant: ConstIdx::new(u16::MAX),
            })
        })?;
        self.constants.push(value);
        Ok(ConstIdx::new(index))
    }

    /// Pushes an expression program and returns its index.
    pub fn push_expression(&mut self, program: ExprProgram) -> Result<ExprIdx, CompileError> {
        let index = u16::try_from(self.expressions.len()).map_err(|_| {
            CompileError::ExpressionLoweringUnsupported {
                feature: "expression table overflow".into(),
            }
        })?;
        self.expressions.push(program);
        Ok(ExprIdx::new(index))
    }

    /// Pushes an accessor program and returns its index.
    pub fn push_accessor(
        &mut self,
        program: AccessorProgram,
    ) -> Result<vb_core::AccessorIdx, CompileError> {
        let index = u16::try_from(self.accessors.len()).map_err(|_| {
            CompileError::ExpressionLoweringUnsupported {
                feature: "accessor table overflow".into(),
            }
        })?;
        self.accessors.push(program);
        Ok(vb_core::AccessorIdx::new(index))
    }

    /// Records a slot reference for slot count tracking.
    pub fn record_slot(&mut self, slot: SlotIdx) {
        let value = slot.as_usize();
        self.max_slot = Some(match self.max_slot {
            Some(current) => current.max(value),
            None => value,
        });
    }

    /// Pushes a compiled node into the node array.
    pub fn push_node(&mut self, node: CompiledNode) {
        self.nodes.push(node);
    }

    /// Returns the current slot count.
    pub fn slot_count(&self) -> Result<u16, CompileError> {
        match self.max_slot {
            Some(value) => {
                let count = value
                    .checked_add(1)
                    .ok_or(CompileError::SlotIndexOutOfRange { value: i64::MAX })?;
                u16::try_from(count).map_err(|_| CompileError::SlotIndexOutOfRange {
                    value: i64::from(u16::MAX),
                })
            }
            None => Ok(0),
        }
    }

    /// Builds the final workflow parts from accumulated state.
    pub fn build_parts(
        self,
        name: &str,
        digest: WorkflowDigest,
    ) -> Result<WorkflowParts, CompileError> {
        Ok(WorkflowParts {
            name: Box::from(name),
            digest,
            slot_count: self.slot_count()?,
            symbols_count: 0,
            nodes: self.nodes.into_boxed_slice(),
            expressions: self.expressions.into_boxed_slice(),
            accessors: self.accessors.into_boxed_slice(),
            constants: self.constants.into_boxed_slice(),
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
    }
}

fn validate_branch_route(
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
) -> Result<(), CompileError> {
    if branches.is_empty() && otherwise.is_none() {
        Err(CompileError::Workflow(WorkflowError::EmptyBranchTable))
    } else {
        Ok(())
    }
}

fn non_string_key_error() -> CompileError {
    CompileError::NonStringKey {
        mark: SourceMark::unavailable(),
    }
}

/// YAML compiler errors.
#[derive(Debug, Clone, Error)]
pub enum CompileError {
    /// Source exceeded configured byte limit.
    #[error("YAML source exceeds byte limit: actual={actual}, limit={limit}")]
    SourceTooLarge {
        /// Actual source size.
        actual: usize,
        /// Configured limit.
        limit: usize,
    },
    /// Source was not UTF-8.
    #[error("YAML source must be UTF-8: {0}")]
    Utf8(#[from] str::Utf8Error),
    /// Source did not contain a YAML document.
    #[error("YAML source must contain exactly one non-empty document")]
    EmptySource,
    /// Native YAML parser rejected the document.
    #[error("YAML parse failed: {0}")]
    Parse(#[from] saphyr::ScanError),
    /// Canonical YAML parser rejected the document.
    #[error("canonical YAML parse failed ({category}): {message}")]
    CanonicalYaml {
        /// Stable category from `vb_yaml::YamlError`.
        category: &'static str,
        /// Preserved YAML error message.
        message: Box<str>,
    },
    /// YAML streams are forbidden.
    #[error("expected exactly one YAML document, found {count}")]
    DocumentCount {
        /// Document count found by parser.
        count: usize,
    },
    /// The top-level YAML node must be a mapping.
    #[error("top-level YAML document must be a mapping")]
    TopLevelNotMapping,
    /// Mapping keys must be strings.
    #[error("mapping key must be a string at {mark:?}")]
    NonStringKey {
        /// Best available source mark.
        mark: SourceMark,
    },
    /// YAML mappings must not contain duplicate keys.
    #[error("duplicate YAML mapping key: {key} at {mark:?}")]
    DuplicateKey {
        /// Duplicated key.
        key: Box<str>,
        /// Best available source mark.
        mark: SourceMark,
    },
    /// YAML anchors/aliases are forbidden.
    #[error("YAML aliases are forbidden at {mark:?}")]
    AliasForbidden {
        /// Parser mark for the alias event.
        mark: SourceMark,
    },
    /// YAML anchors are forbidden.
    #[error("YAML anchors are forbidden at {mark:?}")]
    AnchorForbidden {
        /// Parser mark for the anchored node.
        mark: SourceMark,
    },
    /// YAML merge keys are forbidden.
    #[error("YAML merge keys are forbidden at {mark:?}")]
    MergeKeyForbidden {
        /// Best available source mark.
        mark: SourceMark,
    },
    /// YAML tags are forbidden.
    #[error("YAML tags are forbidden at {mark:?}")]
    TagForbidden {
        /// Parser mark for the tagged node.
        mark: SourceMark,
    },
    /// Saphyr produced a bad scalar value.
    #[error("YAML scalar value is invalid")]
    BadValue,
    /// Floating-point YAML scalars are forbidden in the initial profile.
    #[error("floating-point YAML scalars are forbidden")]
    FloatForbidden,
    /// YAML depth exceeded configured limit.
    #[error("YAML nesting depth exceeds limit: depth={depth}, limit={limit}")]
    DepthLimit {
        /// Observed depth.
        depth: u16,
        /// Configured depth limit.
        limit: u16,
    },
    /// YAML node count exceeded configured limit.
    #[error("YAML node count exceeds limit: limit={limit}")]
    NodeLimit {
        /// Configured node limit.
        limit: u32,
    },
    /// YAML sequence exceeded configured limit.
    #[error("YAML sequence length exceeds limit: actual={actual}, limit={limit}")]
    SequenceLimit {
        /// Actual sequence length.
        actual: usize,
        /// Configured sequence limit.
        limit: usize,
    },
    /// YAML mapping exceeded configured limit.
    #[error("YAML mapping entry count exceeds limit: actual={actual}, limit={limit}")]
    MappingLimit {
        /// Actual mapping entries.
        actual: usize,
        /// Configured mapping limit.
        limit: usize,
    },
    /// YAML scalar exceeded configured limit.
    #[error("YAML scalar length exceeds limit: actual={actual}, limit={limit}")]
    ScalarLimit {
        /// Actual scalar length.
        actual: usize,
        /// Configured scalar limit.
        limit: usize,
    },
    /// Compiled IR validation failed.
    #[error("compiled workflow IR failed validation: {0}")]
    Workflow(#[from] WorkflowError),
    /// Shared validation pipeline gate failure.
    #[error("validation gate failure: {0}")]
    Validation(#[from] vb_validate::ValidationError),
    /// Required workflow field is missing.
    #[error("required workflow field is missing: {field}")]
    MissingField {
        /// Missing field name.
        field: &'static str,
    },
    /// Top-level workflow field is not part of the supported schema.
    #[error("unknown top-level workflow field: {field}")]
    UnknownTopLevelField {
        /// Unknown field name.
        field: Box<str>,
    },
    /// Workflow version must match the public Velvet v1 version exactly.
    #[error("unsupported workflow version: {actual}")]
    InvalidVersion {
        /// Version found in source YAML.
        actual: Box<str>,
    },
    /// Workflow trigger declaration must contain exactly one trigger.
    #[error("workflow when must declare exactly one trigger, found {count}")]
    InvalidTriggerCount {
        /// Number of trigger entries found.
        count: usize,
    },
    /// Trigger kind is not part of Velvet v1.
    #[error("unknown workflow trigger kind: {trigger}")]
    UnknownTriggerKind {
        /// Unknown trigger kind.
        trigger: Box<str>,
    },
    /// Trigger configuration has the wrong YAML shape.
    #[error("trigger {trigger} must be {expected}")]
    TriggerShape {
        /// Trigger kind.
        trigger: Box<str>,
        /// Expected shape.
        expected: &'static str,
    },
    /// Trigger field is not valid for the selected trigger kind.
    #[error("trigger {trigger} has unknown field: {field}")]
    UnknownTriggerField {
        /// Trigger kind.
        trigger: &'static str,
        /// Unknown trigger field.
        field: Box<str>,
    },
    /// Required trigger field is missing.
    #[error("trigger {trigger} is missing required field: {field}")]
    MissingTriggerField {
        /// Trigger kind.
        trigger: &'static str,
        /// Missing trigger field.
        field: &'static str,
    },
    /// Trigger field value failed semantic validation.
    #[error("trigger {trigger} field {field} must be {expected}")]
    InvalidTriggerField {
        /// Trigger kind.
        trigger: &'static str,
        /// Trigger field.
        field: &'static str,
        /// Expected value shape or semantic rule.
        expected: &'static str,
    },
    /// Workflow field has the wrong YAML shape.
    #[error("workflow field {field} must be {expected}")]
    FieldShape {
        /// Field name.
        field: &'static str,
        /// Expected shape.
        expected: &'static str,
    },
    /// Input schema field is not part of Velvet v1.
    #[error("input schema has unknown field: {field}")]
    UnknownInputSchemaField {
        /// Unknown schema field.
        field: Box<str>,
    },
    /// Input schema field failed shape or semantic validation.
    #[error("input schema field {field} must be {expected}")]
    InvalidInputSchema {
        /// Schema field path.
        field: &'static str,
        /// Expected shape or semantic rule.
        expected: &'static str,
    },
    /// Phase 0 compiler does not yet compile top-level result mappings.
    #[error("non-empty top-level result is not supported by the Phase 0 compiler")]
    UnsupportedTopLevelResult,
    /// Canonical AST declarations not yet lowered by the narrow compiler slice.
    #[error("top-level declaration {field} is not supported by canonical compiler handoff")]
    UnsupportedTopLevelDeclaration { field: &'static str },
    /// Canonical set output name was declared more than once.
    #[error("duplicate set output name: {name}")]
    DuplicateOutputName { name: Box<str> },
    /// Canonical finish referenced an unknown output name.
    #[error("unknown finish output name: {name}")]
    UnknownOutputName { name: Box<str> },
    /// Workflow must contain at least one executable step.
    #[error("workflow steps must not be empty")]
    EmptySteps,
    /// Public workflow or step name does not match the Velvet v1 identifier grammar.
    #[error("{field} is not a valid Velvet v1 name: {value}")]
    InvalidName {
        /// Field containing the invalid name.
        field: &'static str,
        /// Invalid name value.
        value: Box<str>,
    },
    /// Step is missing its required public ID.
    #[error("step {step} is missing required id")]
    MissingStepId {
        /// Step index.
        step: usize,
    },
    /// Step ID appears more than once in the workflow.
    #[error("duplicate step id: {id}")]
    DuplicateStepId {
        /// Duplicate step ID.
        id: Box<str>,
    },
    /// Step must be a mapping.
    #[error("step {step} must be a mapping")]
    StepShape {
        /// Step index.
        step: usize,
    },
    /// Step field is not part of the Velvet v1 schema.
    #[error("step {step} has unknown field: {field}")]
    UnknownStepField {
        /// Step index.
        step: usize,
        /// Unknown field name.
        field: Box<str>,
    },
    /// Primitive body field is not accepted by the Phase 0 compiler.
    #[error("step {step} primitive {primitive} has unknown field: {field}")]
    UnknownStepPrimitiveField {
        /// Step index.
        step: usize,
        /// Primitive containing the field.
        primitive: &'static str,
        /// Unknown primitive field.
        field: Box<str>,
    },
    /// Step is missing its single required primitive.
    #[error("step {step} is missing a primitive field")]
    MissingStepPrimitive {
        /// Step index.
        step: usize,
    },
    /// Step contains more than one primitive.
    #[error("step {step} has multiple primitive fields")]
    MultipleStepPrimitives {
        /// Step index.
        step: usize,
    },
    /// Primitive is valid Velvet v1 but not compiled by the Phase 0 IR subset.
    #[error("step {step} primitive {primitive} is not supported by the Phase 0 compiler")]
    UnsupportedStepPrimitive {
        /// Step index.
        step: usize,
        /// Canonical primitive name.
        primitive: &'static str,
    },
    /// Step control field is valid Velvet v1 but not compiled by the Phase 0 IR subset.
    #[error("step {step} control field {field} is not supported by the Phase 0 compiler")]
    UnsupportedStepControlField {
        /// Step index.
        step: usize,
        /// Unsupported control field.
        field: Box<str>,
    },
    /// Required step field is missing.
    #[error("step {step} is missing required field: {field}")]
    MissingStepField {
        /// Step index.
        step: usize,
        /// Missing field name.
        field: &'static str,
    },
    /// Step field has the wrong YAML shape.
    #[error("step {step} field {field} must be {expected}")]
    StepFieldShape {
        /// Step index.
        step: usize,
        /// Field name.
        field: &'static str,
        /// Expected shape.
        expected: &'static str,
    },
    /// Numeric step index exceeds the IR representation.
    #[error("step index exceeds u16: {value}")]
    StepIndexOutOfRange {
        /// Invalid value.
        value: usize,
    },
    /// Slot index must be an unsigned u16.
    #[error("slot index is outside u16 range: {value}")]
    SlotIndexOutOfRange {
        /// Invalid value.
        value: i64,
    },
    /// Branch target must be an unsigned u16.
    #[error("branch target is outside u16 range: {value}")]
    BranchTargetOutOfRange {
        /// Invalid value.
        value: i64,
    },
    /// Branch target must point forward in v1.
    #[error("branch target {target} at step {step} must point forward")]
    BackwardBranchTarget {
        /// Step containing the branch.
        step: usize,
        /// Invalid target.
        target: usize,
    },
    /// Primitive lowering would exceed a bounded compiler representation.
    #[error("step primitive {primitive} field {field} value {value} exceeds limit {limit}")]
    PrimitiveLoweringLimitExceeded {
        /// Primitive being lowered.
        primitive: &'static str,
        /// Bounded field being computed.
        field: &'static str,
        /// Attempted value or source value at the limit.
        value: usize,
        /// Maximum accepted representation value.
        limit: usize,
    },
    /// Linear workflows must end with an explicit finish step.
    #[error("last workflow step must be finish")]
    LastStepMustFinish,
    /// Constant values must be scalar YAML values.
    #[error("constant value for step {step} must be a scalar")]
    UnsupportedConstantValue {
        /// Step index.
        step: usize,
    },
    /// Reference root is not part of the bounded Velvet v1 reference surface.
    #[error("unknown reference root in {reference}: {root}")]
    UnknownReferenceRoot {
        /// Full source reference string.
        reference: Box<str>,
        /// Unknown root segment without the leading `$`.
        root: Box<str>,
    },
    /// Reference root is known but forbidden in deterministic compiled IR.
    #[error("illegal reference in deterministic workflow: {reference}")]
    IllegalReference {
        /// Full source reference string.
        reference: Box<str>,
    },
    /// Reference points at an undeclared input, variable, secret, or step.
    #[error("unknown {kind} reference in {reference}: {name}")]
    UnknownReferenceName {
        /// Declaration table that was searched.
        kind: &'static str,
        /// Full source reference string.
        reference: Box<str>,
        /// Missing declaration name.
        name: Box<str>,
    },
    /// Reference uses an accessor path outside the current compiled surface.
    #[error("unsupported accessor reference in {reference}: {root}.{path}")]
    UnsupportedAccessorReference {
        /// Full source reference string.
        reference: Box<str>,
        /// Resolved root segment.
        root: Box<str>,
        /// Unsupported accessor tail.
        path: Box<str>,
    },
    /// Branch target points outside the declared step table.
    #[error("step {step} branch target {target} is not a declared step")]
    UnknownStepTarget {
        /// Step containing the invalid target.
        step: usize,
        /// Missing target index.
        target: usize,
    },
    /// A declared step cannot be reached from the entry step.
    #[error("step {step} is unreachable from workflow entry")]
    UnreachableStep {
        /// Unreachable step index.
        step: usize,
    },
    /// Expression type did not match the field contract.
    #[error("type mismatch in {field}: expected {expected}, found {found}")]
    TypeMismatch {
        /// Field being validated.
        field: &'static str,
        /// Required type.
        expected: &'static str,
        /// Inferred type.
        found: &'static str,
    },
    /// Expression referenced a slot whose type is not known at validation time.
    #[error("unknown slot type in {field}: {slot}")]
    UnknownSlotType {
        /// Field being validated.
        field: &'static str,
        /// Missing slot index.
        slot: usize,
    },
    /// Secret-tainted data cannot cross a public result boundary.
    #[error("secret-tainted value cannot be used in {field}")]
    SecretTaintLeak {
        /// Field being validated.
        field: &'static str,
    },
    /// Expression lexer found a character outside the v1 expression grammar.
    #[error("expression lex failed at byte {index} in {expression}: unexpected {found:?}")]
    ExpressionUnexpectedChar {
        /// Full source expression.
        expression: Box<str>,
        /// Byte index in the expression string.
        index: usize,
        /// Character that could not be tokenized.
        found: char,
    },
    /// Expression lexer reached EOF inside a string literal.
    #[error("expression string is unterminated at byte {index} in {expression}")]
    ExpressionUnterminatedString {
        /// Full source expression.
        expression: Box<str>,
        /// Opening quote byte index.
        index: usize,
    },
    /// Expression integer literal exceeded i64.
    #[error("expression integer is outside i64 range at byte {index} in {expression}")]
    ExpressionIntegerOutOfRange {
        /// Full source expression.
        expression: Box<str>,
        /// Literal start byte index.
        index: usize,
    },
    /// Expression float literal is non-finite (inf/nan) or otherwise invalid.
    #[error("expression float is invalid at byte {index} in {expression}")]
    ExpressionFloatOutOfRange {
        /// Full source expression.
        expression: Box<str>,
        /// Literal start byte index.
        index: usize,
    },
    /// Expression exceeded a compiler-side hard bound.
    #[error("expression exceeds {limit} limit {max} in {expression}")]
    ExpressionLimitExceeded {
        /// Full source expression.
        expression: Box<str>,
        /// Limit category.
        limit: &'static str,
        /// Maximum allowed value.
        max: usize,
    },
    /// Expression parser found the wrong token shape.
    #[error("expression parse failed at byte {index} in {expression}: expected {expected}")]
    ExpressionUnexpectedToken {
        /// Full source expression.
        expression: Box<str>,
        /// Byte index in the expression string.
        index: usize,
        /// Expected syntactic element.
        expected: &'static str,
    },
    /// Expression parser does not accept bare identifiers beyond literals.
    #[error("unknown expression identifier at byte {index} in {expression}: {identifier}")]
    ExpressionUnknownIdentifier {
        /// Full source expression.
        expression: Box<str>,
        /// Byte index in the expression string.
        index: usize,
        /// Unknown identifier.
        identifier: Box<str>,
    },
    /// Expression bytecode lowering needs a later compiler/runtime table.
    #[error("expression bytecode lowering does not support {feature} yet")]
    ExpressionLoweringUnsupported {
        /// Unsupported expression feature.
        feature: Box<str>,
    },
    /// Helper call has the wrong number of arguments for bytecode lowering.
    #[error("expression helper {helper} expects {expected} args, found {actual}")]
    ExpressionHelperArity {
        /// Helper name.
        helper: &'static str,
        /// Required arity.
        expected: usize,
        /// Actual argument count.
        actual: usize,
    },
    /// Side-effecting action lacks safe retry semantics.
    #[error("action {action:?} has side-effect {side_effect:?} with unsafe retry: {reason}")]
    IdempotencyViolation {
        /// Action that failed the idempotency gate.
        action: ActionId,
        /// Side-effect classification of the action.
        side_effect: SideEffect,
        /// Human-readable reason for the rejection.
        reason: Box<str>,
    },
}

impl CompileError {
    /// Stable machine-readable validation diagnostic code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::SourceTooLarge { .. } => "PAYLOAD_TOO_LARGE",
            Self::Utf8(_)
            | Self::Parse(_)
            | Self::DocumentCount { .. }
            | Self::NonStringKey { .. }
            | Self::AliasForbidden { .. }
            | Self::AnchorForbidden { .. }
            | Self::MergeKeyForbidden { .. }
            | Self::TagForbidden { .. }
            | Self::BadValue
            | Self::FloatForbidden => "FORBIDDEN_YAML_FEATURE",
            Self::EmptySource
            | Self::MissingField { .. }
            | Self::MissingTriggerField { .. }
            | Self::MissingStepId { .. }
            | Self::MissingStepField { .. } => "MISSING_REQUIRED_FIELD",
            Self::TopLevelNotMapping
            | Self::FieldShape { .. }
            | Self::InvalidInputSchema { .. }
            | Self::StepShape { .. }
            | Self::UnsupportedConstantValue { .. }
            | Self::TypeMismatch { .. }
            | Self::UnknownSlotType { .. } => "TYPE_MISMATCH",
            Self::DuplicateKey { .. } => "DUPLICATE_KEY",
            Self::DepthLimit { .. }
            | Self::NodeLimit { .. }
            | Self::SequenceLimit { .. }
            | Self::MappingLimit { .. }
            | Self::ScalarLimit { .. }
            | Self::StepIndexOutOfRange { .. }
            | Self::SlotIndexOutOfRange { .. }
            | Self::BranchTargetOutOfRange { .. }
            | Self::PrimitiveLoweringLimitExceeded { .. } => "LIMIT_EXCEEDED",
            Self::Workflow(error) => workflow_error_code(error),
            Self::UnknownTopLevelField { .. } => "UNKNOWN_TOP_LEVEL_FIELD",
            Self::InvalidVersion { .. } => "INVALID_VERSION",
            Self::InvalidTriggerCount { .. }
            | Self::UnknownTriggerKind { .. }
            | Self::TriggerShape { .. }
            | Self::UnknownTriggerField { .. }
            | Self::InvalidTriggerField { .. } => "UNSUPPORTED_TRIGGER",
            Self::UnknownInputSchemaField { .. } => "UNKNOWN_INPUT_SCHEMA_FIELD",
            Self::UnsupportedTopLevelResult | Self::LastStepMustFinish => "INVALID_FINISH",
            Self::UnsupportedTopLevelDeclaration { .. } => "UNSUPPORTED_TOP_LEVEL_DECLARATION",
            Self::EmptySteps | Self::MissingStepPrimitive { .. } => "MISSING_STEP_PRIMITIVE",
            Self::InvalidName { field, value } => invalid_name_code(field, value),
            Self::DuplicateStepId { .. } | Self::DuplicateOutputName { .. } => "DUPLICATE_ID",
            Self::UnknownOutputName { .. } => "UNKNOWN_OUTPUT_NAME",
            Self::UnknownStepField { .. } | Self::UnknownStepPrimitiveField { .. } => {
                "UNKNOWN_STEP_FIELD"
            }
            Self::MultipleStepPrimitives { .. } => "MULTIPLE_STEP_PRIMITIVES",
            Self::UnsupportedStepPrimitive { primitive, .. } => primitive_code(primitive),
            Self::UnsupportedStepControlField { field, .. } => control_field_code(field),
            Self::StepFieldShape { field, .. } => step_field_shape_code(field),
            Self::BackwardBranchTarget { .. } | Self::UnknownStepTarget { .. } => {
                "INVALID_THEN_TARGET"
            }
            Self::UnknownReferenceRoot { .. } => "UNKNOWN_REFERENCE",
            Self::IllegalReference { .. } => "DIRECT_RUNTIME_REFERENCE",
            Self::UnknownReferenceName { kind, .. } => unknown_reference_code(kind),
            Self::UnsupportedAccessorReference { .. } => "UNSUPPORTED_ACCESSOR_REFERENCE",
            Self::UnreachableStep { .. } => "UNREACHABLE_STEP",
            Self::SecretTaintLeak { .. } => "SECRET_RESULT_LEAK",
            Self::ExpressionUnexpectedChar { .. }
            | Self::ExpressionUnterminatedString { .. }
            | Self::ExpressionIntegerOutOfRange { .. }
            | Self::ExpressionFloatOutOfRange { .. }
            | Self::ExpressionLimitExceeded { .. }
            | Self::ExpressionUnexpectedToken { .. }
            | Self::ExpressionUnknownIdentifier { .. }
            | Self::ExpressionLoweringUnsupported { .. }
            | Self::ExpressionHelperArity { .. } => "INVALID_EXPRESSION",
            Self::IdempotencyViolation { .. } => "IDEMPOTENCY_VIOLATION",
            Self::Validation(error) => validation_error_code(error),
            Self::CanonicalYaml { category, .. } => canonical_yaml_code(category),
        }
    }

    /// Alias for integrations that name the machine field explicitly.
    #[must_use]
    pub fn diagnostic_code(&self) -> &'static str {
        self.code()
    }
}

fn canonical_yaml_code(category: &str) -> &'static str {
    match category {
        "duplicate_key" => "DUPLICATE_KEY",
        "document_count" => "FORBIDDEN_YAML_FEATURE",
        "limit_exceeded" => "LIMIT_EXCEEDED",
        "unknown_field" => "UNKNOWN_TOP_LEVEL_FIELD",
        "empty_source" | "missing_field" => "MISSING_REQUIRED_FIELD",
        "field_shape" => "TYPE_MISMATCH",
        "parse_error" | "forbidden_feature" => "FORBIDDEN_YAML_FEATURE",
        _ => "FORBIDDEN_YAML_FEATURE",
    }
}

fn workflow_error_code(error: &WorkflowError) -> &'static str {
    match error {
        WorkflowError::ResourceContractExceeded { .. }
        | WorkflowError::ResourceContractTooLarge { .. }
        | WorkflowError::BudgetPolicyExceeded { .. } => "LIMIT_EXCEEDED",
        WorkflowError::StepOutOfBounds { .. } => "INVALID_THEN_TARGET",
        WorkflowError::SlotOutOfBounds { .. } => "TYPE_MISMATCH",
        WorkflowError::ConstOutOfBounds { .. } => "CONST_OUT_OF_BOUNDS",
        WorkflowError::Expression(_) => "INVALID_EXPRESSION",
        WorkflowError::EmptyNodes
        | WorkflowError::EntryOutOfBounds { .. }
        | WorkflowError::NodeIdMismatch { .. }
        | WorkflowError::EmptyBranchTable
        | WorkflowError::UnreachableNode { .. }
        | WorkflowError::BackwardEdge { .. }
        | WorkflowError::ImproperLoopNesting { .. }
        | WorkflowError::SymbolOutOfBounds { .. }
        | WorkflowError::AccessorPathTooDeep { .. }
        | WorkflowError::StepCountOverflow { .. }
        | WorkflowError::JumpCycle { .. } => "INVALID_COMPILED_WORKFLOW",
    }
}

fn validation_error_code(error: &vb_validate::ValidationError) -> &'static str {
    match error {
        vb_validate::ValidationError::ExpressionStackExceeded { .. }
        | vb_validate::ValidationError::ExpressionStackMismatch { .. } => "LIMIT_EXCEEDED",
        vb_validate::ValidationError::AccessorSlotOutOfRange { .. }
        | vb_validate::ValidationError::AccessorPathInvalid { .. } => "TYPE_MISMATCH",
        vb_validate::ValidationError::SlotReferenceOutOfRange { .. } => "TYPE_MISMATCH",
        vb_validate::ValidationError::LoopBodyStepOutOfRange { .. } => "INVALID_THEN_TARGET",
        vb_validate::ValidationError::SlotDependencyCycle { .. } => "INVALID_COMPILED_WORKFLOW",
        _ => "INVALID_COMPILED_WORKFLOW",
    }
}

fn invalid_name_code(_field: &str, value: &str) -> &'static str {
    if is_reserved_name(value) {
        "RESERVED_ID"
    } else {
        "INVALID_ID"
    }
}

fn primitive_code(primitive: &str) -> &'static str {
    match primitive {
        "for_each" => "INVALID_FOR_EACH",
        "together" => "INVALID_TOGETHER",
        "collect" | "gather" => "INVALID_COLLECT",
        "reduce" | "summarize" => "INVALID_REDUCE",
        "repeat" => "INVALID_REPEAT",
        "wait" => "INVALID_WAIT",
        "ask" => "INVALID_ASK",
        "try_again" => "INVALID_RETRY",
        "on_error" => "INVALID_ON_ERROR",
        "finish" => "INVALID_FINISH",
        "choose" => "INVALID_CHOOSE",
        _ => "UNKNOWN_STEP_FIELD",
    }
}

fn control_field_code(field: &str) -> &'static str {
    match field {
        "then" => "INVALID_THEN_TARGET",
        "try_again" => "INVALID_RETRY",
        "on_error" => "INVALID_ON_ERROR",
        _ => "UNKNOWN_STEP_FIELD",
    }
}

fn step_field_shape_code(field: &str) -> &'static str {
    match field {
        "choose" | "condition" | "on_true" | "on_false" => "INVALID_CHOOSE",
        "for_each" => "INVALID_FOR_EACH",
        "together" | "branches" => "INVALID_TOGETHER",
        "collect" => "INVALID_COLLECT",
        "reduce" => "INVALID_REDUCE",
        "repeat" => "INVALID_REPEAT",
        "finish" | "result" => "INVALID_FINISH",
        _ => "TYPE_MISMATCH",
    }
}

fn unknown_reference_code(kind: &str) -> &'static str {
    if kind == "secret" || kind == "secrets" {
        "SECRET_NOT_DECLARED"
    } else {
        "UNKNOWN_REFERENCE"
    }
}

/// Multiple compilation errors collected in one pass (railway programming).
#[derive(Debug)]
pub struct CompileErrors(pub Vec<CompileError>);

impl CompileErrors {
    /// Returns the first error, or None if empty (should not happen by construction).
    #[must_use]
    pub fn first(&self) -> Option<&CompileError> {
        self.0.first()
    }

    /// Returns all collected errors as a slice.
    #[must_use]
    pub fn as_slice(&self) -> &[CompileError] {
        &self.0
    }

    /// Iterates over collected errors in reporting order.
    #[allow(clippy::iter_without_into_iter)]
    pub fn iter(&self) -> std::slice::Iter<'_, CompileError> {
        self.0.iter()
    }

    /// Iterates over stable machine-readable diagnostic codes in reporting order.
    pub fn diagnostic_codes(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.0.iter().map(CompileError::code)
    }

    /// Total number of collected errors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if there are no errors (should never happen by construction).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for CompileErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, error) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "[{i}] {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for CompileErrors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.first() {
            Some(error) => Some(error),
            None => None,
        }
    }
}

/// Appends an error to the collector, if the result is `Err`.
fn collect(errors: &mut Vec<CompileError>, result: Result<(), CompileError>) {
    if let Err(error) = result {
        errors.push(error);
    }
}

fn checked_utf8(source: &[u8], limits: YamlLimits) -> Result<&str, CompileError> {
    if source.len() > limits.max_source_bytes {
        return Err(CompileError::SourceTooLarge {
            actual: source.len(),
            limit: limits.max_source_bytes,
        });
    }
    let text = str::from_utf8(source)?;
    if text.trim().is_empty() {
        Err(CompileError::EmptySource)
    } else {
        Ok(text)
    }
}

fn single_document<'a>(docs: &'a [Yaml<'a>]) -> Result<&'a Yaml<'a>, CompileError> {
    match docs {
        [doc] => Ok(doc),
        _ => Err(CompileError::DocumentCount { count: docs.len() }),
    }
}

fn reject_duplicate_mapping_keys(text: &str) -> Result<(), CompileError> {
    let mut parser = Parser::new_from_str(text);

    while let Some((event, mark)) = parser.next_event().transpose()? {
        validate_duplicate_keys_in_started_node(event, mark, &mut parser)?;
    }

    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
fn validate_duplicate_keys_in_started_node<'input>(
    event: Event<'input>,
    mark: Span,
    parser: &mut Parser<'input, StrInput<'input>>,
) -> Result<(), CompileError> {
    match event {
        Event::MappingStart(_, _) => validate_duplicate_keys_in_mapping(parser),
        Event::SequenceStart(_, _) => validate_duplicate_keys_in_sequence(parser),
        Event::Alias(_) => Err(CompileError::AliasForbidden {
            mark: SourceMark::from_parser_span(mark),
        }),
        _ => Ok(()),
    }
}

fn validate_duplicate_keys_in_mapping<'input>(
    parser: &mut Parser<'input, StrInput<'input>>,
) -> Result<(), CompileError> {
    let mut seen = HashSet::new();
    loop {
        let Some((key_event, key_mark)) = parser.next_event().transpose()? else {
            return Ok(());
        };
        if key_event == Event::MappingEnd {
            return Ok(());
        }
        validate_unique_mapping_key(key_event, key_mark, &mut seen)?;
        let Some((value_event, value_mark)) = parser.next_event().transpose()? else {
            return Ok(());
        };
        validate_duplicate_keys_in_started_node(value_event, value_mark, parser)?;
    }
}

fn validate_unique_mapping_key(
    event: Event<'_>,
    mark: Span,
    seen: &mut HashSet<Box<str>>,
) -> Result<(), CompileError> {
    let key = mapping_key_text(event, mark)?;
    let duplicate = key.clone();
    if seen.insert(key) {
        Ok(())
    } else {
        Err(CompileError::DuplicateKey {
            key: duplicate,
            mark: SourceMark::from_parser_span(mark),
        })
    }
}

fn validate_duplicate_keys_in_sequence<'input>(
    parser: &mut Parser<'input, StrInput<'input>>,
) -> Result<(), CompileError> {
    loop {
        let Some((event, mark)) = parser.next_event().transpose()? else {
            return Ok(());
        };
        if event == Event::SequenceEnd {
            return Ok(());
        }
        validate_duplicate_keys_in_started_node(event, mark, parser)?;
    }
}

fn mapping_key_text(event: Event<'_>, mark: Span) -> Result<Box<str>, CompileError> {
    let source_mark = SourceMark::from_parser_span(mark);
    match event {
        Event::Scalar(value, style, _, tag) => {
            let key = Yaml::value_from_cow_and_metadata(value, style, tag.as_ref());
            match key.as_str() {
                Some("<<") => Err(CompileError::MergeKeyForbidden { mark: source_mark }),
                Some(value) => Ok(Box::<str>::from(value)),
                None => Err(CompileError::NonStringKey { mark: source_mark }),
            }
        }
        Event::Alias(_) => Err(CompileError::AliasForbidden { mark: source_mark }),
        _ => Err(CompileError::NonStringKey { mark: source_mark }),
    }
}

fn validate_strict_profile(root: &Yaml<'_>, limits: YamlLimits) -> Result<(), CompileError> {
    if !root.is_mapping() {
        return Err(CompileError::TopLevelNotMapping);
    }

    let mut stack = vec![(root, 0_u16)];
    let mut visited = 0_u32;

    while let Some((node, depth)) = stack.pop() {
        visited = next_visited_count(visited, limits)?;
        validate_depth(depth, limits)?;
        validate_one_node(node, depth, limits, &mut stack)?;
    }

    Ok(())
}

fn next_visited_count(visited: u32, limits: YamlLimits) -> Result<u32, CompileError> {
    let next = visited.checked_add(1).ok_or(CompileError::NodeLimit {
        limit: limits.max_nodes,
    })?;
    if next > limits.max_nodes {
        Err(CompileError::NodeLimit {
            limit: limits.max_nodes,
        })
    } else {
        Ok(next)
    }
}

fn validate_depth(depth: u16, limits: YamlLimits) -> Result<(), CompileError> {
    if depth > limits.max_depth {
        Err(CompileError::DepthLimit {
            depth,
            limit: limits.max_depth,
        })
    } else {
        Ok(())
    }
}

fn validate_one_node<'a>(
    node: &'a Yaml<'a>,
    depth: u16,
    limits: YamlLimits,
    stack: &mut Vec<(&'a Yaml<'a>, u16)>,
) -> Result<(), CompileError> {
    match node {
        Yaml::Mapping(mapping) => push_mapping(mapping, depth, limits, stack),
        Yaml::Sequence(sequence) => push_sequence(sequence, depth, limits, stack),
        Yaml::Tagged(_, _) => Err(CompileError::TagForbidden {
            mark: SourceMark::unavailable(),
        }),
        Yaml::Alias(_) => Err(CompileError::AliasForbidden {
            mark: SourceMark::unavailable(),
        }),
        Yaml::BadValue => Err(CompileError::BadValue),
        Yaml::Value(value) => validate_scalar(value, limits),
        Yaml::Representation(value, _, tag) => {
            validate_representation(value.as_ref(), tag.is_some(), limits)
        }
    }
}

fn validate_representation(
    value: &str,
    has_tag: bool,
    limits: YamlLimits,
) -> Result<(), CompileError> {
    if has_tag {
        return Err(CompileError::TagForbidden {
            mark: SourceMark::unavailable(),
        });
    }
    validate_scalar_len(value, limits)
}

fn push_mapping<'a>(
    mapping: &'a saphyr::Mapping<'a>,
    depth: u16,
    limits: YamlLimits,
    stack: &mut Vec<(&'a Yaml<'a>, u16)>,
) -> Result<(), CompileError> {
    validate_mapping_len(mapping, limits)?;
    let next_depth = depth.checked_add(1).ok_or(CompileError::DepthLimit {
        depth,
        limit: limits.max_depth,
    })?;
    let mut seen = HashSet::with_capacity(mapping.len());
    for (key, value) in mapping {
        let key = validate_mapping_key(key, limits)?;
        if !seen.insert(key) {
            return Err(CompileError::DuplicateKey {
                key: Box::<str>::from(key),
                mark: SourceMark::unavailable(),
            });
        }
        stack.push((value, next_depth));
    }
    Ok(())
}

fn validate_mapping_len(
    mapping: &saphyr::Mapping<'_>,
    limits: YamlLimits,
) -> Result<(), CompileError> {
    if mapping.len() > limits.max_mapping_entries {
        Err(CompileError::MappingLimit {
            actual: mapping.len(),
            limit: limits.max_mapping_entries,
        })
    } else {
        Ok(())
    }
}

fn push_sequence<'a>(
    sequence: &'a saphyr::Sequence<'a>,
    depth: u16,
    limits: YamlLimits,
    stack: &mut Vec<(&'a Yaml<'a>, u16)>,
) -> Result<(), CompileError> {
    if sequence.len() > limits.max_sequence_len {
        return Err(CompileError::SequenceLimit {
            actual: sequence.len(),
            limit: limits.max_sequence_len,
        });
    }
    let next_depth = depth.checked_add(1).ok_or(CompileError::DepthLimit {
        depth,
        limit: limits.max_depth,
    })?;
    for item in sequence {
        stack.push((item, next_depth));
    }
    Ok(())
}

fn validate_mapping_key<'a>(
    key: &'a Yaml<'a>,
    limits: YamlLimits,
) -> Result<&'a str, CompileError> {
    match key.as_str() {
        Some(value) => {
            validate_scalar_len(value, limits)?;
            if value == "<<" {
                Err(CompileError::MergeKeyForbidden {
                    mark: SourceMark::unavailable(),
                })
            } else {
                Ok(value)
            }
        }
        None => Err(CompileError::NonStringKey {
            mark: SourceMark::unavailable(),
        }),
    }
}

fn validate_scalar(value: &saphyr::Scalar<'_>, limits: YamlLimits) -> Result<(), CompileError> {
    match value {
        saphyr::Scalar::String(value) => validate_scalar_len(value.as_ref(), limits),
        saphyr::Scalar::FloatingPoint(_) => Err(CompileError::FloatForbidden),
        saphyr::Scalar::Null | saphyr::Scalar::Boolean(_) | saphyr::Scalar::Integer(_) => Ok(()),
    }
}

fn validate_scalar_len(value: &str, limits: YamlLimits) -> Result<(), CompileError> {
    if value.len() > limits.max_scalar_bytes {
        Err(CompileError::ScalarLimit {
            actual: value.len(),
            limit: limits.max_scalar_bytes,
        })
    } else {
        Ok(())
    }
}

#[allow(dead_code)]
fn build_workflow_parts(text: &str, doc: &Yaml<'_>) -> Result<WorkflowParts, CompileError> {
    validate_workflow_document_shape(doc)?;

    let name = required_string_field(doc, "name")?;
    let steps = required_sequence_field(doc, "steps")?;
    let digest = WorkflowDigest::from_bytes(blake3::hash(text.as_bytes()).into());
    let mut builder = WorkflowBuilder::new();
    let last_step = steps.len().checked_sub(1).ok_or(CompileError::EmptySteps)?;
    let source_ir_starts = build_source_ir_starts(steps)?;

    let total_nodes = source_ir_starts
        .last()
        .map(|s| s.as_usize())
        .unwrap_or(0)
        .checked_add(compiled_step_width(
            steps.last().ok_or(CompileError::EmptySteps)?,
            last_step,
        )?)
        .unwrap_or(0);
    let mut step_names: Vec<Box<str>> = Vec::new();
    step_names.resize_with(total_nodes, || Box::from(""));

    for (index, step) in steps.iter().enumerate() {
        let id = source_ir_start(&source_ir_starts, index)?;
        let next = optional_source_ir_start(&source_ir_starts, index)?;
        let step_id_str = required_step_id(step, index)?;
        let width = compiled_step_width(step, index)?;
        let start = id.as_usize();
        let end = start.checked_add(width).unwrap_or(start);
        for pos in start..end {
            if let Some(slot) = step_names.get_mut(pos) {
                *slot = Box::from(step_id_str);
            }
        }
        let nodes = compile_step(
            step,
            index,
            last_step,
            id,
            next,
            &source_ir_starts,
            &mut builder,
        )?;
        builder.nodes.extend(nodes);
    }
    Ok(WorkflowParts {
        name: Box::<str>::from(name),
        digest,
        slot_count: builder.slot_count()?,
        symbols_count: 0,
        nodes: builder.nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: step_names.into_boxed_slice(),
    })
}

#[allow(dead_code)]
fn build_source_ir_starts(steps: &saphyr::Sequence<'_>) -> Result<Vec<StepIdx>, CompileError> {
    let mut starts = Vec::with_capacity(steps.len());
    let mut cursor = 0usize;
    for (index, step) in steps.iter().enumerate() {
        starts.push(step_idx(cursor)?);
        cursor = cursor
            .checked_add(compiled_step_width(step, index)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: cursor })?;
    }
    Ok(starts)
}

#[allow(dead_code)]
fn compiled_step_width(step: &Yaml<'_>, index: usize) -> Result<usize, CompileError> {
    let StepSpec { primitive, body } = step_spec(step, index)?;
    match primitive {
        StepPrimitive::Ask | StepPrimitive::ForEach | StepPrimitive::Together => Ok(2),
        StepPrimitive::Collect | StepPrimitive::Reduce | StepPrimitive::Repeat => Ok(3),
        StepPrimitive::Finish => {
            let result = required_step_field(body, index, "result")?;
            if finish_result_slot(result, index)?.is_some() {
                Ok(1)
            } else {
                Ok(2)
            }
        }
        _ => Ok(1),
    }
}

#[allow(dead_code)]
fn source_ir_start(starts: &[StepIdx], index: usize) -> Result<StepIdx, CompileError> {
    starts
        .get(index)
        .copied()
        .ok_or(CompileError::StepIndexOutOfRange { value: index })
}

#[allow(dead_code)]
fn optional_source_ir_start(
    starts: &[StepIdx],
    index: usize,
) -> Result<Option<StepIdx>, CompileError> {
    let next = index
        .checked_add(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: index })?;
    Ok(starts.get(next).copied())
}

fn validate_workflow_document_shape(doc: &Yaml<'_>) -> Result<(), CompileError> {
    validate_top_level_keys(doc)?;
    validate_workflow_version(doc)?;
    validate_workflow_trigger(doc)?;
    validate_optional_top_level_shapes(doc)?;
    validate_phase_zero_result(doc)?;
    let name = required_string_field(doc, "name")?;
    validate_public_name("name", name)?;
    let steps = required_sequence_field(doc, "steps")?;
    if steps.is_empty() {
        return Err(CompileError::EmptySteps);
    }
    validate_step_ids(steps)?;
    validate_phase_zero_step_shapes(steps)
}

fn validate_phase_zero_step_shapes(steps: &saphyr::Sequence<'_>) -> Result<(), CompileError> {
    let last_step = steps.len().checked_sub(1).ok_or(CompileError::EmptySteps)?;
    for (index, step) in steps.iter().enumerate() {
        validate_phase_zero_step_shape(step, index, last_step)?;
    }
    Ok(())
}

fn validate_phase_zero_step_shape(
    step: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    let StepSpec { primitive, body } = step_spec(step, index)?;
    match primitive {
        StepPrimitive::Run | StepPrimitive::Do => {
            validate_run_shape(body, index, last_step, primitive.as_str())
        }
        StepPrimitive::Set | StepPrimitive::Save => {
            validate_save_shape(body, index, last_step, primitive.as_str())
        }
        StepPrimitive::Choose => validate_choose_shape(body, index, last_step),
        StepPrimitive::ForEach => validate_for_each_shape(body, index, last_step),
        StepPrimitive::Together => validate_together_shape(body, index, last_step),
        StepPrimitive::Collect => validate_collect_shape(body, index, last_step),
        StepPrimitive::Reduce => validate_reduce_shape(body, index, last_step),
        StepPrimitive::Repeat => validate_repeat_shape(body, index, last_step),
        StepPrimitive::Wait => validate_wait_shape(body, index, last_step),
        StepPrimitive::Ask => validate_ask_shape(body, index, last_step),
        StepPrimitive::Finish => validate_finish_shape(body, index, last_step),
    }
}

fn validate_run_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    primitive: &'static str,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    if !body.is_mapping() {
        return Err(CompileError::UnsupportedStepPrimitive {
            step: index,
            primitive,
        });
    }
    reject_unknown_primitive_fields(body, index, primitive, &["action", "input"])?;
    required_action(body, index, primitive)?;
    required_slot(body, index, "input")?;
    Ok(())
}

fn validate_wait_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "wait", &["until", "event", "timeout"])?;
    let until = optional_slot_field(body, index, "until")?;
    let event = optional_slot_field(body, index, "event")?;
    let timeout = optional_slot_field(body, index, "timeout")?;
    match (until, event, timeout) {
        (Some(_), None, None) | (None, Some(_), _) => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field: "wait",
            expected: "until without timeout or event with optional timeout",
        }),
    }
}

fn validate_ask_shape(body: &Yaml<'_>, index: usize, last_step: usize) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "ask", &["prompt", "answer", "timeout"])?;
    required_slot(body, index, "prompt")?;
    required_slot(body, index, "answer")?;
    optional_slot_field(body, index, "timeout")?;
    Ok(())
}

fn validate_save_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    primitive: &'static str,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    if body.is_mapping() {
        Ok(())
    } else {
        Err(CompileError::StepFieldShape {
            step: index,
            field: primitive,
            expected: "an object",
        })
    }
}

fn validate_choose_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "choose", &["condition", "on_true", "on_false"])?;
    required_step_field(body, index, "condition")?;
    required_branch_target(body, index, "on_true")?;
    required_branch_target(body, index, "on_false")?;
    Ok(())
}

fn validate_for_each_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unsupported_for_each_fields(body, index)?;
    reject_unknown_primitive_fields(
        body,
        index,
        "for_each",
        &["input", "item", "limit", "at_once"],
    )?;
    required_slot(body, index, "input")?;
    required_slot(body, index, "item")?;
    required_u32_field(body, index, "for_each", "limit")?;
    Ok(())
}

fn reject_unsupported_for_each_fields(_body: &Yaml<'_>, _step: usize) -> Result<(), CompileError> {
    Ok(())
}

fn validate_together_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "together", &["branches"])?;
    required_branch_targets(body, index, "branches")?;
    Ok(())
}

fn validate_collect_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "collect", &["source", "limit", "page_size"])?;
    required_slot(body, index, "source")?;
    required_u32_field(body, index, "collect", "limit")?;
    required_u32_field(body, index, "collect", "page_size")?;
    Ok(())
}

fn validate_reduce_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "reduce", &["input", "accumulator", "initial"])?;
    required_slot(body, index, "input")?;
    required_slot(body, index, "accumulator")?;
    let initial = required_step_field(body, index, "initial")?;
    slot_value(initial, index)?;
    Ok(())
}

fn validate_repeat_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "repeat", &["max_attempts"])?;
    required_u16_field(body, index, "repeat", "max_attempts")?;
    Ok(())
}

fn validate_finish_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    if index != last_step {
        return Err(CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        });
    }
    reject_unknown_primitive_fields(body, index, "finish", &["result"])?;
    required_step_field(body, index, "result")?;
    Ok(())
}

fn validate_phase_zero_result(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("result") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "result",
        expected: "a mapping",
    })?;
    if mapping.is_empty() {
        Ok(())
    } else {
        Err(CompileError::UnsupportedTopLevelResult)
    }
}

fn validate_optional_top_level_shapes(doc: &Yaml<'_>) -> Result<(), CompileError> {
    optional_inputs_mapping(doc)?;
    optional_vars_mapping(doc)?;
    optional_secret_mapping(doc)?;
    optional_examples_sequence(doc)
}

fn optional_inputs_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("inputs") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "inputs",
        expected: "a mapping",
    })?;
    for (key, _) in mapping {
        let Some(name) = key.as_str() else {
            return Err(non_string_key_error());
        };
        validate_public_name("inputs", name)?;
    }
    Ok(())
}

fn optional_vars_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("vars") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "vars",
        expected: "a mapping",
    })?;
    for (key, value) in mapping {
        let Some(name) = key.as_str() else {
            return Err(non_string_key_error());
        };
        validate_public_name("vars", name)?;
        slot_value(value, 0)?;
    }
    Ok(())
}

fn optional_secret_mapping(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("secrets") else {
        return Ok(());
    };
    let mapping = node.as_mapping().ok_or(CompileError::FieldShape {
        field: "secrets",
        expected: "a mapping",
    })?;
    for (key, value) in mapping {
        let Some(name) = key.as_str() else {
            return Err(non_string_key_error());
        };
        validate_public_name("secrets", name)?;
        if value.as_str().is_none() {
            return Err(CompileError::FieldShape {
                field: "secrets",
                expected: "a mapping of secret names to environment variable names",
            });
        }
    }
    Ok(())
}

fn optional_examples_sequence(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(node) = doc.as_mapping_get("examples") else {
        return Ok(());
    };
    let examples = node.as_sequence().ok_or(CompileError::FieldShape {
        field: "examples",
        expected: "a sequence",
    })?;
    for example in examples {
        if !example.is_mapping() {
            return Err(CompileError::FieldShape {
                field: "examples",
                expected: "a sequence of mappings",
            });
        }
        let name = required_example_name(example)?;
        validate_public_name("examples", name)?;
    }
    Ok(())
}

fn required_example_name<'a>(example: &'a Yaml<'a>) -> Result<&'a str, CompileError> {
    let name = example
        .as_mapping_get("name")
        .ok_or(CompileError::MissingField {
            field: "examples.name",
        })?;
    name.as_str().ok_or(CompileError::FieldShape {
        field: "examples.name",
        expected: "a string",
    })
}

fn validate_step_ids(steps: &saphyr::Sequence<'_>) -> Result<(), CompileError> {
    let mut seen = HashSet::with_capacity(steps.len());
    for (index, step) in steps.iter().enumerate() {
        let id = required_step_id(step, index)?;
        validate_public_name("step id", id)?;
        if !seen.insert(id) {
            return Err(CompileError::DuplicateStepId {
                id: Box::<str>::from(id),
            });
        }
    }
    Ok(())
}

fn required_step_id<'a>(step: &'a Yaml<'a>, index: usize) -> Result<&'a str, CompileError> {
    if !step.is_mapping() {
        return Err(CompileError::StepShape { step: index });
    }
    let node = step
        .as_mapping_get("id")
        .ok_or(CompileError::MissingStepId { step: index })?;
    node.as_str().ok_or(CompileError::StepFieldShape {
        step: index,
        field: "id",
        expected: "a string",
    })
}

pub(crate) fn validate_public_name(field: &'static str, value: &str) -> Result<(), CompileError> {
    if is_public_name(value) {
        Ok(())
    } else {
        Err(CompileError::InvalidName {
            field,
            value: Box::<str>::from(value),
        })
    }
}

fn is_public_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    value.len() <= 64
        && first.is_ascii_lowercase()
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
        && !is_reserved_name(value)
}

const RESERVED_NAMES: &[&str] = &[
    "input",
    "inputs",
    "vars",
    "secrets",
    "steps",
    "result",
    "when",
    "item",
    "error",
    "summary",
    "cursor",
    "page",
    "event",
    "attempt",
    "attempts",
    "true",
    "false",
    "null",
    "run",
    "do",
    "set",
    "save",
    "choose",
    "for_each",
    "together",
    "collect",
    "reduce",
    "repeat",
    "wait",
    "ask",
    "try_again",
    "on_error",
    "then",
    "finish",
];

fn is_reserved_name(value: &str) -> bool {
    RESERVED_NAMES.contains(&value)
}

fn validate_top_level_keys(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(mapping) = doc.as_mapping() else {
        return Err(CompileError::TopLevelNotMapping);
    };
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            return Err(non_string_key_error());
        };
        if !is_top_level_field(field) {
            return Err(CompileError::UnknownTopLevelField {
                field: Box::<str>::from(field),
            });
        }
    }
    Ok(())
}

fn is_top_level_field(field: &str) -> bool {
    matches!(
        field,
        "version"
            | "name"
            | "when"
            | "steps"
            | "inputs"
            | "vars"
            | "secrets"
            | "result"
            | "examples"
    )
}

fn validate_workflow_version(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let version = required_string_field(doc, "version")?;
    if version == WORKFLOW_VERSION {
        Ok(())
    } else {
        Err(CompileError::InvalidVersion {
            actual: Box::<str>::from(version),
        })
    }
}

fn validate_workflow_trigger(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let triggers = required_mapping_field(doc, "when")?;
    if triggers.len() != 1 {
        return Err(CompileError::InvalidTriggerCount {
            count: triggers.len(),
        });
    }
    let Some((key, value)) = triggers.iter().next() else {
        return Err(CompileError::InvalidTriggerCount { count: 0 });
    };
    let Some(trigger) = key.as_str() else {
        return Err(non_string_key_error());
    };
    match trigger {
        "manual" => validate_manual_trigger(value),
        "webhook" => validate_webhook_trigger(value),
        "schedule" => validate_schedule_trigger(value),
        "event" => validate_event_trigger(value),
        value => Err(CompileError::UnknownTriggerKind {
            trigger: Box::<str>::from(value),
        }),
    }
}

fn validate_manual_trigger(node: &Yaml<'_>) -> Result<(), CompileError> {
    let mapping = trigger_mapping("manual", node)?;
    reject_unknown_trigger_fields("manual", mapping, &[])
}

fn validate_webhook_trigger(node: &Yaml<'_>) -> Result<(), CompileError> {
    let mapping = trigger_mapping("webhook", node)?;
    reject_unknown_trigger_fields("webhook", mapping, &["path", "method", "unique"])?;
    let path = required_trigger_string_field(node, "webhook", "path")?;
    if !path.starts_with('/') {
        return Err(CompileError::InvalidTriggerField {
            trigger: "webhook",
            field: "path",
            expected: "a string starting with /",
        });
    }
    let method = required_trigger_string_field(node, "webhook", "method")?;
    if !is_webhook_method(method) {
        return Err(CompileError::InvalidTriggerField {
            trigger: "webhook",
            field: "method",
            expected: "one of GET, POST, PUT, PATCH, DELETE",
        });
    }
    optional_trigger_string_field(node, "webhook", "unique")
}

fn validate_schedule_trigger(node: &Yaml<'_>) -> Result<(), CompileError> {
    let mapping = trigger_mapping("schedule", node)?;
    reject_unknown_trigger_fields("schedule", mapping, &["cron", "timezone"])?;
    let cron = required_trigger_string_field(node, "schedule", "cron")?;
    if cron.split_whitespace().count() != 5 {
        return Err(CompileError::InvalidTriggerField {
            trigger: "schedule",
            field: "cron",
            expected: "a five-field cron expression",
        });
    }
    optional_trigger_string_field(node, "schedule", "timezone")
}

fn validate_event_trigger(node: &Yaml<'_>) -> Result<(), CompileError> {
    let mapping = trigger_mapping("event", node)?;
    reject_unknown_trigger_fields("event", mapping, &["name"])?;
    required_trigger_string_field(node, "event", "name").map(|_| ())
}

fn trigger_mapping<'a>(
    trigger: &str,
    node: &'a Yaml<'a>,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    node.as_mapping().ok_or_else(|| CompileError::TriggerShape {
        trigger: Box::<str>::from(trigger),
        expected: "a mapping",
    })
}

fn reject_unknown_trigger_fields(
    trigger: &'static str,
    mapping: &saphyr::Mapping<'_>,
    allowed: &[&str],
) -> Result<(), CompileError> {
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            return Err(non_string_key_error());
        };
        if !allowed.contains(&field) {
            return Err(CompileError::UnknownTriggerField {
                trigger,
                field: Box::<str>::from(field),
            });
        }
    }
    Ok(())
}

fn required_trigger_string_field<'a>(
    node: &'a Yaml<'a>,
    trigger: &'static str,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    let value = node
        .as_mapping_get(field)
        .ok_or(CompileError::MissingTriggerField { trigger, field })?;
    value.as_str().ok_or(CompileError::InvalidTriggerField {
        trigger,
        field,
        expected: "a string",
    })
}

fn optional_trigger_string_field(
    node: &Yaml<'_>,
    trigger: &'static str,
    field: &'static str,
) -> Result<(), CompileError> {
    match node.as_mapping_get(field) {
        Some(value) if value.as_str().is_none() => Err(CompileError::InvalidTriggerField {
            trigger,
            field,
            expected: "a string",
        }),
        _ => Ok(()),
    }
}

fn is_webhook_method(method: &str) -> bool {
    matches!(method, "GET" | "POST" | "PUT" | "PATCH" | "DELETE")
}

fn required_string_field<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    let node = doc
        .as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?;
    node.as_str().ok_or(CompileError::FieldShape {
        field,
        expected: "a string",
    })
}

fn required_sequence_field<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Sequence<'a>, CompileError> {
    let node = doc
        .as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?;
    node.as_sequence().ok_or(CompileError::FieldShape {
        field,
        expected: "a sequence",
    })
}

fn required_mapping_field<'a>(
    doc: &'a Yaml<'a>,
    field: &'static str,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    let node = doc
        .as_mapping_get(field)
        .ok_or(CompileError::MissingField { field })?;
    node.as_mapping().ok_or(CompileError::FieldShape {
        field,
        expected: "a mapping",
    })
}

#[derive(Debug, Default)]
#[allow(dead_code)]
struct WorkflowBuilder {
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    max_slot: Option<usize>,
}

impl WorkflowBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn push_constant(&mut self, value: ConstValue) -> Result<ConstIdx, CompileError> {
        let index = u16::try_from(self.constants.len()).map_err(|_| {
            CompileError::Workflow(WorkflowError::ConstOutOfBounds {
                constant: ConstIdx::new(u16::MAX),
            })
        })?;
        self.constants.push(value);
        Ok(ConstIdx::new(index))
    }

    fn record_slot(&mut self, slot: SlotIdx) {
        let value = slot.as_usize();
        self.max_slot = Some(match self.max_slot {
            Some(current) => current.max(value),
            None => value,
        });
    }

    fn slot_count(&self) -> Result<u16, CompileError> {
        match self.max_slot {
            Some(value) => {
                let count = value
                    .checked_add(1)
                    .ok_or(CompileError::SlotIndexOutOfRange { value: i64::MAX })?;
                u16::try_from(count).map_err(|_| CompileError::SlotIndexOutOfRange {
                    value: i64::from(u16::MAX),
                })
            }
            None => Ok(0),
        }
    }
}

#[allow(dead_code)]
fn compile_step(
    step: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    source_ir_starts: &[StepIdx],
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    let StepSpec { primitive, body } = step_spec(step, index)?;
    let node = match primitive {
        StepPrimitive::Run | StepPrimitive::Do => compile_run(
            body,
            index,
            last_step,
            id,
            next,
            primitive.as_str(),
            builder,
        ),
        StepPrimitive::Set | StepPrimitive::Save => compile_save(
            body,
            index,
            last_step,
            id,
            next,
            primitive.as_str(),
            builder,
        ),
        StepPrimitive::Choose => {
            compile_choose(body, index, last_step, id, source_ir_starts, builder)
        }
        StepPrimitive::ForEach => return compile_for_each(body, index, last_step, id, builder),
        StepPrimitive::Together => {
            return compile_together(body, index, last_step, id, source_ir_starts, builder);
        }
        StepPrimitive::Collect => {
            return compile_collect(body, index, last_step, id, next, builder);
        }
        StepPrimitive::Reduce => {
            return compile_reduce(body, index, last_step, id, next, builder);
        }
        StepPrimitive::Repeat => {
            return compile_repeat(body, index, last_step, id, next, builder);
        }
        StepPrimitive::Wait => compile_wait(body, index, last_step, id, next, builder),
        StepPrimitive::Ask => return compile_ask(body, index, last_step, id, next, builder),
        StepPrimitive::Finish => return compile_finish(body, index, last_step, id, builder),
    }?;
    Ok(vec![node])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepPrimitive {
    Set,
    Run,
    Do,
    Save,
    Choose,
    ForEach,
    Together,
    Collect,
    Reduce,
    Repeat,
    Wait,
    Ask,
    Finish,
}

impl StepPrimitive {
    fn from_field(field: &str) -> Option<Self> {
        match field {
            "set" => Some(Self::Set),
            "run" => Some(Self::Run),
            "do" => Some(Self::Do),
            "save" => Some(Self::Save),
            "choose" => Some(Self::Choose),
            "for_each" => Some(Self::ForEach),
            "together" => Some(Self::Together),
            "collect" => Some(Self::Collect),
            "reduce" => Some(Self::Reduce),
            "repeat" => Some(Self::Repeat),
            "wait" => Some(Self::Wait),
            "ask" => Some(Self::Ask),
            "finish" => Some(Self::Finish),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Run => "run",
            Self::Do => "do",
            Self::Save => "save",
            Self::Choose => "choose",
            Self::ForEach => "for_each",
            Self::Together => "together",
            Self::Collect => "collect",
            Self::Reduce => "reduce",
            Self::Repeat => "repeat",
            Self::Wait => "wait",
            Self::Ask => "ask",
            Self::Finish => "finish",
        }
    }
}

#[allow(dead_code)]
fn compile_run(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    primitive: &'static str,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, primitive, &["action", "input"])?;
    let action = required_action(body, index, primitive)?;
    let input = required_slot(body, index, "input")?;
    let output = slot_idx_for_step(index)?;
    builder.record_slot(input);
    builder.record_slot(output);
    Ok(lower_do(
        id,
        action,
        input,
        Some(output),
        Some(required_next_step(next, index)?),
        &mut SlotCompiler::new(),
    ))
}

#[derive(Debug, Clone, Copy)]
struct StepSpec<'a> {
    primitive: StepPrimitive,
    body: &'a Yaml<'a>,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum ChooseCondition {
    Slot(SlotIdx),
    Literal(bool),
}

fn step_spec<'a>(step: &'a Yaml<'a>, index: usize) -> Result<StepSpec<'a>, CompileError> {
    let Some(mapping) = step.as_mapping() else {
        return Err(CompileError::StepShape { step: index });
    };
    let mut selected = None;

    for (key, body) in mapping {
        let Some(field) = key.as_str() else {
            return Err(CompileError::StepShape { step: index });
        };
        if let Some(primitive) = StepPrimitive::from_field(field) {
            if selected.is_some() {
                return Err(CompileError::MultipleStepPrimitives { step: index });
            }
            selected = Some(StepSpec { primitive, body });
        } else {
            validate_phase_zero_step_metadata(field, body, index)?;
        }
    }

    selected.ok_or(CompileError::MissingStepPrimitive { step: index })
}

fn validate_phase_zero_step_metadata(
    field: &str,
    body: &Yaml<'_>,
    step: usize,
) -> Result<(), CompileError> {
    match field {
        "id" => Ok(()),
        "name" => validate_step_display_name(body, step),
        "if" | "with" | "try_again" | "on_error" | "then" => {
            Err(CompileError::UnsupportedStepControlField {
                step,
                field: Box::<str>::from(field),
            })
        }
        _ => Err(CompileError::UnknownStepField {
            step,
            field: Box::<str>::from(field),
        }),
    }
}

fn validate_step_display_name(body: &Yaml<'_>, step: usize) -> Result<(), CompileError> {
    if body.as_str().is_some() {
        Ok(())
    } else {
        Err(CompileError::StepFieldShape {
            step,
            field: "name",
            expected: "a string",
        })
    }
}

#[allow(dead_code)]
fn compile_save(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    primitive: &'static str,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_non_mapping_step_body(body, index, primitive, "an object")?;
    let output = slot_idx_for_step(index)?;
    let constant = save_slot_value(body, index, primitive)?;
    let constant = builder.push_constant(constant)?;
    builder.record_slot(output);
    set_const_node(id, output, constant, required_next_step(next, index)?)
}

#[allow(dead_code)]
fn reject_non_mapping_step_body(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
    expected: &'static str,
) -> Result<(), CompileError> {
    if body.is_mapping() {
        Ok(())
    } else {
        Err(CompileError::StepFieldShape {
            step,
            field,
            expected,
        })
    }
}

#[allow(clippy::unnecessary_wraps)]
#[allow(dead_code)]
fn set_const_node(
    id: StepIdx,
    output: SlotIdx,
    value: ConstIdx,
    next: StepIdx,
) -> Result<CompiledNode, CompileError> {
    Ok(CompiledNode {
        id,
        output: Some(output),
        next: Some(next),
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::SetConst { value },
    })
}

#[allow(dead_code)]
fn save_slot_value(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
) -> Result<ConstValue, CompileError> {
    let Some(mapping) = body.as_mapping() else {
        return Err(CompileError::StepFieldShape {
            step,
            field: primitive,
            expected: "an object",
        });
    };
    if mapping.len() != 1 {
        return Err(CompileError::UnsupportedConstantValue { step });
    }
    match mapping.iter().next() {
        Some((key, value)) if key.as_str() == Some("value") => slot_value(value, step),
        Some((key, _)) if key.as_str().is_none() => Err(non_string_key_error()),
        Some(_) | None => Err(CompileError::UnsupportedConstantValue { step }),
    }
}

#[allow(dead_code)]
fn compile_choose(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    source_ir_starts: &[StepIdx],
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "choose", &["condition", "on_true", "on_false"])?;
    let condition = required_choose_condition(body, index)?;
    let on_true = mapped_branch_target(body, index, "on_true", source_ir_starts)?;
    let on_false = mapped_branch_target(body, index, "on_false", source_ir_starts)?;
    match condition {
        ChooseCondition::Slot(condition) => {
            compile_slot_choose(id, condition, on_true, on_false, builder)
        }
        ChooseCondition::Literal(value) => {
            compile_literal_choose(index, id, value, on_true, on_false, builder)
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
#[allow(dead_code)]
fn compile_slot_choose(
    id: StepIdx,
    condition: SlotIdx,
    on_true: StepIdx,
    on_false: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    builder.record_slot(condition);
    Ok(CompiledNode {
        id,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::ChooseSlot {
            branches: vec![SlotBranch {
                condition,
                target: on_true,
            }]
            .into_boxed_slice(),
            otherwise: Some(on_false),
        },
    })
}

#[allow(dead_code)]
fn compile_literal_choose(
    index: usize,
    id: StepIdx,
    value: bool,
    on_true: StepIdx,
    on_false: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    let output = slot_idx_for_step(index)?;
    let constant = builder.push_constant(ConstValue::Bool(value))?;
    builder.record_slot(output);
    Ok(CompiledNode {
        id,
        output: Some(output),
        next: Some(if value { on_true } else { on_false }),
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::SetConst { value: constant },
    })
}

#[allow(dead_code)]
fn compile_for_each(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unsupported_for_each_fields(body, index)?;
    reject_unknown_primitive_fields(body, index, "for_each", &["input", "item", "limit"])?;
    let input = required_slot(body, index, "input")?;
    let item = required_slot(body, index, "item")?;
    let limit = required_u32_field(body, index, "for_each", "limit")?;
    let body_step = checked_step_offset(id, 1, "for_each", "body")?;
    let done = checked_step_offset(id, 2, "for_each", "done")?;
    builder.record_slot(input);
    builder.record_slot(item);
    lower_for_each(
        id,
        input,
        item,
        limit,
        body_step,
        done,
        &mut SlotCompiler::new(),
    )
}

#[allow(dead_code)]
fn compile_together(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    source_ir_starts: &[StepIdx],
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "together", &["branches"])?;
    let branch_sources = required_branch_targets(body, index, "branches")?;
    let mut branches = Vec::with_capacity(branch_sources.len());
    for source in branch_sources {
        branches.push(source_ir_start(source_ir_starts, source.as_usize())?);
    }
    let branch_count = u16::try_from(branches.len()).map_err(|_| {
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "together",
            field: "branches",
            value: branches.len(),
            limit: usize::from(u16::MAX),
        }
    })?;
    let accumulator = alloc_workflow_slot(builder)?;
    let join = checked_step_offset(id, 1, "together", "join")?;
    Ok(vec![
        CompiledNode {
            id,
            output: Some(accumulator),
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: branches.into_boxed_slice(),
                join,
            },
        },
        CompiledNode {
            id: join,
            output: Some(accumulator),
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count,
                accumulator,
            },
        },
    ])
}

#[allow(dead_code)]
fn compile_collect(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "collect", &["source", "limit", "page_size"])?;
    let source = required_slot(body, index, "source")?;
    let limit = required_u32_field(body, index, "collect", "limit")?;
    let page_size = required_u32_field(body, index, "collect", "page_size")?;
    let body_step = checked_step_offset(id, 1, "collect", "body")?;
    let done = checked_step_offset(id, 2, "collect", "done")?;
    builder.record_slot(source);
    let mut nodes = lower_collect(
        id,
        source,
        limit,
        page_size,
        body_step,
        done,
        &mut SlotCompiler::new(),
    )?;
    // CollectFinish (index 2) chains to the next step.
    if let Some(finish) = nodes.get_mut(2) {
        finish.next = next;
    }
    Ok(nodes)
}

#[allow(dead_code)]
fn compile_reduce(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "reduce", &["input", "accumulator", "initial"])?;
    let input = required_slot(body, index, "input")?;
    let accumulator = required_slot(body, index, "accumulator")?;
    let initial = slot_value(required_step_field(body, index, "initial")?, index)?;
    let initial = builder.push_constant(initial)?;
    let body_step = checked_step_offset(id, 1, "reduce", "body")?;
    let done = checked_step_offset(id, 2, "reduce", "done")?;
    builder.record_slot(input);
    builder.record_slot(accumulator);
    let mut nodes = lower_reduce(
        id,
        input,
        accumulator,
        initial,
        body_step,
        done,
        &mut SlotCompiler::new(),
    )?;
    // ReduceFinish (index 2) chains to the next step.
    if let Some(finish) = nodes.get_mut(2) {
        finish.next = next;
    }
    Ok(nodes)
}

#[allow(dead_code)]
fn compile_repeat(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "repeat", &["max_attempts"])?;
    let max_attempts = required_u16_field(body, index, "repeat", "max_attempts")?;
    let body_step = checked_step_offset(id, 1, "repeat", "body")?;
    let done = checked_step_offset(id, 2, "repeat", "done")?;
    let attempt_slot = slot_idx_for_step(id.as_usize().checked_add(1).ok_or({
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "repeat",
            field: "attempt_slot",
            value: id.as_usize(),
            limit: usize::from(u16::MAX),
        }
    })?)?;
    builder.record_slot(attempt_slot);
    let mut nodes = lower_repeat(id, max_attempts, body_step, done, &mut SlotCompiler::new())?;
    // RepeatFinish (index 2) chains to the next step.
    if let Some(finish) = nodes.get_mut(2) {
        finish.next = next;
    }
    Ok(nodes)
}

#[allow(dead_code)]
fn compile_wait(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "wait", &["until", "event", "timeout"])?;
    let until = optional_slot_field(body, index, "until")?;
    let event = optional_slot_field(body, index, "event")?;
    let timeout = optional_slot_field(body, index, "timeout")?;
    let mut node = match (until, event, timeout) {
        (Some(deadline), None, None) => {
            builder.record_slot(deadline);
            lower_wait(id, WaitKind::Until { deadline }, &mut SlotCompiler::new())
        }
        (None, Some(event_slot), timeout_slot) => {
            builder.record_slot(event_slot);
            if let Some(slot) = timeout_slot {
                builder.record_slot(slot);
            }
            lower_wait(
                id,
                WaitKind::Event {
                    event: event_slot,
                    timeout: timeout_slot,
                },
                &mut SlotCompiler::new(),
            )
        }
        _ => {
            return Err(CompileError::StepFieldShape {
                step: index,
                field: "wait",
                expected: "until without timeout or event with optional timeout",
            });
        }
    };
    node.next = next;
    Ok(node)
}

#[allow(dead_code)]
fn compile_ask(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "ask", &["prompt", "answer", "timeout"])?;
    let prompt = required_slot(body, index, "prompt")?;
    let answer = required_slot(body, index, "answer")?;
    let timeout = optional_slot_field(body, index, "timeout")?;
    builder.record_slot(prompt);
    builder.record_slot(answer);
    if let Some(slot) = timeout {
        builder.record_slot(slot);
    }
    let mut nodes = lower_ask(id, prompt, answer, timeout, &mut SlotCompiler::new())?;
    // Ask (index 0) chains to AskResume for structural reachability.
    if let (Some(_ask_node), Some(resume_node)) = (nodes.first(), nodes.get(1)) {
        let resume_id = resume_node.id;
        if let Some(ask_node) = nodes.first_mut() {
            ask_node.next = Some(resume_id);
        }
    }
    // AskResume (index 1) chains to the next step.
    if let Some(resume) = nodes.get_mut(1) {
        resume.next = next;
    }
    Ok(nodes)
}

#[allow(dead_code)]
fn compile_finish(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    if index != last_step {
        return Err(CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        });
    }
    reject_unknown_primitive_fields(body, index, "finish", &["result"])?;
    let result = required_step_field(body, index, "result")?;
    compile_finish_result(result, index, id, builder)
}

#[allow(dead_code)]
fn compile_finish_result(
    result: &Yaml<'_>,
    index: usize,
    id: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    if let Some(slot) = finish_result_slot(result, index)? {
        builder.record_slot(slot);
        return Ok(vec![CompiledNode {
            id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Finish { result: slot },
        }]);
    }
    let value = slot_value(result, index)?;
    let value = builder.push_constant(value)?;
    let output = slot_idx_for_step(index)?;
    builder.record_slot(output);
    let finish_id = id.checked_add(1).ok_or(CompileError::StepIndexOutOfRange {
        value: id.as_usize(),
    })?;
    Ok(vec![
        CompiledNode {
            id,
            output: Some(output),
            next: Some(finish_id),
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::SetConst { value },
        },
        CompiledNode {
            id: finish_id,
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Finish { result: output },
        },
    ])
}

#[allow(dead_code)]
fn finish_result_slot(result: &Yaml<'_>, index: usize) -> Result<Option<SlotIdx>, CompileError> {
    let Some(value) = result.as_integer() else {
        return Ok(None);
    };
    if !finish_integer_is_slot(value, index) {
        return Ok(None);
    }
    let value = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(Some(SlotIdx::new(value)))
}

#[allow(dead_code)]
fn finish_integer_is_slot(value: i64, index: usize) -> bool {
    match usize::try_from(value) {
        Ok(slot) => slot <= index,
        Err(_) => false,
    }
}

fn reject_last_non_finish(index: usize, last_step: usize) -> Result<(), CompileError> {
    if index == last_step {
        Err(CompileError::LastStepMustFinish)
    } else {
        Ok(())
    }
}

fn step_idx(value: usize) -> Result<StepIdx, CompileError> {
    let value = u16::try_from(value).map_err(|_| CompileError::StepIndexOutOfRange { value })?;
    Ok(StepIdx::new(value))
}

fn slot_idx_for_step(value: usize) -> Result<SlotIdx, CompileError> {
    let value = u16::try_from(value).map_err(|_| CompileError::StepIndexOutOfRange { value })?;
    Ok(SlotIdx::new(value))
}

fn required_step_field<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    field: &'static str,
) -> Result<&'a Yaml<'a>, CompileError> {
    body.as_mapping_get(field)
        .ok_or(CompileError::MissingStepField { step, field })
}

fn optional_slot_field(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<Option<SlotIdx>, CompileError> {
    match body.as_mapping_get(field) {
        Some(_) => required_slot(body, step, field).map(Some),
        None => Ok(None),
    }
}

#[allow(dead_code)]
fn required_next_step(next: Option<StepIdx>, index: usize) -> Result<StepIdx, CompileError> {
    next.ok_or(CompileError::StepIndexOutOfRange { value: index })
}

#[allow(dead_code)]
fn mapped_branch_target(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
    source_ir_starts: &[StepIdx],
) -> Result<StepIdx, CompileError> {
    let source = required_branch_target(body, step, field)?;
    source_ir_start(source_ir_starts, source.as_usize())
}

fn reject_unknown_primitive_fields(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    allowed: &[&str],
) -> Result<(), CompileError> {
    let mapping = primitive_body_mapping(body, step, primitive)?;
    for (key, _) in mapping {
        reject_unknown_primitive_field(key, step, primitive, allowed)?;
    }
    Ok(())
}

fn primitive_body_mapping<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    primitive: &'static str,
) -> Result<&'a saphyr::Mapping<'a>, CompileError> {
    body.as_mapping().ok_or(CompileError::StepFieldShape {
        step,
        field: primitive,
        expected: "a mapping",
    })
}

fn reject_unknown_primitive_field(
    key: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    allowed: &[&str],
) -> Result<(), CompileError> {
    let Some(field) = key.as_str() else {
        return Err(CompileError::StepShape { step });
    };
    if allowed.contains(&field) {
        Ok(())
    } else {
        Err(CompileError::UnknownStepPrimitiveField {
            step,
            primitive,
            field: Box::<str>::from(field),
        })
    }
}

fn required_slot(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<SlotIdx, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "an integer slot index",
    })?;
    let value = u16::try_from(value).map_err(|_| CompileError::SlotIndexOutOfRange { value })?;
    Ok(SlotIdx::new(value))
}

fn required_u32_field(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    field: &'static str,
) -> Result<u32, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a non-negative u32 integer",
    })?;
    u32::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field,
        value: integer_error_value(value),
        limit: usize::try_from(u32::MAX).map_or(usize::MAX, |limit| limit),
    })
}

fn required_u16_field(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
    field: &'static str,
) -> Result<u16, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a non-negative u16 integer",
    })?;
    u16::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field,
        value: integer_error_value(value),
        limit: usize::from(u16::MAX),
    })
}

fn integer_error_value(value: i64) -> usize {
    match usize::try_from(value) {
        Ok(value) => value,
        Err(_) => usize::MAX,
    }
}

fn required_branch_targets(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<Vec<StepIdx>, CompileError> {
    let node = required_step_field(body, step, field)?;
    let sequence = node.as_sequence().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "a sequence of integer step indexes",
    })?;
    if sequence.is_empty() {
        return Err(CompileError::StepFieldShape {
            step,
            field,
            expected: "at least one integer step index",
        });
    }
    let mut targets = Vec::with_capacity(sequence.len());
    let mut index = 0usize;
    while index < sequence.len() {
        let Some(node) = sequence.get(index) else {
            return Err(CompileError::StepIndexOutOfRange { value: index });
        };
        let value = node.as_integer().ok_or(CompileError::StepFieldShape {
            step,
            field,
            expected: "a sequence of integer step indexes",
        })?;
        let value =
            u16::try_from(value).map_err(|_| CompileError::BranchTargetOutOfRange { value })?;
        targets.push(StepIdx::new(value));
        index = index
            .checked_add(1)
            .ok_or(CompileError::StepIndexOutOfRange { value: index })?;
    }
    Ok(targets)
}

#[allow(dead_code)]
fn checked_step_offset(
    id: StepIdx,
    offset: u16,
    primitive: &'static str,
    field: &'static str,
) -> Result<StepIdx, CompileError> {
    id.checked_add(offset)
        .ok_or(CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field,
            value: id.as_usize(),
            limit: usize::from(u16::MAX),
        })
}

#[allow(dead_code)]
fn alloc_workflow_slot(builder: &mut WorkflowBuilder) -> Result<SlotIdx, CompileError> {
    let value = builder.slot_count()?;
    let slot = SlotIdx::new(value);
    builder.record_slot(slot);
    Ok(slot)
}

fn required_action(
    body: &Yaml<'_>,
    step: usize,
    primitive: &'static str,
) -> Result<vb_core::ActionId, CompileError> {
    let node = required_step_field(body, step, "action")?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field: "action",
        expected: "an integer action id",
    })?;
    let raw = u16::try_from(value).map_err(|_| CompileError::PrimitiveLoweringLimitExceeded {
        primitive,
        field: "action",
        value: integer_error_value(value),
        limit: usize::from(u16::MAX),
    })?;
    Ok(vb_core::ActionId::new(raw))
}

#[allow(dead_code)]
fn required_choose_condition(
    body: &Yaml<'_>,
    step: usize,
) -> Result<ChooseCondition, CompileError> {
    let node = required_step_field(body, step, "condition")?;
    if let Some(value) = node.as_bool() {
        return Ok(ChooseCondition::Literal(value));
    }
    required_slot(body, step, "condition").map(ChooseCondition::Slot)
}

fn required_branch_target(
    body: &Yaml<'_>,
    step: usize,
    field: &'static str,
) -> Result<StepIdx, CompileError> {
    let node = required_step_field(body, step, field)?;
    let value = node.as_integer().ok_or(CompileError::StepFieldShape {
        step,
        field,
        expected: "an integer step index",
    })?;
    let value = u16::try_from(value).map_err(|_| CompileError::BranchTargetOutOfRange { value })?;
    Ok(StepIdx::new(value))
}

fn slot_value(node: &Yaml<'_>, step: usize) -> Result<ConstValue, CompileError> {
    match node {
        Yaml::Value(saphyr::Scalar::Null) => Ok(ConstValue::Null),
        Yaml::Value(saphyr::Scalar::Boolean(value)) => Ok(ConstValue::Bool(*value)),
        Yaml::Value(saphyr::Scalar::Integer(value)) => Ok(ConstValue::I64(*value)),
        Yaml::Value(saphyr::Scalar::String(value)) | Yaml::Representation(value, _, None) => {
            text_slot_value(value.as_ref(), step)
        }
        Yaml::Sequence(sequence) => list_slot_value(sequence, step),
        Yaml::Mapping(mapping) => object_slot_value(mapping, step),
        _ => Err(CompileError::UnsupportedConstantValue { step }),
    }
}

fn text_slot_value(_value: &str, step: usize) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

fn list_slot_value(
    _sequence: &saphyr::Sequence<'_>,
    step: usize,
) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

fn object_slot_value(
    _mapping: &saphyr::Mapping<'_>,
    step: usize,
) -> Result<ConstValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_parts_for_lower(
        nodes: Vec<CompiledNode>,
        expressions: Vec<ExprProgram>,
        slot_count: u16,
    ) -> WorkflowParts {
        WorkflowParts {
            name: Box::from("test_lower_steps"),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: expressions.into_boxed_slice(),
            accessors: Box::new([]),
            constants: Box::new([ConstValue::I64(0)]),
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

    fn do_node(index: u16, action: ActionId, input: SlotIdx) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do { action, input },
        }
    }

    fn public_compile_source() -> &'static [u8] {
        br#"version: velvet-ballastics/v1
name: adapter_case
when:
  manual: {}
steps:
  - id: set_value
    set:
      output: answer
      value: "1"
  - id: done
    finish:
      result: answer
"#
    }

    fn first_error(result: Result<CompiledWorkflow, CompileErrors>) -> CompileError {
        match result {
            Ok(_) => panic!("expected compile error"),
            Err(errors) => match errors.0.into_iter().next() {
                Some(error) => error,
                None => panic!("expected at least one compile error"),
            },
        }
    }

    fn canonical_named_source(trigger: &str) -> String {
        format!(
            "version: velvet-ballastics/v1\nname: canonical_compile\nwhen:\n  {trigger}\nsteps:\n  - id: make\n    set:\n      output: answer\n      value: \"42\"\n  - id: done\n    finish:\n      result: answer\n"
        )
    }

    #[test]
    fn compile_source_lowers_named_finish_without_runtime_lookup() {
        let yaml = canonical_named_source("manual: {}");
        let source = vb_yaml::parse_workflow_source(&yaml).expect("canonical parse");
        let workflow = compile_source(&source).expect("canonical compile");
        let parts = workflow.to_parts();
        assert!(matches!(
            parts.nodes.first().map(|node| &node.kind),
            Some(CompiledNodeKind::SetConst { .. })
        ));
        assert!(
            matches!(parts.nodes.get(1).map(|node| &node.kind), Some(CompiledNodeKind::Finish { result }) if *result == SlotIdx::new(0))
        );
    }

    #[test]
    fn compile_source_rejects_duplicate_and_unknown_outputs() {
        let duplicate = "version: velvet-ballastics/v1\nname: dup\nwhen: { manual: {} }\nsteps:\n  - id: a\n    set: { output: answer, value: \"1\" }\n  - id: b\n    set: { output: answer, value: \"2\" }\n  - id: done\n    finish: { result: answer }\n";
        let source = vb_yaml::parse_workflow_source(duplicate).expect("canonical parse");
        assert!(matches!(
            first_error(compile_source(&source)),
            CompileError::DuplicateOutputName { .. }
        ));
        let unknown = "version: velvet-ballastics/v1\nname: unknown\nwhen: { manual: {} }\nsteps:\n  - id: a\n    set: { output: answer, value: \"1\" }\n  - id: done\n    finish: { result: missing }\n";
        let source = vb_yaml::parse_workflow_source(unknown).expect("canonical parse");
        assert!(matches!(
            first_error(compile_source(&source)),
            CompileError::UnknownOutputName { .. }
        ));
    }

    #[test]
    fn canonical_route_accepts_event_and_webhook_and_digest_changes() {
        let event = canonical_named_source("event: { type: invoice.created }");
        let webhook = canonical_named_source("webhook: {}");
        let event_workflow = compile_workflow(event.as_bytes()).expect("event compiles");
        let webhook_workflow = compile_workflow(webhook.as_bytes()).expect("webhook compiles");
        assert_ne!(
            event_workflow.to_parts().digest,
            webhook_workflow.to_parts().digest
        );
    }

    #[test]
    fn compile_rejects_legacy_numeric_save_without_fallback() {
        let yaml = br#"version: velvet-ballastics/v1
name: legacy_save
when:
  manual: {}
steps:
  - id: set_value
    save: { value: 1 }
  - id: done
    finish: { result: 0 }
"#;

        let error = first_error(compile_workflow(yaml));

        assert!(matches!(
            error,
            CompileError::CanonicalYaml {
                category: "missing_field",
                ..
            }
        ));
        assert_eq!(error.diagnostic_code(), "MISSING_REQUIRED_FIELD");
    }

    #[test]
    fn compile_source_rejects_unsupported_declarations_and_controls() {
        let yaml = "version: velvet-ballastics/v1\nname: controls\nwhen: { manual: {} }\ninputs: { x: 1 }\nsteps:\n  - id: a\n    if: ready\n    set: { output: answer, value: \"1\" }\n  - id: done\n    finish: { result: answer }\n";
        let source = vb_yaml::parse_workflow_source(yaml).expect("canonical parse");
        let errors = compile_source(&source).expect_err("unsupported scope rejects");
        assert!(errors.0.iter().any(|error| matches!(
            error,
            CompileError::UnsupportedTopLevelDeclaration { field: "inputs" }
        )));
        assert!(errors.0.iter().any(|error| matches!(error, CompileError::UnsupportedStepControlField { field, .. } if field.as_ref() == "if")));
    }

    #[test]
    fn test_existing_compile_api_returns_expected_artifact() {
        let workflow = compile_workflow(public_compile_source()).expect("valid workflow compiles");

        assert_eq!(workflow.name(), "adapter_case");
        assert_eq!(workflow.entry(), StepIdx::new(0));
        assert_eq!(workflow.slot_count(), 1);
        assert_eq!(workflow.node_count(), 2);

        let parts = workflow.to_parts();
        assert_eq!(parts.constants.as_ref(), &[ConstValue::I64(1)]);
        assert_eq!(vb_validate::shared::validate(&parts), Ok(()));
    }

    #[test]
    fn test_existing_invalid_input_returns_same_diagnostic_code() {
        let source = br#"version: velvet-ballastics/v1
name: adapter_case
unexpected: true
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

        let error = first_error(compile_workflow(source));

        assert_eq!(error.code(), "UNKNOWN_TOP_LEVEL_FIELD");
        assert_eq!(error.diagnostic_code(), "UNKNOWN_TOP_LEVEL_FIELD");
    }

    #[test]
    fn test_compile_invalid_input_matches_validate_diagnostic() {
        let validate_parts = make_parts_for_lower(
            vec![do_node(0, ActionId::new(7), SlotIdx::new(1))],
            vec![],
            1,
        );
        let lower_parts = validate_parts.clone();

        let validate_error = first_error(validate_ir(validate_parts));
        let lower_error = first_error(lower_steps_to_ir(
            lower_parts.nodes.into_vec(),
            lower_parts.expressions.into_vec(),
            lower_parts.accessors.into_vec(),
            lower_parts.constants.into_vec(),
            lower_parts.slot_count,
            lower_parts.symbols_count,
            &lower_parts.name,
            lower_parts.digest,
        ));

        assert_eq!(validate_error.code(), lower_error.code());
        assert_eq!(
            validate_error.diagnostic_code(),
            lower_error.diagnostic_code()
        );

        match (validate_error, lower_error) {
            (
                CompileError::Validation(vb_validate::ValidationError::SlotReferenceOutOfRange {
                    slot: validate_slot,
                    slot_count: validate_count,
                    context: validate_context,
                }),
                CompileError::Validation(vb_validate::ValidationError::SlotReferenceOutOfRange {
                    slot: lower_slot,
                    slot_count: lower_count,
                    context: lower_context,
                }),
            ) => {
                assert_eq!(validate_slot, lower_slot);
                assert_eq!(validate_count, lower_count);
                assert_eq!(validate_context, lower_context);
            }
            other => panic!("expected matching slot reference diagnostics, got {other:?}"),
        }
    }

    #[test]
    fn test_full_pipeline() {
        let workflow = compile_workflow(public_compile_source()).expect("valid workflow compiles");
        let artifact = emit_compiled_artifact(&workflow).expect("artifact encodes");
        let decoded_parts =
            postcard::from_bytes::<WorkflowParts>(&artifact).expect("artifact decodes");
        let validated = validate_ir(decoded_parts).expect("decoded artifact validates");

        assert_eq!(validated.name(), workflow.name());
        assert_eq!(validated.digest(), workflow.digest());
        assert_eq!(validated.node_count(), workflow.node_count());
    }

    /// RED-PHASE: lower_steps_to_ir bypasses Gate 9 (slot reference validation).
    #[test]
    fn lower_steps_to_ir_bypasses_gate_9_slot_reference_validation() {
        let nodes = vec![do_node(0, ActionId::new(7), SlotIdx::new(1))];
        let parts = make_parts_for_lower(nodes, vec![], 1);

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

        // Check that result is Err with exactly one ValidationError::SlotReferenceOutOfRange
        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::Validation(
                        vb_validate::ValidationError::SlotReferenceOutOfRange {
                            slot,
                            slot_count: sc,
                            context,
                        },
                    ) => {
                        assert_eq!(*slot, 1);
                        assert_eq!(*sc, 1);
                        assert!(
                            context.contains("node 0"),
                            "context should contain 'Do.input', got: {}",
                            context
                        );
                    }
                    other => panic!(
                        "Expected ValidationError::SlotReferenceOutOfRange, got: {:?}",
                        other
                    ),
                }
            }
            Ok(_) => panic!(
                "Expected error but lower_steps_to_ir succeeded. \
                 This FAILS before fix because lower_steps_to_ir bypasses Gate 9."
            ),
        }
    }

    /// RED-PHASE: validate_ir correctly orders shared validation BEFORE core.
    #[test]
    fn validate_ir_orders_shared_validation_before_core() {
        let nodes = vec![do_node(0, ActionId::new(7), SlotIdx::new(1))];
        let parts = make_parts_for_lower(nodes, vec![], 1);

        let result = validate_ir(parts);

        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::Validation(
                        vb_validate::ValidationError::SlotReferenceOutOfRange {
                            slot,
                            slot_count: sc,
                            context,
                        },
                    ) => {
                        assert_eq!(*slot, 1);
                        assert_eq!(*sc, 1);
                        assert!(
                            context.contains("node 0"),
                            "context should contain 'node 0', got: {}",
                            context
                        );
                    }
                    other => panic!(
                        "Expected ValidationError::SlotReferenceOutOfRange, got: {:?}",
                        other
                    ),
                }
            }
            Ok(_) => panic!("Expected error from validate_ir, got Ok"),
        }
    }

    /// RED-PHASE: lower_steps_to_ir output passes shared validation.
    #[test]
    fn lower_steps_to_ir_output_passes_shared_validation() {
        let nodes = vec![finish_node(0, 0)];
        let parts = make_parts_for_lower(nodes, vec![], 1);

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
            "lower_steps_to_ir should succeed for valid parts"
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

    /// RED-PHASE: validate_ir output passes shared validation.
    #[test]
    fn validate_ir_output_passes_shared_validation() {
        let nodes = vec![finish_node(0, 0)];
        let parts = make_parts_for_lower(nodes, vec![], 1);

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

    /// RED-PHASE: lower_steps_to_ir preserves WorkflowError for empty nodes.
    #[test]
    fn lower_steps_to_ir_returns_workflow_error_for_empty_nodes() {
        let parts = make_parts_for_lower(vec![], vec![], 0);

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

        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::Workflow(WorkflowError::EmptyNodes) => {}
                    other => panic!("Expected WorkflowError::EmptyNodes, got: {:?}", other),
                }
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    /// RED-PHASE: lower_steps_to_ir preserves WorkflowError for node ID mismatch.
    #[test]
    fn lower_steps_to_ir_returns_workflow_error_for_node_id_mismatch() {
        let mut node = do_node(1, ActionId::new(7), SlotIdx::new(0));
        node.id = StepIdx::new(1);
        let parts = make_parts_for_lower(vec![node], vec![], 1);

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
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::Workflow(WorkflowError::NodeIdMismatch { expected, actual }) => {
                        assert_eq!(expected.as_usize(), 0);
                        assert_eq!(actual.as_usize(), 1);
                    }
                    other => panic!("Expected WorkflowError::NodeIdMismatch, got: {:?}", other),
                }
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    /// RED-PHASE: validate_ir returns Workflow error when core fails after shared passes.
    #[test]
    fn validate_ir_returns_workflow_error_when_core_fails_after_shared_passes() {
        let parts = make_parts_for_lower(vec![], vec![], 0);

        let result = validate_ir(parts);

        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::Workflow(WorkflowError::EmptyNodes) => {}
                    other => panic!("Expected WorkflowError::EmptyNodes, got: {:?}", other),
                }
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    /// compile_workflow_with_contracts currently stops at canonical do lowering.
    #[test]
    fn compile_workflow_with_contracts_reports_unsupported_canonical_do_before_contracts() {
        let source = br#"version: velvet-ballastics/v1
name: test_do
when:
  manual: {}
steps:
  - id: seed
    set:
      output: request
      value: "1"
  - id: do_it
    do:
      action: call_service
      input: request
  - id: done
    finish:
      result: request
"#;

        let result = compile_workflow_with_contracts(source, &[]);

        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::UnsupportedStepPrimitive { step, primitive } => {
                        assert_eq!(*step, 1);
                        assert_eq!(*primitive, "do");
                    }
                    other => panic!(
                        "Expected CompileError::UnsupportedStepPrimitive for canonical do, got: {:?}",
                        other
                    ),
                }
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    /// compile_workflow_with_contracts applies contract validation on supported canonical compile.
    #[test]
    fn compile_workflow_with_contracts_rejects_orphan_action_contract() {
        let source = br#"version: velvet-ballastics/v1
name: test_no_do
when:
  manual: {}
steps:
  - id: seed
    set:
      output: answer
      value: "1"
  - id: done
    finish:
      result: answer
"#;

        let orphan_contract = ActionContract {
            id: ActionId::new(99),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };

        let result = compile_workflow_with_contracts(source, &[orphan_contract]);

        match result {
            Err(CompileErrors(ref errors)) => {
                assert_eq!(
                    errors.len(),
                    1,
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
                let err = errors.first().expect("should have first error");
                match err {
                    CompileError::Validation(
                        vb_validate::ValidationError::ActionContractOrphan { action_id },
                    ) => {
                        assert_eq!(*action_id, 99);
                    }
                    other => panic!(
                        "Expected ValidationError::ActionContractOrphan, got: {:?}",
                        other
                    ),
                }
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    /// RED-PHASE: plain vb_validate::shared::validate does NOT claim gate 12.
    #[test]
    fn plain_validate_does_not_claim_gate_12() {
        let nodes = vec![do_node(0, ActionId::new(7), SlotIdx::new(0))];
        let parts = make_parts_for_lower(nodes, vec![], 1);

        let result = vb_validate::shared::validate(&parts);

        assert!(
            result.is_ok(),
            "plain validate should NOT check gate 12 for Do with action 7, got: {:?}",
            result
        );
    }

    /// RED-PHASE: validate_with_contracts catches missing contracts.
    #[test]
    fn validate_with_contracts_catches_missing_contracts() {
        let nodes = vec![do_node(0, ActionId::new(7), SlotIdx::new(0))];
        let parts = make_parts_for_lower(nodes, vec![], 1);

        let result = vb_validate::shared::validate_with_contracts(&parts, &[]);

        match result {
            Err(vb_validate::ValidationError::ActionContractMissing {
                action_id,
                node_index,
            }) => {
                assert_eq!(action_id, 7);
                assert_eq!(node_index, 0);
            }
            Ok(_) => panic!("Expected error, got Ok"),
            other => panic!(
                "Expected ValidationError::ActionContractMissing, got: {:?}",
                other
            ),
        }
    }

    /// RED-PHASE: validate_with_contracts catches orphan contracts.
    #[test]
    fn validate_with_contracts_catches_orphan_contracts() {
        let nodes = vec![finish_node(0, 0)];
        let parts = make_parts_for_lower(nodes, vec![], 1);

        let orphan_contract = ActionContract {
            id: ActionId::new(99),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        };

        let result = vb_validate::shared::validate_with_contracts(&parts, &[orphan_contract]);

        match result {
            Err(vb_validate::ValidationError::ActionContractOrphan { action_id }) => {
                assert_eq!(action_id, 99);
            }
            Ok(_) => panic!("Expected error, got Ok"),
            other => panic!(
                "Expected ValidationError::ActionContractOrphan, got: {:?}",
                other
            ),
        }
    }

    /// RED-PHASE: CompileErrors contains exactly one error for isolated failures.
    #[test]
    fn compile_errors_contains_one_error_for_isolated_validation_failure() {
        let nodes = vec![do_node(0, ActionId::new(7), SlotIdx::new(1))];
        let parts = make_parts_for_lower(nodes, vec![], 1);

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
                    "Expected exactly 1 error, got {}",
                    errors.len()
                );
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }
}
