//! Cold-path YAML compiler boundary.
//!
//! YAML enters the system only through this crate. The hot engine consumes only
//! `vb_core::CompiledWorkflow` values built from native Rust `saphyr` parsing.

pub mod ast;
mod schema;
pub mod strict_yaml;

use saphyr::{LoadableYamlNode, Yaml};
use saphyr_parser::{Event, Parser, Span, StrInput};
use std::collections::HashSet;
use std::str;
use thiserror::Error;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, SlotIdx, SlotValue, StepIdx,
    WorkflowDigest, WorkflowError, WorkflowParts,
};

const DEFAULT_MAX_SOURCE_BYTES: usize = 1_048_576;
const DEFAULT_MAX_DEPTH: u16 = 64;
const DEFAULT_MAX_NODES: u32 = 100_000;
const DEFAULT_MAX_SEQUENCE_LEN: usize = 10_000;
const DEFAULT_MAX_MAPPING_ENTRIES: usize = 1_024;
const DEFAULT_MAX_SCALAR_BYTES: usize = 65_536;
const WORKFLOW_VERSION: &str = "velvet/v1";

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
/// `column` are one-indexed parser marks. Tree-only validation paths use
/// [`SourceMark::unavailable`] because `saphyr::Yaml` nodes do not retain marks.
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
    pub fn compile(&self, source: &[u8]) -> Result<CompiledWorkflow, CompileError> {
        let text = checked_utf8(source, self.limits)?;
        strict_yaml::reject_unsupported_profile_events(text)?;
        reject_duplicate_mapping_keys(text)?;
        let docs = Yaml::load_from_str(text)?;
        let doc = single_document(&docs)?;
        validate_strict_profile(doc, self.limits)?;
        let workflow = compile_validated_document(text, doc)?;
        schema::validate_input_schemas(doc)?;
        Ok(workflow)
    }

    /// Parses strict YAML into the cold typed AST without emitting runtime IR.
    pub fn parse_ast(&self, source: &[u8]) -> Result<ast::WorkflowAst, CompileError> {
        let text = checked_utf8(source, self.limits)?;
        strict_yaml::reject_unsupported_profile_events(text)?;
        reject_duplicate_mapping_keys(text)?;
        let docs = Yaml::load_from_str(text)?;
        let doc = single_document(&docs)?;
        validate_strict_profile(doc, self.limits)?;
        let _ = compile_validated_document(text, doc)?;
        schema::validate_input_schemas(doc)?;
        ast::parse_workflow_ast(text, doc)
    }
}

impl Default for YamlCompiler {
    fn default() -> Self {
        Self::new(YamlLimits::default())
    }
}

fn non_string_key_error() -> CompileError {
    CompileError::NonStringKey {
        mark: SourceMark::unavailable(),
    }
}

/// YAML compiler errors.
#[derive(Debug, Error)]
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
        let key = mapping_key_text(key_event, key_mark)?;
        let duplicate = key.clone();
        if !seen.insert(key) {
            return Err(CompileError::DuplicateKey {
                key: duplicate,
                mark: SourceMark::from_parser_span(key_mark),
            });
        }

        let Some((value_event, value_mark)) = parser.next_event().transpose()? else {
            return Ok(());
        };
        validate_duplicate_keys_in_started_node(value_event, value_mark, parser)?;
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
            if tag.is_some() {
                return Err(CompileError::TagForbidden {
                    mark: SourceMark::unavailable(),
                });
            }
            validate_scalar_len(value.as_ref(), limits)
        }
    }
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

