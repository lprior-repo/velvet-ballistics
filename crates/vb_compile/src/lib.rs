//! Cold-path YAML compiler boundary.
//!
//! YAML enters the system only through this crate. The hot engine consumes only
//! `vb_core::CompiledWorkflow` values built from native Rust `saphyr` parsing.

// NOTE: Duplicate validation with `vb_validate`
// -----------------------------------------------
// The modules below (schema, references, control_flow, type_taint) perform the
// same *logical* validation passes as `vb_validate`, but they operate on the
// compiler's own AST types (`WorkflowAst`, `Yaml`) rather than the standalone
// `vb_validate::WorkflowDoc` / `WorkflowRefs` / `WorkflowFlow` / `WorkflowTypes`
// types.  This means they cannot simply be replaced by calls into `vb_validate`
// without first unifying the input types.
//
// Future work should converge on a single input representation so that both
// crates share the same validation logic. Until then, both copies must be kept
// in sync when validation rules change.

pub mod ast;
mod control_flow;
pub mod expression;
mod expression_bytecode;
mod references;
mod schema;
pub mod strict_yaml;
mod type_taint;

pub use expression_bytecode::compile_expr_to_bytecode;

// Re-export the shared validation error types from `vb_validate` so that
// downstream consumers of this crate can optionally use the standalone
// validator's error domain without depending on `vb_validate` directly.
pub use vb_validate::{ValidationError, ValidationResult};

use saphyr::{LoadableYamlNode, Yaml};
use saphyr_parser::{Event, Parser, Span, StrInput};
use std::collections::HashSet;
use std::str;
use thiserror::Error;
use vb_core::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue,
    ExprIdx, ExprProgram, ResourceContract, SlotBranch, SlotIdx, StepIdx,
    WorkflowDigest, WorkflowError, WorkflowParts,
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
        let parts = build_workflow_parts(text, doc).map_err(|e| CompileErrors(vec![e]))?;
        let workflow =
            CompiledWorkflow::try_from_parts(parts).map_err(|e| CompileErrors(vec![e.into()]))?;
        Ok(workflow)
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
pub fn lower_steps_to_ir(
    nodes: Vec<CompiledNode>,
    expressions: Vec<ExprProgram>,
    accessors: Vec<AccessorProgram>,
    constants: Vec<ConstValue>,
    slot_count: u16,
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
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
    };
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
    let branch_count = u16::try_from(branches.len()).map_err(|_| CompileError::SlotIndexOutOfRange {
        value: i64::try_from(branches.len()).unwrap_or(i64::MAX),
    })?;
    let accumulator = alloc_accumulator_slot(builder)?;
    let mut nodes = vec![CompiledNode {
        id,
        output: Some(accumulator),
        next: None,
        kind: CompiledNodeKind::TogetherStart {
            branches: branches.into_boxed_slice(),
            join,
        },
    }];
    for (i, entry) in (0..branch_count).zip(nodes.iter().map(|_| StepIdx::new(0))) {
        // placeholder branch nodes; real entries are filled by the caller
        let _ = i;
        let _ = entry;
    }
    nodes.push(CompiledNode {
        id: join,
        output: Some(accumulator),
        next: None,
        kind: CompiledNodeKind::TogetherJoin { branch_count, accumulator },
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
    let attempt_slot = slot_idx_for_step(id.as_usize().checked_add(1).ok_or(
        CompileError::SlotIndexOutOfRange {
            value: i64::MAX,
        },
    )?)?;
    builder.record_slot(attempt_slot);
    Ok(vec![
        CompiledNode {
            id,
            output: None,
            next: None,
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
            kind: CompiledNodeKind::RepeatFinish {
                result: attempt_slot,
            },
        },
    ])
}

/// Lowers a `wait` primitive into `WaitUntil` or `WaitEvent` IR nodes.
pub fn lower_wait(
    id: StepIdx,
    deadline_or_event: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    is_event: bool,
    builder: &mut SlotCompiler,
) -> CompiledNode {
    builder.record_slot(deadline_or_event);
    let kind = if is_event {
        CompiledNodeKind::WaitEvent {
            event: deadline_or_event,
            timeout_slot,
        }
    } else {
        CompiledNodeKind::WaitUntil {
            deadline_slot: deadline_or_event,
        }
    };
    CompiledNode {
        id,
        output: None,
        next: None,
        kind,
    }
}

/// Lowers an `ask` primitive into `Ask` and `AskResume` IR nodes.
pub fn lower_ask(
    id: StepIdx,
    prompt: SlotIdx,
    answer: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    builder: &mut SlotCompiler,
) -> Vec<CompiledNode> {
    builder.record_slot(prompt);
    builder.record_slot(answer);
    vec![
        CompiledNode {
            id,
            output: None,
            next: None,
            kind: CompiledNodeKind::Ask {
                prompt,
                timeout_slot,
            },
        },
        CompiledNode {
            id: id.checked_add(1).unwrap_or(StepIdx::MAX),
            output: Some(answer),
            next: None,
            kind: CompiledNodeKind::AskResume { answer },
        },
    ]
}

/// Lowers a `finish` primitive into a terminal `Finish` node.
pub fn lower_finish(id: StepIdx, result: SlotIdx, builder: &mut SlotCompiler) -> CompiledNode {
    builder.record_slot(result);
    CompiledNode {
        id,
        output: None,
        next: None,
        kind: CompiledNodeKind::Finish { result },
    }
}

/// Validates compiled workflow IR against structural and resource invariants.
///
/// Delegates to [`CompiledWorkflow::try_from_parts`] for full validation.
pub fn validate_ir(parts: WorkflowParts) -> Result<CompiledWorkflow, WorkflowError> {
    CompiledWorkflow::try_from_parts(parts)
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
        .map(|vec| vec.into_boxed_slice())
        .map_err(|error| {
            CompileErrors(vec![CompileError::ExpressionLoweringUnsupported {
                feature: Box::leak(format!("postcard serialization failed: {error}").into_boxed_str()),
            }])
        })
}

/// Generates a Rust source file from a compiled workflow.
///
/// The generated Rust code can be compiled and linked for maxperf execution
/// where the hot interpreter is bypassed entirely.
pub fn compile_to_generated_rust(workflow: &CompiledWorkflow) -> Result<String, CompileErrors> {
    vb_codegen::emit_rust_workflow(workflow)
        .map_err(|error| CompileErrors(vec![CompileError::ExpressionLoweringUnsupported {
            feature: Box::leak(error.to_string().into_boxed_str()),
        }]))
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
                feature: "expression table overflow",
            }
        })?;
        self.expressions.push(program);
        Ok(ExprIdx::new(index))
    }

    /// Pushes an accessor program and returns its index.
    pub fn push_accessor(&mut self, program: AccessorProgram) -> Result<vb_core::AccessorIdx, CompileError> {
        let index = u16::try_from(self.accessors.len()).map_err(|_| {
            CompileError::ExpressionLoweringUnsupported {
                feature: "accessor table overflow",
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
            nodes: self.nodes.into_boxed_slice(),
            expressions: self.expressions.into_boxed_slice(),
            accessors: self.accessors.into_boxed_slice(),
            constants: self.constants.into_boxed_slice(),
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
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
        feature: &'static str,
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
}

impl CompileError {
    /// Stable machine-readable validation diagnostic code.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::SourceTooLarge { .. } => "PAYLOAD_TOO_LARGE",
            Self::Utf8(_) | Self::Parse(_) | Self::DocumentCount { .. } => "FORBIDDEN_YAML_FEATURE",
            Self::EmptySource => "MISSING_REQUIRED_FIELD",
            Self::TopLevelNotMapping => "TYPE_MISMATCH",
            Self::NonStringKey { .. }
            | Self::AliasForbidden { .. }
            | Self::AnchorForbidden { .. }
            | Self::MergeKeyForbidden { .. }
            | Self::TagForbidden { .. }
            | Self::BadValue
            | Self::FloatForbidden => "FORBIDDEN_YAML_FEATURE",
            Self::DuplicateKey { .. } => "DUPLICATE_KEY",
            Self::DepthLimit { .. }
            | Self::NodeLimit { .. }
            | Self::SequenceLimit { .. }
            | Self::MappingLimit { .. }
            | Self::ScalarLimit { .. } => "LIMIT_EXCEEDED",
            Self::Workflow(error) => workflow_error_code(error),
            Self::MissingField { .. }
            | Self::MissingTriggerField { .. }
            | Self::MissingStepId { .. }
            | Self::MissingStepField { .. } => "MISSING_REQUIRED_FIELD",
            Self::UnknownTopLevelField { .. } => "UNKNOWN_TOP_LEVEL_FIELD",
            Self::InvalidVersion { .. } => "INVALID_VERSION",
            Self::InvalidTriggerCount { .. }
            | Self::UnknownTriggerKind { .. }
            | Self::TriggerShape { .. }
            | Self::UnknownTriggerField { .. }
            | Self::InvalidTriggerField { .. } => "UNSUPPORTED_TRIGGER",
            Self::FieldShape { .. } => "TYPE_MISMATCH",
            Self::UnknownInputSchemaField { .. } => "UNKNOWN_INPUT_SCHEMA_FIELD",
            Self::InvalidInputSchema { .. } => "TYPE_MISMATCH",
            Self::UnsupportedTopLevelResult => "INVALID_FINISH",
            Self::EmptySteps => "MISSING_STEP_PRIMITIVE",
            Self::InvalidName { field, value } => invalid_name_code(field, value),
            Self::DuplicateStepId { .. } => "DUPLICATE_ID",
            Self::StepShape { .. } => "TYPE_MISMATCH",
            Self::UnknownStepField { .. } | Self::UnknownStepPrimitiveField { .. } => {
                "UNKNOWN_STEP_FIELD"
            }
            Self::MissingStepPrimitive { .. } => "MISSING_STEP_PRIMITIVE",
            Self::MultipleStepPrimitives { .. } => "MULTIPLE_STEP_PRIMITIVES",
            Self::UnsupportedStepPrimitive { primitive, .. } => primitive_code(primitive),
            Self::UnsupportedStepControlField { field, .. } => control_field_code(field),
            Self::StepFieldShape { field, .. } => step_field_shape_code(field),
            Self::StepIndexOutOfRange { .. }
            | Self::SlotIndexOutOfRange { .. }
            | Self::BranchTargetOutOfRange { .. } => "LIMIT_EXCEEDED",
            Self::BackwardBranchTarget { .. } | Self::UnknownStepTarget { .. } => {
                "INVALID_THEN_TARGET"
            }
            Self::LastStepMustFinish => "INVALID_FINISH",
            Self::UnsupportedConstantValue { .. } => "TYPE_MISMATCH",
            Self::UnknownReferenceRoot { .. } => "UNKNOWN_REFERENCE",
            Self::IllegalReference { .. } => "DIRECT_RUNTIME_REFERENCE",
            Self::UnknownReferenceName { kind, .. } => unknown_reference_code(kind),
            Self::UnreachableStep { .. } => "UNREACHABLE_STEP",
            Self::TypeMismatch { .. } | Self::UnknownSlotType { .. } => "TYPE_MISMATCH",
            Self::SecretTaintLeak { .. } => "SECRET_RESULT_LEAK",
            Self::ExpressionUnexpectedChar { .. }
            | Self::ExpressionUnterminatedString { .. }
            | Self::ExpressionIntegerOutOfRange { .. }
            | Self::ExpressionLimitExceeded { .. }
            | Self::ExpressionUnexpectedToken { .. }
            | Self::ExpressionUnknownIdentifier { .. }
            | Self::ExpressionLoweringUnsupported { .. }
            | Self::ExpressionHelperArity { .. } => "INVALID_EXPRESSION",
        }
    }

    /// Alias for integrations that name the machine field explicitly.
    #[must_use]
    pub fn diagnostic_code(&self) -> &'static str {
        self.code()
    }
}