fn compile_validated_document(
    text: &str,
    doc: &Yaml<'_>,
) -> Result<CompiledWorkflow, CompileError> {
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

    let digest = WorkflowDigest::from_bytes(blake3::hash(text.as_bytes()).into());
    let mut builder = WorkflowBuilder::new();
    let last_step = steps.len().checked_sub(1).ok_or(CompileError::EmptySteps)?;

    for (index, step) in steps.iter().enumerate() {
        let node = compile_step(step, index, last_step, &mut builder)?;
        builder.nodes.push(node);
    }
    let parts = WorkflowParts {
        name: Box::<str>::from(name),
        digest,
        slot_count: builder.slot_count()?,
        nodes: builder.nodes.into_boxed_slice(),
        constants: builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
    };
    Ok(CompiledWorkflow::try_from_parts(parts)?)
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

fn is_reserved_name(value: &str) -> bool {
    matches!(
        value,
        "input"
            | "inputs"
            | "vars"
            | "secrets"
            | "steps"
            | "result"
            | "when"
            | "item"
            | "error"
            | "summary"
            | "cursor"
            | "page"
            | "event"
            | "attempt"
            | "attempts"
            | "true"
            | "false"
            | "null"
            | "run"
            | "save"
            | "choose"
            | "for_each"
            | "together"
            | "gather"
            | "summarize"
            | "repeat"
            | "wait"
            | "ask"
            | "try_again"
            | "on_error"
            | "then"
            | "finish"
    )
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
    constants: Vec<SlotValue>,
    max_slot: Option<usize>,
}

impl WorkflowBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn push_constant(&mut self, value: SlotValue) -> Result<ConstIdx, CompileError> {
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
) -> Result<CompiledNode, CompileError> {
    let StepSpec { primitive, body } = step_spec(step, index)?;
    match primitive {
        StepPrimitive::Save => compile_save(body, index, last_step, builder),
        StepPrimitive::Choose => compile_choose(body, index, last_step, builder),
        StepPrimitive::Finish => compile_finish(body, index, last_step, builder),
        value => Err(CompileError::UnsupportedStepPrimitive {
            step: index,
            primitive: value.as_str(),
        }),
    }
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
    if !body.is_mapping() {
        return Err(CompileError::StepFieldShape {
            step: index,
            field: "save",
            expected: "an object",
        });
    }
    let output = slot_idx_for_step(index)?;
    let constant = save_slot_value(body, index)?;
    let constant = builder.push_constant(constant)?;
    builder.record_slot(output);
    Ok(CompiledNode {
        kind: CompiledNodeKind::SetConst {
            output,
            value: constant,
            next: next_step(index)?,
        },
    })
}

fn save_slot_value(body: &Yaml<'_>, step: usize) -> Result<SlotValue, CompileError> {
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
    let condition = required_slot(body, index, "condition")?;
    let on_true = required_branch_target(body, index, "on_true")?;
    let on_false = required_branch_target(body, index, "on_false")?;
    reject_backward_branch(index, on_true)?;
    reject_backward_branch(index, on_false)?;
    builder.record_slot(condition);
    Ok(CompiledNode {
        kind: CompiledNodeKind::ChooseSlot {
            condition,
            on_true,
            on_false,
        },
    })
}

fn compile_finish(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    if index != last_step {
        return Err(CompileError::StepFieldShape {
            step: index,
            field: "finish",
            expected: "the last step",
        });
    }
    reject_unknown_primitive_fields(body, index, "finish", &["result"])?;
    let result = required_slot(body, index, "result")?;
    builder.record_slot(result);
    Ok(CompiledNode {
        kind: CompiledNodeKind::Finish { result },
    })
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
    let Some(mapping) = body.as_mapping() else {
        return Err(CompileError::StepFieldShape {
            step,
            field: primitive,
            expected: "a mapping",
        });
    };
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            return Err(CompileError::StepShape { step });
        };
        if !allowed.contains(&field) {
            return Err(CompileError::UnknownStepPrimitiveField {
                step,
                primitive,
                field: Box::<str>::from(field),
            });
        }
    }
    Ok(())
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

fn reject_backward_branch(step: usize, target: StepIdx) -> Result<(), CompileError> {
    let target = target.as_usize();
    if target <= step {
        Err(CompileError::BackwardBranchTarget { step, target })
    } else {
        Ok(())
    }
}

fn slot_value(node: &Yaml<'_>, step: usize) -> Result<SlotValue, CompileError> {
    match node {
        Yaml::Value(saphyr::Scalar::Null) => Ok(SlotValue::Null),
        Yaml::Value(saphyr::Scalar::Boolean(value)) => Ok(SlotValue::Bool(*value)),
        Yaml::Value(saphyr::Scalar::Integer(value)) => Ok(SlotValue::I64(*value)),
        Yaml::Value(saphyr::Scalar::String(value)) => text_slot_value(value.as_ref(), step),
        Yaml::Representation(value, _, None) => text_slot_value(value.as_ref(), step),
        Yaml::Sequence(sequence) => list_slot_value(sequence, step),
        Yaml::Mapping(mapping) => object_slot_value(mapping, step),
        _ => Err(CompileError::UnsupportedConstantValue { step }),
    }
}

fn text_slot_value(_value: &str, step: usize) -> Result<SlotValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

fn list_slot_value(
    _sequence: &saphyr::Sequence<'_>,
    step: usize,
) -> Result<SlotValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

fn object_slot_value(
    _mapping: &saphyr::Mapping<'_>,
    step: usize,
) -> Result<SlotValue, CompileError> {
    Err(CompileError::UnsupportedConstantValue { step })
}

#[cfg(test)]
mod tests {
    use super::{CompileError, YamlCompiler, YamlLimits};
    use vb_core::CompiledWorkflow;

    fn compile_with_inputs(inputs: &str) -> Result<CompiledWorkflow, CompileError> {
        let source = format!(
            "version: velvet/v1\nname: schema_case\nwhen:\n  manual: {{}}\ninputs:\n{inputs}steps:\n  - id: done\n    finish:\n      result: 0\n"
        );
        YamlCompiler::default().compile(source.as_bytes())
    }

    fn compile_error_text(source: &[u8]) -> String {
        match YamlCompiler::default().compile(source) {
            Ok(_) => "compile unexpectedly succeeded".to_owned(),
            Err(error) => error.to_string(),
        }
    }

    fn parse_ast_error_text(source: &[u8]) -> String {
        match YamlCompiler::default().parse_ast(source) {
            Ok(_) => "parse_ast unexpectedly succeeded".to_owned(),
            Err(error) => error.to_string(),
        }
    }

    fn assert_compile_parse_first_error(source: &[u8]) {
        assert_eq!(compile_error_text(source), parse_ast_error_text(source));
    }