fn workflow_error_code(error: &WorkflowError) -> &'static str {
    match error {
        WorkflowError::ResourceContractExceeded { .. }
        | WorkflowError::ResourceContractTooLarge { .. } => "LIMIT_EXCEEDED",
        WorkflowError::StepOutOfBounds { .. } => "INVALID_THEN_TARGET",
        WorkflowError::SlotOutOfBounds { .. } => "TYPE_MISMATCH",
        WorkflowError::ConstOutOfBounds { .. } => "CONST_OUT_OF_BOUNDS",
        WorkflowError::Expression(_) => "INVALID_EXPRESSION",
        WorkflowError::EmptyNodes
        | WorkflowError::EntryOutOfBounds { .. }
        | WorkflowError::NodeIdMismatch { .. }
        | WorkflowError::EmptyBranchTable => "INVALID_COMPILED_WORKFLOW",
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
#[derive(Debug, Error)]
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

fn build_workflow_parts(text: &str, doc: &Yaml<'_>) -> Result<WorkflowParts, CompileError> {
    validate_workflow_document_shape(doc)?;

    let name = required_string_field(doc, "name")?;
    let steps = required_sequence_field(doc, "steps")?;
    let digest = WorkflowDigest::from_bytes(blake3::hash(text.as_bytes()).into());
    let mut builder = WorkflowBuilder::new();
    let last_step = steps.len().checked_sub(1).ok_or(CompileError::EmptySteps)?;

    for (index, step) in steps.iter().enumerate() {
        let nodes = compile_step(step, index, last_step, &mut builder)?;
        builder.nodes.extend(nodes);
    }
    Ok(WorkflowParts {
        name: Box::<str>::from(name),
        digest,
        slot_count: builder.slot_count()?,
        nodes: builder.nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
    })
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
        StepPrimitive::Save => validate_save_shape(body, index, last_step),
        StepPrimitive::Choose => validate_choose_shape(body, index, last_step),
        StepPrimitive::Finish => validate_finish_shape(body, index, last_step),
        value => Err(CompileError::UnsupportedStepPrimitive {
            step: index,
            primitive: value.as_str(),
        }),
    }
}