    #[test]
    fn compiler_rejects_save_object_until_handle_arenas_exist() {
        let source = br#"
version: velvet/v1
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
            Err(CompileError::UnsupportedConstantValue { step: 0 })
        ));
    }

    #[test]
    fn compiler_rejects_nested_save_values_until_handle_arenas_exist() {
        let source = br#"
version: velvet/v1
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
        let result = YamlCompiler::default().compile(source);

        assert!(matches!(
            result,
            Err(CompileError::UnsupportedConstantValue { step: 0 })
        ));
    }

    #[test]
    fn compiler_rejects_scalar_save_body() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save: done\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::StepFieldShape { field: "save", .. })
        ));
    }

    #[test]
    fn compiler_rejects_save_references_until_expressions_exist() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      text: $input.value\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::UnsupportedConstantValue { .. })
        ));
    }

    #[test]
    fn compiler_rejects_empty_steps() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps: []\n");

        assert!(matches!(result, Err(CompileError::EmptySteps)));
    }

    #[test]
    fn compiler_rejects_unsupported_top_level_fields() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nunexpected: true\nsteps:\n  - finish:\n      result: 0\n");

        assert!(matches!(
            result,
            Err(CompileError::UnknownTopLevelField { .. })
        ));
    }

    #[test]
    fn compiler_rejects_missing_workflow_version() {
        let result = YamlCompiler::default().compile(
            b"name: fast_path\nwhen:\n  manual: {}\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(matches!(result, Err(CompileError::MissingField { .. })));
    }

    #[test]
    fn compiler_rejects_non_canonical_workflow_version() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(matches!(result, Err(CompileError::InvalidVersion { .. })));
    }

    #[test]
    fn compiler_accepts_optional_top_level_fields() {
        let source = br#"
version: velvet/v1
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
        let result = YamlCompiler::default().compile(source);

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
                matches!(result, Err(CompileError::InvalidInputSchema { .. })),
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
                "version: velvet/v1\nname: schema_case\nwhen:\n  manual: {{}}\ninputs:\n{inputs}steps:\n  - id: done\n    finish:\n      result: 0\n"
            );

            assert_compile_parse_first_error(source.as_bytes());
        }
    }

    #[test]
    fn schema_validation_does_not_preempt_yaml_profile_errors() {
        assert_compile_parse_first_error(
            b"version: velvet/v1\nname: &n schema_case\ninputs:\n  value: integer\ncopy: *n\n",
        );
    }

    #[test]
    fn schema_validation_does_not_preempt_duplicate_key_errors() {
        assert_compile_parse_first_error(
            b"version: velvet/v1\nversion: velvet/v1\nname: schema_case\ninputs:\n  value: integer\n",
        );
    }

    #[test]
    fn schema_validation_does_not_preempt_lowering_errors() {
        let source = b"version: velvet/v1\nname: schema_case\nwhen:\n  manual: {}\ninputs:\n  value: integer\nsteps:\n  - id: route\n    choose: true\n";

        assert_eq!(
            compile_error_text(source),
            CompileError::LastStepMustFinish.to_string()
        );
        assert_compile_parse_first_error(source);
    }

    #[test]
    fn schema_validation_does_not_preempt_finish_position_errors() {
        let source = b"version: velvet/v1\nname: schema_case\nwhen:\n  manual: {}\ninputs:\n  value: integer\nsteps:\n  - id: early\n    finish:\n      result: 0\n      status: success\n  - id: done\n    finish:\n      result: 0\n";

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
                Err(CompileError::UnknownInputSchemaField { .. })
            ));
        }
    }

    #[test]
    fn compiler_rejects_pattern_until_bounded_regex_exists() {
        let result = compile_with_inputs("  value:\n    is: text\n    pattern: ^[a-z]+$\n");

        assert!(matches!(
            result,
            Err(CompileError::InvalidInputSchema {
                field: "inputs.pattern",
                ..
            })
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
                matches!(result, Err(CompileError::InvalidInputSchema { .. })),
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
                Err(CompileError::InvalidInputSchema { .. })
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
                Err(CompileError::InvalidInputSchema { .. })
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
            Err(CompileError::InvalidInputSchema { .. })
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
                matches!(result, Err(CompileError::InvalidInputSchema { .. })),
                "invalid bounds should be rejected: {inputs}"
            );
        }
    }

    #[test]
    fn compiler_rejects_non_mapping_optional_top_level_fields() {
        for field in ["inputs", "vars", "secrets"] {
            let source = format!(
                "version: velvet/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(CompileError::FieldShape { .. })),
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
                "version: velvet/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}:\n  {key}: value\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(CompileError::InvalidName { .. })),
                "{field}.{key} must use Velvet v1 public naming"
            );
        }
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_shapes() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\ninputs:\n  value:\n    - text\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Err(CompileError::FieldShape { .. })));
    }

    #[test]
    fn compiler_rejects_runtime_references_in_vars() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nvars:\n  label: $input.value\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::UnsupportedConstantValue { .. })
        ));
    }

    #[test]
    fn compiler_rejects_non_string_secret_bindings() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsecrets:\n  api_key: 42\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Err(CompileError::FieldShape { .. })));
    }

    #[test]
    fn compiler_rejects_invalid_examples_shape() {
        for examples in ["true", "\n  - fixture"] {
            let source = format!(
                "version: velvet/v1\nname: fast_path\nwhen:\n  manual: {{}}\nexamples: {examples}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(CompileError::FieldShape { .. })),
                "examples must be a sequence of mappings"
            );
        }
    }

    #[test]
    fn compiler_rejects_examples_without_valid_names() {
        for examples in ["\n  - input: {}", "\n  - name: 42", "\n  - name: run"] {
            let source = format!(
                "version: velvet/v1\nname: fast_path\nwhen:\n  manual: {{}}\nexamples: {examples}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(CompileError::MissingField { .. }
                        | CompileError::FieldShape { .. }
                        | CompileError::InvalidName { .. })
                ),
                "examples must declare valid fixture names"
            );
        }
    }

    #[test]
    fn compiler_rejects_non_empty_top_level_result_until_result_ir_exists() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nresult:\n  value: $build_result.value\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::UnsupportedTopLevelResult)
        ));
    }

    #[test]
    fn compiler_rejects_non_mapping_top_level_result() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nresult: done\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::FieldShape {
                field: "result",
                ..
            })
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_names() {
        for name in ["", "FastPath", "fast-path", "run"] {
            let source = format!(
                "version: velvet/v1\nname: \"{name}\"\nwhen:\n  manual: {{}}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(CompileError::InvalidName { field: "name", .. })),
                "workflow name {name:?} must be rejected"
            );
        }
    }

    #[test]
    fn compiler_rejects_missing_step_ids() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Err(CompileError::MissingStepId { .. })));
    }

    #[test]
    fn compiler_rejects_invalid_step_ids() {
        for id in ["", "BuildResult", "build-result", "finish"] {
            let source = format!(
                "version: velvet/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: \"{id}\"\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(CompileError::InvalidName {
                        field: "step id",
                        ..
                    })
                ),
                "step id {id:?} must be rejected"
            );
        }
    }

    #[test]
    fn compiler_rejects_duplicate_step_ids() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: duplicate\n    save:\n      value: 1\n  - id: duplicate\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Err(CompileError::DuplicateStepId { .. })));
    }

    #[test]
    fn compiler_accepts_step_display_name_metadata() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    name: Build Result\n    save:\n      value: 1\n  - id: done\n    name: Done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"));
    }

    #[test]
    fn compiler_rejects_non_string_step_display_name() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    name: 42\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::StepFieldShape { field: "name", .. })
        ));
    }

    #[test]
    fn compiler_rejects_unsupported_phase_zero_step_control_fields() {
        for control in ["if", "with", "try_again", "on_error", "then"] {
            let source = format!(
                "version: velvet/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: build_result\n    {control}: true\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(CompileError::UnsupportedStepControlField { .. })
                ),
                "control field {control} must be rejected until Phase 0 compiles it"
            );
        }
    }

    #[test]
    fn compiler_rejects_missing_workflow_trigger() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(matches!(result, Err(CompileError::MissingField { .. })));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_shapes() {
        for source in [
            b"version: velvet/v1\nname: fast_path\nwhen: manual\nsteps:\n  - finish:\n      result: 0\n".as_slice(),
            b"version: velvet/v1\nname: fast_path\nwhen: {}\nsteps:\n  - finish:\n      result: 0\n",
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\n  event: {}\nsteps:\n  - finish:\n      result: 0\n",
        ] {
            let result = YamlCompiler::default().compile(source);

            assert!(matches!(
                result,
                Err(CompileError::FieldShape { .. } | CompileError::InvalidTriggerCount { .. })
            ));
        }
    }

    #[test]
    fn compiler_rejects_unknown_workflow_trigger_kind() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  file: {}\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::UnknownTriggerKind { .. })
        ));
    }

    #[test]
    fn compiler_rejects_scalar_workflow_trigger_config() {
        for trigger in ["manual", "webhook", "schedule", "event"] {
            let source = format!(
                "version: velvet/v1\nname: fast_path\nwhen:\n  {trigger}: true\nsteps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(CompileError::TriggerShape { .. })),
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
                "version: velvet/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
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
                "version: velvet/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(matches!(
                result,
                Err(CompileError::UnknownTriggerField { .. })
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
                "version: velvet/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(matches!(
                result,
                Err(CompileError::MissingTriggerField { .. })
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
                "version: velvet/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(matches!(
                result,
                Err(CompileError::InvalidTriggerField { .. })
            ));
        }
    }

    #[test]
    fn compiler_rejects_backward_branch_targets() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose:\n      condition: 0\n      on_true: 0\n      on_false: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::BackwardBranchTarget { .. })
        ));
    }

    #[test]
    fn compiler_rejects_extra_phase_zero_choose_fields() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose:\n      condition: 0\n      on_true: 1\n      on_false: 1\n      otherwise: true\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::UnknownStepPrimitiveField {
                primitive: "choose",
                ..
            })
        ));
    }

    #[test]
    fn compiler_rejects_non_mapping_phase_zero_choose_body() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose: true\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::StepFieldShape {
                field: "choose",
                ..
            })
        ));
    }

    #[test]
    fn compiler_rejects_extra_phase_zero_finish_fields() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n      status: success\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::UnknownStepPrimitiveField {
                primitive: "finish",
                ..
            })
        ));
    }

    #[test]
    fn compiler_rejects_non_mapping_phase_zero_finish_body() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish: success\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::StepFieldShape {
                field: "finish",
                ..
            })
        ));
    }

    #[test]
    fn compiler_rejects_aliases() {
        let result =
            YamlCompiler::default().compile(b"version: velvet/v1\nname: &n fast\ncopy: *n\n");

        assert!(matches!(
            result,
            Err(CompileError::AnchorForbidden { mark }) if mark.available
        ));
    }

    #[test]
    fn compiler_rejects_custom_tags_with_mark() {
        let result = YamlCompiler::default().compile(b"version: !custom velvet/v1\n");

        assert!(matches!(
            result,
            Err(CompileError::TagForbidden { mark }) if mark.available
        ));
    }

    #[test]
    fn compiler_rejects_non_string_object_keys_with_mark() {
        let result = YamlCompiler::default().compile(b"? [bad]\n: value\n");

        assert!(matches!(
            result,
            Err(CompileError::NonStringKey { mark }) if mark.available
        ));
    }

    #[test]
    fn compiler_rejects_duplicate_top_level_keys() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nversion: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Err(CompileError::DuplicateKey { .. })));
    }

    #[test]
    fn compiler_rejects_duplicate_nested_keys() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      text: first\n      text: second\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Err(CompileError::DuplicateKey { .. })));
    }

    #[test]
    fn compiler_rejects_legacy_step_aliases() {
        for alias in ["do", "set", "collect", "reduce", "copy"] {
            let source = format!(
                "version: velvet/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: legacy\n    {alias}:\n      slot: 0\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(CompileError::UnknownStepField { .. })),
                "legacy alias {alias} must be rejected"
            );
        }
    }

    #[test]
    fn compiler_rejects_missing_step_primitive() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: only_metadata\n    name: Only Metadata\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::MissingStepPrimitive { .. })
        ));
    }

    #[test]
    fn compiler_rejects_multiple_step_primitives() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      slot: 0\n      value: 1\n    finish:\n      result: 0\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::MultipleStepPrimitives { .. })
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
                "version: velvet/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: unsupported\n    {primitive}: noop\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(CompileError::UnsupportedStepPrimitive { .. })),
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

        assert!(matches!(result, Err(CompileError::SourceTooLarge { .. })));
    }

    #[test]
    fn compiler_accepts_minimal_strict_workflow() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: strict_minimal\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "strict_minimal"));
    }

    #[test]
    fn compiler_rejects_empty_yaml_source() {
        let result = YamlCompiler::default().compile(b"   \n\t  ");

        assert!(matches!(result, Err(CompileError::EmptySource)));
    }

    #[test]
    fn compiler_rejects_multiple_yaml_documents() {
        let result = YamlCompiler::default().compile(
            b"---\nversion: velvet/v1\nname: first\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n---\nversion: velvet/v1\nname: second\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::DocumentCount { count: 2 })
        ));
    }

    #[test]
    fn compiler_rejects_yaml_merge_keys() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: merge_key\nwhen:\n  manual: {}\n<<:\n  steps: []\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::MergeKeyForbidden { .. })
        ));
    }
}