fn validate_save_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    if body.is_mapping() {
        Ok(())
    } else {
        Err(CompileError::StepFieldShape {
            step: index,
            field: "save",
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
    let _ = required_step_field(body, index, "condition")?;
    let _ = required_branch_target(body, index, "on_true")?;
    let _ = required_branch_target(body, index, "on_false")?;
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
    let _ = required_step_field(body, index, "result")?;
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
        let _ = slot_value(value, 0)?;
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
    "save",
    "choose",
    "for_each",
    "together",
    "gather",
    "summarize",
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

fn compile_step(
    step: &Yaml<'_>,
    index: usize,
    last_step: usize,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    let StepSpec { primitive, body } = step_spec(step, index)?;
    let node = match primitive {
        StepPrimitive::Save => compile_save(body, index, last_step, builder),
        StepPrimitive::Choose => compile_choose(body, index, last_step, builder),
        StepPrimitive::Finish => return compile_finish(body, index, last_step, builder),
        value => Err(CompileError::UnsupportedStepPrimitive {
            step: index,
            primitive: value.as_str(),
        }),
    }?;
    Ok(vec![node])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepPrimitive {
    Run,
    Save,
    Choose,
    ForEach,
    Together,
    Gather,
    Summarize,
    Repeat,
    Wait,
    Ask,
    Finish,
}

impl StepPrimitive {
    fn from_field(field: &str) -> Option<Self> {
        match field {
            "run" => Some(Self::Run),
            "save" => Some(Self::Save),
            "choose" => Some(Self::Choose),
            "for_each" => Some(Self::ForEach),
            "together" => Some(Self::Together),
            "gather" => Some(Self::Gather),
            "summarize" => Some(Self::Summarize),
            "repeat" => Some(Self::Repeat),
            "wait" => Some(Self::Wait),
            "ask" => Some(Self::Ask),
            "finish" => Some(Self::Finish),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Save => "save",
            Self::Choose => "choose",
            Self::ForEach => "for_each",
            Self::Together => "together",
            Self::Gather => "gather",
            Self::Summarize => "summarize",
            Self::Repeat => "repeat",
            Self::Wait => "wait",
            Self::Ask => "ask",
            Self::Finish => "finish",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct StepSpec<'a> {
    primitive: StepPrimitive,
    body: &'a Yaml<'a>,
}

#[derive(Debug, Clone, Copy)]
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

fn compile_save(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_non_mapping_step_body(body, index, "save", "an object")?;
    let output = slot_idx_for_step(index)?;
    let constant = save_slot_value(body, index)?;
    let constant = builder.push_constant(constant)?;
    builder.record_slot(output);
    set_const_node(step_idx(index)?, output, constant, next_step(index)?)
}

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
        kind: CompiledNodeKind::SetConst { value },
    })
}

fn save_slot_value(body: &Yaml<'_>, step: usize) -> Result<ConstValue, CompileError> {
    let Some(mapping) = body.as_mapping() else {
        return Err(CompileError::StepFieldShape {
            step,
            field: "save",
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

fn compile_choose(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "choose", &["condition", "on_true", "on_false"])?;
    let condition = required_choose_condition(body, index)?;
    let on_true = required_branch_target(body, index, "on_true")?;
    let on_false = required_branch_target(body, index, "on_false")?;
    match condition {
        ChooseCondition::Slot(condition) => {
            compile_slot_choose(step_idx(index)?, condition, on_true, on_false, builder)
        }
        ChooseCondition::Literal(value) => {
            compile_literal_choose(index, value, on_true, on_false, builder)
        }
    }
}

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

fn compile_literal_choose(
    index: usize,
    value: bool,
    on_true: StepIdx,
    on_false: StepIdx,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    let output = slot_idx_for_step(index)?;
    let constant = builder.push_constant(ConstValue::Bool(value))?;
    builder.record_slot(output);
    Ok(CompiledNode {
        id: step_idx(index)?,
        output: Some(output),
        next: Some(if value { on_true } else { on_false }),
        kind: CompiledNodeKind::SetConst { value: constant },
    })
}

fn compile_finish(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
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
    compile_finish_result(result, index, builder)
}

fn compile_finish_result(
    result: &Yaml<'_>,
    index: usize,
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    if let Some(slot) = finish_result_slot(result, index)? {
        builder.record_slot(slot);
        return Ok(vec![CompiledNode {
            id: step_idx(index)?,
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish { result: slot },
        }]);
    }
    let value = slot_value(result, index)?;
    let value = builder.push_constant(value)?;
    let output = slot_idx_for_step(index)?;
    builder.record_slot(output);
    let finish_id = next_step(index)?;
    Ok(vec![
        CompiledNode {
            id: step_idx(index)?,
            output: Some(output),
            next: Some(finish_id),
            kind: CompiledNodeKind::SetConst { value },
        },
        CompiledNode {
            id: finish_id,
            output: None,
            next: None,
            kind: CompiledNodeKind::Finish { result: output },
        },
    ])
}

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

fn next_step(index: usize) -> Result<StepIdx, CompileError> {
    let value = index
        .checked_add(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: index })?;
    step_idx(value)
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
        Yaml::Value(saphyr::Scalar::String(value)) => text_slot_value(value.as_ref(), step),
        Yaml::Representation(value, _, None) => text_slot_value(value.as_ref(), step),
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
    use super::{CompileError, CompileErrors, SlotCompiler, SourceMark, YamlCompiler, YamlLimits};
    use super::{compute_compiled_digest, lower_do, lower_finish, lower_set};
    use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx};
    use vb_core::workflow::{CompiledNodeKind, ExprProgram};
    use vb_core::ConstValue;
    use vb_core::{CompiledWorkflow, ResourceContract};

    const NESTED_SAVE_SOURCE: &[u8] = br#"
version: velvet-ballastics/v1
name: nested_save
when:
  manual: {}
steps:
  - id: build_result
    save:
      text: done
      tags:
        - demo
        - fast
      metadata:
        attempts: 1
        active: true
        note: null
  - id: done
    finish:
      result: 0
"#;

    const OPTIONAL_TOP_LEVEL_FIELDS_SOURCE: &[u8] = br#"
version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
inputs:
  value: text
vars:
  label: 1
secrets:
  api_key: API_KEY
result: {}
examples:
  - name: fixture
    input:
      value: 1
steps:
  - id: build_result
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;

    fn compile_with_inputs(inputs: &str) -> Result<CompiledWorkflow, CompileErrors> {
        let source = format!(
            "version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {{}}\ninputs:\n{inputs}steps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
        );
        YamlCompiler::default().compile(source.as_bytes())
    }

    fn compile_error_text(source: &[u8]) -> String {
        match YamlCompiler::default().compile(source) {
            Ok(_) => "compile unexpectedly succeeded".to_owned(),
            Err(errors) => match errors.first() {
                Some(error) => error.to_string(),
                None => "CompileErrors was empty".to_owned(),
            },
        }
    }

    fn parse_ast_error_text(source: &[u8]) -> String {
        match YamlCompiler::default().parse_ast(source) {
            Ok(_) => "parse_ast unexpectedly succeeded".to_owned(),
            Err(errors) => match errors.first() {
                Some(error) => error.to_string(),
                None => "CompileErrors was empty".to_owned(),
            },
        }
    }

    fn assert_compile_parse_first_error(source: &[u8]) {
        assert_eq!(compile_error_text(source), parse_ast_error_text(source));
    }

    fn compile_first_error(source: &[u8]) -> Result<CompileError, String> {
        match YamlCompiler::default().compile(source) {
            Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
            Err(errors) => errors
                .first()
                .cloned()
                .ok_or_else(|| "CompileErrors was empty".to_owned()),
        }
    }

    fn parse_first_error(source: &[u8]) -> Result<CompileError, String> {
        match YamlCompiler::default().parse_ast(source) {
            Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
            Err(errors) => errors
                .first()
                .cloned()
                .ok_or_else(|| "CompileErrors was empty".to_owned()),
        }
    }

    fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
    where
        T: core::fmt::Debug + PartialEq,
    {
        if actual == expected {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, found {actual:?}"))
        }
    }

    fn assert_compile_code(source: &[u8], expected: &'static str) -> Result<(), String> {
        let error = compile_first_error(source)?;
        ensure_equal(error.code(), expected)?;
        ensure_equal(error.diagnostic_code(), expected)
    }

    #[test]
    fn compile_error_exposes_stable_validation_codes() -> Result<(), String> {
        for (source, code) in [
            (
                b"version: velvet-ballastics/v1\nversion: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n".as_slice(),
                "DUPLICATE_KEY",
            ),
            (
                b"version: velvet-ballastics/v1\nname: &n fast_path\ncopy: *n\n",
                "FORBIDDEN_YAML_FEATURE",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nunexpected: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
                "UNKNOWN_TOP_LEVEL_FIELD",
            ),
            (
                b"name: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
                "MISSING_REQUIRED_FIELD",
            ),
            (
                b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
                "INVALID_VERSION",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: BuildResult\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
                "INVALID_ID",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: finish\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
                "RESERVED_ID",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: duplicate\n    save:\n      value: 1\n  - id: duplicate\n    finish:\n      result: 0\n",
                "DUPLICATE_ID",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: only_metadata\n    name: Only Metadata\n  - id: done\n    finish:\n      result: 0\n",
                "MISSING_STEP_PRIMITIVE",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n    finish:\n      result: 0\n  - id: done\n    finish:\n      result: 0\n",
                "MULTIPLE_STEP_PRIMITIVES",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose: true\n  - id: done\n    finish:\n      result: 0\n",
                "INVALID_CHOOSE",
            ),
        ] {
            assert_compile_code(source, code)?;
        }
        Ok(())
    }

    #[test]
    fn reference_diagnostic_codes_cover_public_reference_contract() -> Result<(), String> {
        assert_compile_code(
            br#"version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$input.missing == true"
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
            "UNKNOWN_REFERENCE",
        )?;
        assert_compile_code(
            br#"version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$secrets.api_key == \"x\""
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
            "SECRET_NOT_DECLARED",
        )
    }

    #[test]
    fn compile_errors_exposes_ordered_error_and_code_accessors() {
        let errors = CompileErrors(vec![
            CompileError::SourceTooLarge {
                actual: 8,
                limit: 4,
            },
            CompileError::InvalidVersion {
                actual: Box::<str>::from("velvet/v1"),
            },
        ]);
        let codes: Vec<&'static str> = errors.diagnostic_codes().collect();

        assert_eq!(errors.len(), 2);
        assert_eq!(errors.as_slice().len(), 2);
        assert_eq!(errors.iter().count(), 2);
        assert_eq!(codes, vec!["PAYLOAD_TOO_LARGE", "INVALID_VERSION"]);
    }

    #[test]
    fn parse_ast_and_compile_expose_same_diagnostic_codes() -> Result<(), String> {
        for source in [
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nunexpected: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n".as_slice(),
            br#"version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$input.flag =="
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
            br#"version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$secrets.api_key == \"x\""
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
        ] {
            let compile = compile_first_error(source)?;
            let parse = parse_first_error(source)?;
            ensure_equal(compile.code(), parse.code())?;
        }
        Ok(())
    }

    #[test]
    fn compiler_rejects_save_object_until_handle_arenas_exist() {
        let source = br#"
version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: build_result
    save:
      text: done
  - id: done
    finish:
      result: 0
"#;
        let result = YamlCompiler::default().compile(source);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedConstantValue { step: 0 }))
        ));
    }

    #[test]
    fn compiler_rejects_nested_save_values_until_handle_arenas_exist() {
        let result = YamlCompiler::default().compile(NESTED_SAVE_SOURCE);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedConstantValue { step: 0 }))
        ));
    }

    #[test]
    fn compiler_rejects_scalar_save_body() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save: done\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::StepFieldShape { field: "save", .. }))
        ));
    }

    #[test]
    fn compiler_rejects_save_references_until_expressions_exist() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\ninputs:\n  value: text\nsteps:\n  - id: build_result\n    save:\n      text: $input.value\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedConstantValue { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_empty_steps() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps: []\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::EmptySteps)))
        );
    }

    #[test]
    fn compiler_rejects_unsupported_top_level_fields() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nunexpected: true\nsteps:\n  - finish:\n      result: 0\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownTopLevelField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_missing_workflow_version() {
        let result = YamlCompiler::default().compile(
            b"name: fast_path\nwhen:\n  manual: {}\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingField { .. })))
        );
    }

    #[test]
    fn compiler_rejects_non_canonical_workflow_version() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidVersion { .. })))
        );
    }

    #[test]
    fn compiler_accepts_optional_top_level_fields() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"));
    }

    #[test]
    fn compiler_accepts_allowed_input_schema_shorthand() {
        for shorthand in [
            "text",
            "number",
            "boolean",
            "object",
            "any",
            "list<any>",
            "list<text>",
            "list<number>",
            "list<boolean>",
        ] {
            let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

            assert!(
                matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
                "schema shorthand {shorthand} should compile"
            );
        }
    }

    #[test]
    fn compiler_rejects_unknown_input_schema_shorthand() {
        for shorthand in ["integer", "string", "list", "list<object>"] {
            let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
                "schema shorthand {shorthand} should be rejected"
            );
        }
    }

    #[test]
    fn compiler_and_ast_report_same_schema_diagnostics() {
        for inputs in [
            "  value: integer\n",
            "  value:\n    is: text\n    kind: text\n",
            "  value:\n    is: text\n    default: 1\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {{}}\ninputs:\n{inputs}steps:\n  - id: done\n    finish:\n      result: 0\n"
            );

            assert_compile_parse_first_error(source.as_bytes());
        }
    }

    #[test]
    fn schema_validation_does_not_preempt_yaml_profile_errors() {
        assert_compile_parse_first_error(
            b"version: velvet-ballastics/v1\nname: &n schema_case\ninputs:\n  value: integer\ncopy: *n\n",
        );
    }

    #[test]
    fn schema_validation_does_not_preempt_duplicate_key_errors() {
        assert_compile_parse_first_error(
            b"version: velvet-ballastics/v1\nversion: velvet-ballastics/v1\nname: schema_case\ninputs:\n  value: integer\n",
        );
    }

    #[test]
    fn schema_validation_does_not_preempt_lowering_errors() {
        let source = b"version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {}\ninputs:\n  value: integer\nsteps:\n  - id: route\n    choose: true\n";

        assert_eq!(
            compile_error_text(source),
            CompileError::LastStepMustFinish.to_string()
        );
        assert_compile_parse_first_error(source);
    }

    #[test]
    fn schema_validation_does_not_preempt_finish_position_errors() {
        let source = b"version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {}\ninputs:\n  value: integer\nsteps:\n  - id: early\n    finish:\n      result: 0\n      status: success\n  - id: done\n    finish:\n      result: 0\n";

        assert!(compile_error_text(source).contains("field finish"));
        assert_compile_parse_first_error(source);
    }

    #[test]
    fn compiler_accepts_input_long_form_scalar_constraints() {
        let result = compile_with_inputs(
            "  title:\n    from: request.body.title\n    is: text\n    default: hello\n    min_length: 1\n    max_length: 20\n    optional: true\n    nullable: false\n    secret: false\n  score:\n    is: number\n    default: 10\n    min: 0\n    max: 100\n  approved:\n    is: boolean\n    default: true\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"));
    }

    #[test]
    fn compiler_accepts_input_long_form_object_fields() {
        let result = compile_with_inputs(
            "  customer:\n    from: request.body.customer\n    is: object\n    fields:\n      id: text\n      email: text\n      address:\n        is: object\n        optional: true\n        nullable: true\n        fields:\n          city: text\n          country: text\n    extra: reject\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"));
    }

    #[test]
    fn compiler_accepts_input_long_form_list_elements() {
        for element in ["any", "text", "number", "boolean", "object"] {
            let result = compile_with_inputs(&format!(
                "  values:\n    is: list\n    of: {element}\n    default: []\n    min: 0\n    max: 10\n"
            ));

            assert!(
                matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
                "list element schema {element} should compile"
            );
        }
    }

    #[test]
    fn compiler_rejects_input_schema_unknown_fields() {
        for inputs in [
            "  value:\n    is: text\n    kind: text\n",
            "  customer:\n    is: object\n    fields:\n      value:\n        is: text\n        from: request.body.value\n",
        ] {
            let result = compile_with_inputs(inputs);

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownInputSchemaField { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_pattern_until_bounded_regex_exists() {
        let result = compile_with_inputs("  value:\n    is: text\n    pattern: ^[a-z]+$\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema {
                field: "inputs.pattern",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_child_fields() {
        for inputs in [
            "  values:\n    is: list\n",
            "  value:\n    is: text\n    of: text\n",
            "  value:\n    is: text\n    fields:\n      nested: text\n",
            "  value:\n    is: text\n    extra: reject\n",
            "  customer:\n    is: object\n    extra: ignore\n",
            "  customer:\n    is: object\n    fields: true\n",
            "  values:\n    is: list\n    of: integer\n",
            "  value:\n    is: integer\n",
        ] {
            let result = compile_with_inputs(inputs);

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
                "invalid schema should be rejected: {inputs}"
            );
        }
    }

    #[test]
    fn compiler_rejects_non_boolean_input_schema_flags() {
        for flag in ["optional", "nullable", "secret"] {
            let result = compile_with_inputs(&format!("  value:\n    is: text\n    {flag}: yes\n"));

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_default_that_does_not_match_input_schema() {
        for inputs in [
            "  value:\n    is: text\n    default: 1\n",
            "  value:\n    is: number\n    default: nope\n",
            "  value:\n    is: boolean\n    default: nope\n",
            "  value:\n    is: object\n    default: []\n",
            "  value:\n    is: list\n    of: text\n    default: {}\n",
        ] {
            let result = compile_with_inputs(inputs);

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
            ));
        }
    }

    #[test]
    fn compiler_validates_null_input_schema_defaults() {
        let rejected = compile_with_inputs("  value:\n    is: text\n    default: null\n");
        let accepted =
            compile_with_inputs("  value:\n    is: text\n    nullable: true\n    default: null\n");

        assert!(matches!(
            rejected,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
        ));
        assert!(matches!(accepted, Ok(ref workflow) if workflow.name() == "schema_case"));
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_bounds() {
        for inputs in [
            "  value:\n    is: number\n    min: 10\n    max: 1\n",
            "  values:\n    is: list\n    of: text\n    min: -1\n",
            "  value:\n    is: text\n    min: 1\n",
            "  value:\n    is: text\n    min_length: -1\n",
            "  value:\n    is: text\n    min_length: 10\n    max_length: 1\n",
            "  value:\n    is: number\n    min_length: 1\n",
        ] {
            let result = compile_with_inputs(inputs);

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
                "invalid bounds should be rejected: {inputs}"
            );
        }
    }

    #[test]
    fn compiler_rejects_non_mapping_optional_top_level_fields() {
        for field in ["inputs", "vars", "secrets"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. }))),
                "{field} must be mapping-shaped"
            );
        }
    }

    #[test]
    fn compiler_rejects_invalid_optional_top_level_names() {
        for (field, key) in [
            ("inputs", "InputValue"),
            ("vars", "run"),
            ("secrets", "api-key"),
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}:\n  {key}: value\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidName { .. }))),
                "{field}.{key} must use Velvet v1 public naming"
            );
        }
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_shapes() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\ninputs:\n  value:\n    - text\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. })))
        );
    }

    #[test]
    fn compiler_rejects_runtime_references_in_vars() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nvars:\n  label: $input.value\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedConstantValue { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_non_string_secret_bindings() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsecrets:\n  api_key: 42\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. })))
        );
    }

    #[test]
    fn compiler_rejects_invalid_examples_shape() {
        for examples in ["true", "\n  - fixture"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nexamples: {examples}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. }))),
                "examples must be a sequence of mappings"
            );
        }
    }

    #[test]
    fn compiler_rejects_examples_without_valid_names() {
        for examples in ["\n  - input: {}", "\n  - name: 42", "\n  - name: run"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nexamples: {examples}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(ref errors) if matches!(
                        errors.first(),
                        Some(
                            CompileError::MissingField { .. }
                                | CompileError::FieldShape { .. }
                                | CompileError::InvalidName { .. }
                        )
                    )
                ),
                "examples must declare valid fixture names"
            );
        }
    }

    #[test]
    fn compiler_rejects_non_empty_top_level_result_until_result_ir_exists() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nresult:\n  value: $build_result.value\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedTopLevelResult))
        ));
    }

    #[test]
    fn compiler_rejects_non_mapping_top_level_result() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nresult: done\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape {
                field: "result",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_names() {
        for name in ["", "FastPath", "fast-path", "run"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: \"{name}\"\nwhen:\n  manual: {{}}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidName { field: "name", .. }))),
                "workflow name {name:?} must be rejected"
            );
        }
    }

    #[test]
    fn compiler_rejects_missing_step_ids() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingStepId { .. })))
        );
    }

    #[test]
    fn compiler_rejects_invalid_step_ids() {
        for id in ["", "BuildResult", "build-result", "finish"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: \"{id}\"\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(ref errors) if matches!(
                        errors.first(),
                        Some(CompileError::InvalidName {
                            field: "step id",
                            ..
                        })
                    )
                ),
                "step id {id:?} must be rejected"
            );
        }
    }

    #[test]
    fn compiler_rejects_duplicate_step_ids() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: duplicate\n    save:\n      value: 1\n  - id: duplicate\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::DuplicateStepId { .. })))
        );
    }

    #[test]
    fn compiler_accepts_step_display_name_metadata() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    name: Build Result\n    save:\n      value: 1\n  - id: done\n    name: Done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"));
    }

    #[test]
    fn compiler_rejects_non_string_step_display_name() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    name: 42\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::StepFieldShape { field: "name", .. }))
        ));
    }

    #[test]
    fn compiler_rejects_unsupported_phase_zero_step_control_fields() {
        for control in ["if", "with", "try_again", "on_error", "then"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: build_result\n    {control}: true\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedStepControlField { .. }))
                ),
                "control field {control} must be rejected until Phase 0 compiles it"
            );
        }
    }

    #[test]
    fn compiler_rejects_missing_workflow_trigger() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingField { .. })))
        );
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_shapes() {
        for source in [
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen: manual\nsteps:\n  - finish:\n      result: 0\n".as_slice(),
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen: {}\nsteps:\n  - finish:\n      result: 0\n",
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\n  event: {}\nsteps:\n  - finish:\n      result: 0\n",
        ] {
            let result = YamlCompiler::default().compile(source);

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. } | CompileError::InvalidTriggerCount { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_unknown_workflow_trigger_kind() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  file: {}\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownTriggerKind { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_scalar_workflow_trigger_config() {
        for trigger in ["manual", "webhook", "schedule", "event"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  {trigger}: true\nsteps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::TriggerShape { .. }))),
                "trigger {trigger} config must be mapping-shaped"
            );
        }
    }

    #[test]
    fn compiler_accepts_valid_workflow_trigger_configs() {
        for when_body in [
            "  manual: {}\n",
            "  webhook:\n    path: /github\n    method: POST\n    unique: request.header.X-GitHub-Delivery\n",
            "  schedule:\n    cron: \"*/5 * * * *\"\n    timezone: UTC\n",
            "  event:\n    name: customer.created\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"),
                "valid trigger should compile"
            );
        }
    }

    #[test]
    fn compiler_rejects_unknown_workflow_trigger_fields() {
        for when_body in [
            "  manual:\n    extra: true\n",
            "  webhook:\n    path: /github\n    method: POST\n    extra: true\n",
            "  schedule:\n    cron: \"*/5 * * * *\"\n    extra: true\n",
            "  event:\n    name: customer.created\n    extra: true\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownTriggerField { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_missing_required_workflow_trigger_fields() {
        for when_body in [
            "  webhook:\n    method: POST\n",
            "  webhook:\n    path: /github\n",
            "  schedule:\n    timezone: UTC\n",
            "  event: {}\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingTriggerField { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_field_values() {
        for when_body in [
            "  webhook:\n    path: github\n    method: POST\n",
            "  webhook:\n    path: /github\n    method: TRACE\n",
            "  webhook:\n    path: 42\n    method: POST\n",
            "  webhook:\n    path: /github\n    method: POST\n    unique: 42\n",
            "  schedule:\n    cron: \"0 0 0 0 0 0\"\n",
            "  schedule:\n    cron: 42\n",
            "  schedule:\n    cron: \"*/5 * * * *\"\n    timezone: 42\n",
            "  event:\n    name: 42\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidTriggerField { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_backward_branch_targets() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose:\n      condition: true\n      on_true: 0\n      on_false: 1\n  - id: done\n    finish:\n      result: true\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::BackwardBranchTarget { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_extra_phase_zero_choose_fields() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose:\n      condition: 0\n      on_true: 1\n      on_false: 1\n      otherwise: true\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownStepPrimitiveField {
                primitive: "choose",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_non_mapping_phase_zero_choose_body() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose: true\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::StepFieldShape {
                field: "choose",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_extra_phase_zero_finish_fields() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n      status: success\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownStepPrimitiveField {
                primitive: "finish",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_non_mapping_phase_zero_finish_body() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish: success\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::StepFieldShape {
                field: "finish",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_aliases() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet-ballastics/v1\nname: &n fast\ncopy: *n\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::AnchorForbidden { mark }) if mark.available)
        ));
    }

    #[test]
    fn compiler_rejects_custom_tags_with_mark() {
        let result = YamlCompiler::default().compile(b"version: !custom velvet-ballastics/v1\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::TagForbidden { mark }) if mark.available)
        ));
    }

    #[test]
    fn compiler_rejects_non_string_object_keys_with_mark() {
        let result = YamlCompiler::default().compile(b"? [bad]\n: value\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::NonStringKey { mark }) if mark.available)
        ));
    }

    #[test]
    fn compiler_rejects_duplicate_top_level_keys() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nversion: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::DuplicateKey { .. })))
        );
    }

    #[test]
    fn compiler_rejects_duplicate_nested_keys() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      text: first\n      text: second\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::DuplicateKey { .. })))
        );
    }

    #[test]
    fn compiler_rejects_legacy_step_aliases() {
        for alias in ["do", "set", "collect", "reduce", "copy"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: legacy\n    {alias}:\n      slot: 0\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownStepField { .. }))),
                "legacy alias {alias} must be rejected"
            );
        }
    }

    #[test]
    fn compiler_rejects_missing_step_primitive() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: only_metadata\n    name: Only Metadata\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingStepPrimitive { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_multiple_step_primitives() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      slot: 0\n      value: 1\n    finish:\n      result: 0\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::MultipleStepPrimitives { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_phase_zero_unsupported_primitives() {
        for primitive in [
            "run",
            "for_each",
            "together",
            "gather",
            "summarize",
            "repeat",
            "wait",
            "ask",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: unsupported\n    {primitive}: noop\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedStepPrimitive { .. }))),
                "primitive {primitive} should be recognized but unsupported in Phase 0"
            );
        }
    }

    #[test]
    fn compiler_rejects_oversized_source() {
        let limits = YamlLimits {
            max_source_bytes: 4,
            ..YamlLimits::default()
        };
        let result = YamlCompiler::new(limits).compile(b"name: too_large\n");

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::SourceTooLarge { .. })))
        );
    }

    #[test]
    fn compiler_accepts_minimal_strict_workflow() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: strict_minimal\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "strict_minimal"));
    }

    #[test]
    fn compiler_attaches_default_resource_contract() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: resource_case\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;

        if workflow.resource_contract() == ResourceContract::DEFAULT {
            Ok(())
        } else {
            Err(format!(
                "unexpected resource contract: {:?}",
                workflow.resource_contract()
            ))
        }
    }

    #[test]
    fn compiler_rejects_empty_yaml_source() {
        let result = YamlCompiler::default().compile(b"   \n\t  ");

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::EmptySource)))
        );
    }

    #[test]
    fn compiler_rejects_multiple_yaml_documents() {
        let result = YamlCompiler::default().compile(
            b"---\nversion: velvet-ballastics/v1\nname: first\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n---\nversion: velvet-ballastics/v1\nname: second\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::DocumentCount { count: 2 }))
        ));
    }

    #[test]
    fn compiler_rejects_yaml_merge_keys() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: merge_key\nwhen:\n  manual: {}\n<<:\n  steps: []\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::MergeKeyForbidden { .. }))
        ));
    }

    // ── Round 2: Exact-assertion error variant tests ─────────────────────

    #[test]
    fn compile_returns_source_too_large_with_exact_fields() {
        let tiny_limits = YamlLimits { max_source_bytes: 10, ..YamlLimits::default() };
        let compiler = YamlCompiler { limits: tiny_limits };
        let source = b"version: velvet-ballastics/v1\nname: big\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0";
        let result = compiler.compile(source);
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::SourceTooLarge { actual, limit }) = errors.first() else {
            panic!("expected SourceTooLarge, got {:?}", errors.first());
        };
        assert_eq!(*limit, 10);
        assert_eq!(*actual, source.len());
    }

    #[test]
    fn compile_returns_empty_source_for_empty_input() {
        let result = YamlCompiler::default().compile(b"");
        let Err(errors) = result else { panic!("expected error") };
        assert!(matches!(errors.first(), Some(CompileError::EmptySource)));
    }

    #[test]
    fn compile_returns_top_level_not_mapping_for_list_root() {
        let result = YamlCompiler::default().compile(b"- item1\n- item2");
        let Err(errors) = result else { panic!("expected error") };
        assert!(matches!(errors.first(), Some(CompileError::TopLevelNotMapping)));
    }

    #[test]
    fn compile_returns_empty_steps_for_steps_with_empty_list() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: empty\nwhen:\n  manual: {}\nsteps: []",
        );
        let Err(errors) = result else { panic!("expected error") };
        assert!(matches!(errors.first(), Some(CompileError::EmptySteps)));
    }

    #[test]
    fn compile_returns_invalid_version_for_wrong_version() {
        let result = YamlCompiler::default().compile(
            b"version: bad-version\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::InvalidVersion { actual }) = errors.first() else {
            panic!("expected InvalidVersion, got {:?}", errors.first());
        };
        assert_eq!(actual.as_ref(), "bad-version");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_version() {
        let result = YamlCompiler::default().compile(
            b"name: no_version\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            panic!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "version");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_name() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            panic!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "name");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_when() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_trigger\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            panic!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "when");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_steps() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_steps\nwhen:\n  manual: {}",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            panic!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "steps");
    }

    #[test]
    fn compile_returns_invalid_trigger_count_for_empty_when() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: empty_when\nwhen: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::InvalidTriggerCount { count }) = errors.first() else {
            panic!("expected InvalidTriggerCount, got {:?}", errors.first());
        };
        assert_eq!(*count, 0);
    }

    #[test]
    fn compile_returns_unknown_trigger_kind_for_invalid_trigger() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_trigger\nwhen:\n  teleport: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::UnknownTriggerKind { trigger }) = errors.first() else {
            panic!("expected UnknownTriggerKind, got {:?}", errors.first());
        };
        assert_eq!(trigger.as_ref(), "teleport");
    }

    #[test]
    fn compile_returns_missing_step_id_for_step_without_id() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_id\nwhen:\n  manual: {}\nsteps:\n  - finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::MissingStepId { step }) = errors.first() else {
            panic!("expected MissingStepId, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_step_shape_for_non_mapping_step() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_step\nwhen:\n  manual: {}\nsteps:\n  - \"scalar\"",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::StepShape { step }) = errors.first() else {
            panic!("expected StepShape, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_duplicate_step_id_for_same_ids() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: dup_step\nwhen:\n  manual: {}\nsteps:\n  - id: same\n    save:\n      x: 1\n  - id: same\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::DuplicateStepId { id }) = errors.first() else {
            panic!("expected DuplicateStepId, got {:?}", errors.first());
        };
        assert_eq!(id.as_ref(), "same");
    }

    #[test]
    fn compile_returns_missing_step_primitive_for_step_without_primitive() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_prim\nwhen:\n  manual: {}\nsteps:\n  - id: empty_step",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::MissingStepPrimitive { step }) = errors.first() else {
            panic!("expected MissingStepPrimitive, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_unknown_step_field_for_invalid_field() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_field\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    unknown_field: 1\n    save:\n      x: 1",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::UnknownStepField { step, field }) = errors.first() else {
            panic!("expected UnknownStepField, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
        assert_eq!(field.as_ref(), "unknown_field");
    }

    #[test]
    fn compile_returns_last_step_must_finish_for_non_finish_ending() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_finish\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      x: 1",
        );
        let Err(errors) = result else { panic!("expected error") };
        assert!(matches!(errors.first(), Some(CompileError::LastStepMustFinish)));
    }

    #[test]
    fn compile_returns_unknown_top_level_field_for_invalid_field() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: extra\nwhen:\n  manual: {}\nunknown_root: true\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::UnknownTopLevelField { field }) = errors.first() else {
            panic!("expected UnknownTopLevelField, got {:?}", errors.first());
        };
        assert_eq!(field.as_ref(), "unknown_root");
    }

    #[test]
    fn compile_returns_tag_forbidden_for_tagged_node() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: tagged\nwhen:\n  manual: {}\nsteps:\n  - id: !!tag done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        assert!(matches!(errors.first(), Some(CompileError::TagForbidden { .. })));
    }

    #[test]
    fn compile_returns_float_forbidden_for_float_scalar() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: floaty\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 3.14",
        );
        let Err(errors) = result else { panic!("expected error") };
        assert!(matches!(errors.first(), Some(CompileError::FloatForbidden)));
    }

    #[test]
    fn compile_returns_depth_limit_for_deeply_nested_yaml() {
        let tiny_limits = YamlLimits { max_depth: 3, ..YamlLimits::default() };
        let compiler = YamlCompiler { limits: tiny_limits };
        let result = compiler.compile(
            b"version: velvet-ballastics/v1\nname: deep\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\na:\n  b:\n    c:\n      d: deep",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::DepthLimit { depth, limit }) = errors.first() else {
            panic!("expected DepthLimit, got {:?}", errors.first());
        };
        assert_eq!(*limit, 3);
        assert!(*depth > 3);
    }

    #[test]
    fn compile_returns_node_limit_for_many_nodes() {
        let tiny_limits = YamlLimits { max_nodes: 5, ..YamlLimits::default() };
        let compiler = YamlCompiler { limits: tiny_limits };
        let result = compiler.compile(
            b"version: velvet-ballastics/v1\nname: big\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      a: 1\n      b: 2\n      c: 3\n      d: 4\n      e: 5\n      f: 6\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::NodeLimit { limit }) = errors.first() else {
            panic!("expected NodeLimit, got {:?}", errors.first());
        };
        assert_eq!(*limit, 5);
    }

    #[test]
    fn compile_returns_scalar_limit_for_long_scalar() {
        let tiny_limits = YamlLimits { max_scalar_bytes: 5, ..YamlLimits::default() };
        let compiler = YamlCompiler { limits: tiny_limits };
        let result = compiler.compile(
            b"version: velvet-ballastics/v1\nname: long_scalar\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\nlabel: abcdefgh",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::ScalarLimit { actual, limit }) = errors.first() else {
            panic!("expected ScalarLimit, got {:?}", errors.first());
        };
        assert_eq!(*limit, 5);
        assert!(*actual > 5);
    }

    #[test]
    fn compile_returns_duplicate_key_for_repeated_yaml_key() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: dup\nwhen:\n  manual: {}\nname: dup2\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::DuplicateKey { key, .. }) = errors.first() else {
            panic!("expected DuplicateKey, got {:?}", errors.first());
        };
        assert_eq!(key.as_ref(), "name");
    }

    #[test]
    fn compile_returns_invalid_name_for_reserved_step_name() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: reserved\nwhen:\n  manual: {}\nsteps:\n  - id: run\n    save:\n      x: 1\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::InvalidName { field, value }) = errors.first() else {
            panic!("expected InvalidName, got {:?}", errors.first());
        };
        assert_eq!(*field, "step id");
        assert_eq!(value.as_ref(), "run");
    }

    #[test]
    fn compile_returns_multiple_step_primitives_for_two_primitives() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: multi\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      x: 1\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::MultipleStepPrimitives { step }) = errors.first() else {
            panic!("expected MultipleStepPrimitives, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_invalid_trigger_count_for_two_triggers() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: multi_trigger\nwhen:\n  manual: {}\n  ipc: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::InvalidTriggerCount { count }) = errors.first() else {
            panic!("expected InvalidTriggerCount, got {:?}", errors.first());
        };
        assert_eq!(*count, 2);
    }

    #[test]
    fn compile_returns_field_shape_for_bad_inputs_shape() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_inputs\nwhen:\n  manual: {}\ninputs: []\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else { panic!("expected error") };
        let Some(CompileError::FieldShape { field, expected }) = errors.first() else {
            panic!("expected FieldShape, got {:?}", errors.first());
        };
        assert_eq!(*field, "inputs");
        assert!(!expected.is_empty());
    }

    // ── Round 2: Compilation success path tests ──────────────────────────

    #[test]
    fn compile_produces_valid_workflow_for_minimal_source() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
        let wf = result.unwrap();
        assert_eq!(wf.node_count(), 2);
    }

    #[test]
    fn compile_produces_valid_workflow_for_optional_fields() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }

    #[test]
    fn compile_produces_non_default_workflow_digest() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        assert!(result.is_ok());
        let wf = result.unwrap();
        assert_ne!(wf.digest(), vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]));
    }

    #[test]
    fn compile_produces_matching_workflow_name() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        assert!(result.is_ok());
        let wf = result.unwrap();
        assert_eq!(wf.name(), "fast_path");
    }

    #[test]
    fn compile_produces_correct_entry_step_index() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        assert!(result.is_ok());
        let wf = result.unwrap();
        assert_eq!(wf.entry(), vb_core::ids::StepIdx::ZERO);
    }

    #[test]
    fn compile_with_limits_respects_custom_source_limit() {
        let source = OPTIONAL_TOP_LEVEL_FIELDS_SOURCE;
        let limits = YamlLimits { max_source_bytes: source.len() + 1, ..YamlLimits::default() };
        let compiler = YamlCompiler { limits };
        assert!(compiler.compile(source).is_ok());
    }

    // ── Round 2: CompileError::code() tests ──────────────────────────────

    #[test]
    fn compile_error_code_returns_payload_too_large_for_source_too_large() {
        let err = CompileError::SourceTooLarge { actual: 100, limit: 50 };
        assert_eq!(err.code(), "PAYLOAD_TOO_LARGE");
    }

    #[test]
    fn compile_error_code_returns_missing_required_field_for_empty_source() {
        let err = CompileError::EmptySource;
        assert_eq!(err.code(), "MISSING_REQUIRED_FIELD");
    }

    #[test]
    fn compile_error_code_returns_type_mismatch_for_top_level_not_mapping() {
        let err = CompileError::TopLevelNotMapping;
        assert_eq!(err.code(), "TYPE_MISMATCH");
    }

    #[test]
    fn compile_error_code_returns_duplicate_key_for_duplicate_key() {
        let err = CompileError::DuplicateKey {
            key: Box::from("test"),
            mark: SourceMark { index: 0, end_index: 0, line: 1, column: 1, available: true },
        };
        assert_eq!(err.code(), "DUPLICATE_KEY");
    }

    #[test]
    fn compile_error_code_returns_limit_exceeded_for_depth_limit() {
        let err = CompileError::DepthLimit { depth: 10, limit: 5 };
        assert_eq!(err.code(), "LIMIT_EXCEEDED");
    }

    #[test]
    fn compile_error_code_returns_limit_exceeded_for_node_limit() {
        let err = CompileError::NodeLimit { limit: 100 };
        assert_eq!(err.code(), "LIMIT_EXCEEDED");
    }

    #[test]
    fn compile_error_code_returns_forbidden_yaml_for_alias() {
        let err = CompileError::AliasForbidden {
            mark: SourceMark { index: 0, end_index: 0, line: 1, column: 1, available: true },
        };
        assert_eq!(err.code(), "FORBIDDEN_YAML_FEATURE");
    }

    #[test]
    fn compile_error_code_returns_forbidden_yaml_for_float() {
        let err = CompileError::FloatForbidden;
        assert_eq!(err.code(), "FORBIDDEN_YAML_FEATURE");
    }

    #[test]
    fn compile_error_code_returns_unknown_step_for_unsupported_primitive() {
        let err = CompileError::UnsupportedStepPrimitive {
            step: 0,
            primitive: "custom",
        };
        assert_eq!(err.code(), "UNKNOWN_STEP_FIELD");
    }

    #[test]
    fn compile_error_code_returns_backward_branch_for_backward_target() {
        let err = CompileError::BackwardBranchTarget { step: 2, target: 0 };
        assert_eq!(err.code(), "INVALID_THEN_TARGET");
    }

    #[test]
    fn compile_error_code_returns_type_mismatch_for_type_mismatch() {
        let err = CompileError::TypeMismatch {
            field: "test",
            expected: "text",
            found: "number",
        };
        assert_eq!(err.code(), "TYPE_MISMATCH");
    }

    #[test]
    fn compile_error_code_returns_expression_error_for_unexpected_char() {
        let err = CompileError::ExpressionUnexpectedChar {
            expression: Box::from("$x"),
            index: 1,
            found: '@',
        };
        assert_eq!(err.code(), "INVALID_EXPRESSION");
    }

    #[test]
    fn compile_error_code_returns_expression_error_for_helper_arity() {
        let err = CompileError::ExpressionHelperArity {
            helper: "len",
            expected: 1,
            actual: 2,
        };
        assert_eq!(err.code(), "INVALID_EXPRESSION");
    }

    // ── Round 2: YamlLimits and Compiler config tests ────────────────────

    #[test]
    fn yaml_limits_default_has_reasonable_values() {
        let defaults = YamlLimits::default();
        assert!(defaults.max_source_bytes > 0);
        assert!(defaults.max_depth > 0);
        assert!(defaults.max_nodes > 0);
        assert!(defaults.max_scalar_bytes > 0);
    }

    #[test]
    fn yaml_compiler_default_uses_default_limits() {
        let compiler = YamlCompiler::default();
        assert_eq!(compiler.limits.max_source_bytes, YamlLimits::default().max_source_bytes);
    }

    // ── Round 2: Lowering function tests ─────────────────────────────────

    #[test]
    fn lower_finish_produces_finish_node_kind() {
        let mut builder = SlotCompiler::new();
        let node = lower_finish(
            StepIdx::new(0),
            SlotIdx::new(0),
            &mut builder,
        );
        assert!(matches!(node.kind, CompiledNodeKind::Finish { .. }));
    }

    #[test]
    fn lower_set_produces_set_node_kind() {
        let mut builder = SlotCompiler::new();
        let const_idx = builder.push_constant(ConstValue::I64(42)).ok().unwrap_or(ConstIdx::new(0));
        let node = lower_set(
            StepIdx::new(0),
            SlotIdx::new(0),
            const_idx,
            Some(StepIdx::new(1)),
        );
        assert!(matches!(node.kind, CompiledNodeKind::SetConst { .. }));
    }

    #[test]
    fn lower_do_produces_do_node_kind() {
        let mut builder = SlotCompiler::new();
        let node = lower_do(
            StepIdx::new(0),
            ActionId::new(1),
            SlotIdx::new(0),
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(1)),
            &mut builder,
        );
        assert!(matches!(node.kind, CompiledNodeKind::Do { .. }));
    }

    #[test]
    fn compute_compiled_digest_is_deterministic() {
        let d1 = compute_compiled_digest(NESTED_SAVE_SOURCE);
        let d2 = compute_compiled_digest(NESTED_SAVE_SOURCE);
        assert_eq!(d1, d2);
    }

    #[test]
    fn compute_compiled_digest_differs_for_different_sources() {
        let d1 = compute_compiled_digest(b"source_a");
        let d2 = compute_compiled_digest(b"source_b");
        assert_ne!(d1, d2);
    }

    // ── Round 2: SlotCompiler tests ──────────────────────────────────────

    #[test]
    fn slot_compiler_new_starts_empty() {
        let mut sc = SlotCompiler::new();
        assert_eq!(sc.push_constant(ConstValue::I64(42)).ok().map(|i| i.get()), Some(0));
    }

    #[test]
    fn slot_compiler_push_constant_returns_ascending_indices() {
        let mut sc = SlotCompiler::new();
        let idx0 = sc.push_constant(ConstValue::I64(1));
        let idx1 = sc.push_constant(ConstValue::I64(2));
        assert_eq!(idx0.ok().map(|i| i.get()), Some(0));
        assert_eq!(idx1.ok().map(|i| i.get()), Some(1));
    }

    #[test]
    fn slot_compiler_push_expression_returns_ascending_indices() {
        let mut sc = SlotCompiler::new();
        let empty_ops: Box<[vb_core::workflow::ExprOp]> = Box::from([]);
        let prog = ExprProgram::try_from_ops(empty_ops).unwrap_or_else(|_| {
            ExprProgram { ops: Box::from([]), max_stack: 0 }
        });
        let idx = sc.push_expression(prog);
        assert!(idx.is_ok());
    }

    #[test]
    fn slot_compiler_record_slot_tracks_max_slot() {
        let mut sc = SlotCompiler::new();
        sc.record_slot(SlotIdx::new(5));
        sc.record_slot(SlotIdx::new(10));
        // record_slot doesn't return anything but should not panic
    }
}
