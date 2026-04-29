//! Cold-path YAML compiler boundary.
//!
//! YAML enters the system only through this crate. The hot engine consumes only
//! `vb_core::CompiledWorkflow` values built from native Rust `saphyr` parsing.

use saphyr::{LoadableYamlNode, Yaml};
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

impl YamlCompiler {
    /// Creates a compiler with explicit strict-profile limits.
    #[must_use]
    pub const fn new(limits: YamlLimits) -> Self {
        Self { limits }
    }

    /// Parses and validates YAML, then emits compiled workflow IR.
    pub fn compile(&self, source: &[u8]) -> Result<CompiledWorkflow, CompileError> {
        let text = checked_utf8(source, self.limits)?;
        reject_yaml_indirection_markers(text)?;
        let docs = Yaml::load_from_str(text)?;
        let doc = single_document(&docs)?;
        validate_strict_profile(doc, self.limits)?;
        compile_validated_document(text, doc)
    }
}

impl Default for YamlCompiler {
    fn default() -> Self {
        Self::new(YamlLimits::default())
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
    #[error("mapping key must be a string")]
    NonStringKey,
    /// YAML anchors/aliases are forbidden.
    #[error("YAML aliases are forbidden")]
    AliasForbidden,
    /// YAML tags are forbidden.
    #[error("YAML tags are forbidden")]
    TagForbidden,
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
    /// Workflow field has the wrong YAML shape.
    #[error("workflow field {field} must be {expected}")]
    FieldShape {
        /// Field name.
        field: &'static str,
        /// Expected shape.
        expected: &'static str,
    },
    /// Workflow must contain at least one executable step.
    #[error("workflow steps must not be empty")]
    EmptySteps,
    /// Step must be a one-entry mapping.
    #[error("step {step} must be a one-entry mapping")]
    StepShape {
        /// Step index.
        step: usize,
    },
    /// Step kind is not supported by the minimal compiler.
    #[error("step {step} has unsupported kind: {kind}")]
    UnknownStepKind {
        /// Step index.
        step: usize,
        /// Unsupported kind.
        kind: Box<str>,
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
    Ok(str::from_utf8(source)?)
}

fn single_document<'a>(docs: &'a [Yaml<'a>]) -> Result<&'a Yaml<'a>, CompileError> {
    match docs {
        [doc] => Ok(doc),
        _ => Err(CompileError::DocumentCount { count: docs.len() }),
    }
}

fn reject_yaml_indirection_markers(text: &str) -> Result<(), CompileError> {
    for line in text.lines() {
        let mut single_quoted = false;
        let mut double_quoted = false;
        let mut escaped = false;

        for ch in line.chars() {
            if escaped {
                escaped = false;
            } else if double_quoted && ch == '\\' {
                escaped = true;
            } else if !double_quoted && ch == '\'' {
                single_quoted = !single_quoted;
            } else if !single_quoted && ch == '"' {
                double_quoted = !double_quoted;
            } else if !single_quoted && !double_quoted && ch == '#' {
                break;
            } else if !single_quoted && !double_quoted && matches!(ch, '&' | '*') {
                return Err(CompileError::AliasForbidden);
            } else if !single_quoted && !double_quoted && ch == '!' {
                return Err(CompileError::TagForbidden);
            }
        }
    }
    Ok(())
}

fn validate_strict_profile(root: &Yaml<'_>, limits: YamlLimits) -> Result<(), CompileError> {
    if !root.is_mapping() {
        return Err(CompileError::TopLevelNotMapping);
    }

    let mut stack = vec![(root, 0_u16)];
    let mut visited = 0_u32;

    while let Some((node, depth)) = stack.pop() {
        visited = visited.checked_add(1).ok_or(CompileError::NodeLimit {
            limit: limits.max_nodes,
        })?;
        if visited > limits.max_nodes {
            return Err(CompileError::NodeLimit {
                limit: limits.max_nodes,
            });
        }
        if depth > limits.max_depth {
            return Err(CompileError::DepthLimit {
                depth,
                limit: limits.max_depth,
            });
        }
        validate_one_node(node, depth, limits, &mut stack)?;
    }

    Ok(())
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
        Yaml::Tagged(_, _) => Err(CompileError::TagForbidden),
        Yaml::Alias(_) => Err(CompileError::AliasForbidden),
        Yaml::BadValue => Err(CompileError::BadValue),
        Yaml::Value(value) => validate_scalar(value, limits),
        Yaml::Representation(value, _, tag) => {
            if tag.is_some() {
                return Err(CompileError::TagForbidden);
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
    if mapping.len() > limits.max_mapping_entries {
        return Err(CompileError::MappingLimit {
            actual: mapping.len(),
            limit: limits.max_mapping_entries,
        });
    }
    let next_depth = depth.checked_add(1).ok_or(CompileError::DepthLimit {
        depth,
        limit: limits.max_depth,
    })?;
    for (key, value) in mapping {
        validate_mapping_key(key, limits)?;
        stack.push((value, next_depth));
    }
    Ok(())
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

fn validate_mapping_key(key: &Yaml<'_>, limits: YamlLimits) -> Result<(), CompileError> {
    match key.as_str() {
        Some(value) => validate_scalar_len(value, limits),
        None => Err(CompileError::NonStringKey),
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
    let name = required_string_field(doc, "name")?;
    let steps = required_sequence_field(doc, "steps")?;
    if steps.is_empty() {
        return Err(CompileError::EmptySteps);
    }

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

fn validate_top_level_keys(doc: &Yaml<'_>) -> Result<(), CompileError> {
    let Some(mapping) = doc.as_mapping() else {
        return Err(CompileError::TopLevelNotMapping);
    };
    for (key, _) in mapping {
        let Some(field) = key.as_str() else {
            return Err(CompileError::NonStringKey);
        };
        if !matches!(field, "name" | "steps") {
            return Err(CompileError::UnknownTopLevelField {
                field: Box::<str>::from(field),
            });
        }
    }
    Ok(())
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
    let (kind, body) = single_step_mapping(step, index)?;
    match kind {
        "set" => compile_set(body, index, last_step, builder),
        "copy" => compile_copy(body, index, last_step, builder),
        "choose" => compile_choose(body, index, last_step, builder),
        "finish" => compile_finish(body, index, last_step, builder),
        value => Err(CompileError::UnknownStepKind {
            step: index,
            kind: Box::<str>::from(value),
        }),
    }
}

fn single_step_mapping<'a>(
    step: &'a Yaml<'a>,
    index: usize,
) -> Result<(&'a str, &'a Yaml<'a>), CompileError> {
    let Some(mapping) = step.as_mapping() else {
        return Err(CompileError::StepShape { step: index });
    };
    if mapping.len() != 1 {
        return Err(CompileError::StepShape { step: index });
    }
    let mut pairs = mapping.iter();
    let Some((key, body)) = pairs.next() else {
        return Err(CompileError::StepShape { step: index });
    };
    let Some(kind) = key.as_str() else {
        return Err(CompileError::StepShape { step: index });
    };
    Ok((kind, body))
}

fn compile_set(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    let output = required_slot(body, index, "slot")?;
    let value_node = required_step_field(body, index, "value")?;
    let constant = slot_value(value_node, index)?;
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

fn compile_copy(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    let source = required_slot(body, index, "from")?;
    let output = required_slot(body, index, "to")?;
    builder.record_slot(source);
    builder.record_slot(output);
    Ok(CompiledNode {
        kind: CompiledNodeKind::Copy {
            output,
            source,
            next: next_step(index)?,
        },
    })
}

fn compile_choose(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
    builder: &mut WorkflowBuilder,
) -> Result<CompiledNode, CompileError> {
    reject_last_non_finish(index, last_step)?;
    let condition = required_slot(body, index, "condition")?;
    let on_true = required_branch_target(body, index, "on_true")?;
    let on_false = required_branch_target(body, index, "on_false")?;
    reject_backward_branch(index, on_true)?;
    reject_backward_branch(index, on_false)?;
    builder.record_slot(condition);
    Ok(CompiledNode {
        kind: CompiledNodeKind::Choose {
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

fn required_step_field<'a>(
    body: &'a Yaml<'a>,
    step: usize,
    field: &'static str,
) -> Result<&'a Yaml<'a>, CompileError> {
    body.as_mapping_get(field)
        .ok_or(CompileError::MissingStepField { step, field })
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
        Yaml::Value(saphyr::Scalar::String(value)) => {
            Ok(SlotValue::Text(Box::<str>::from(value.as_ref())))
        }
        Yaml::Representation(value, _, None) => {
            Ok(SlotValue::Text(Box::<str>::from(value.as_ref())))
        }
        _ => Err(CompileError::UnsupportedConstantValue { step }),
    }
}

#[cfg(test)]
mod tests {
    use super::{CompileError, YamlCompiler, YamlLimits};
    use vb_core::{
        EngineSignal, RunFrame, RunId, SlotValue, StepBudget, engine::run_until_blocked,
    };

    #[test]
    fn compiler_executes_compiled_set_and_finish_steps() {
        let source = br#"
name: fast_path
steps:
  - set:
      slot: 0
      value: done
  - finish:
      result: 0
"#;
        let result = YamlCompiler::default().compile(source);

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"),
            "compiler should build executable workflow"
        );
        let Ok(workflow) = result else {
            return;
        };
        let mut run = RunFrame::new(RunId::new(9), &workflow);
        let signal = run_until_blocked(&workflow, &mut run, StepBudget::MAX);

        assert_eq!(
            signal,
            Ok(EngineSignal::Finished(SlotValue::Text(Box::<str>::from(
                "done"
            ))))
        );
    }

    #[test]
    fn compiler_rejects_empty_steps() {
        let result = YamlCompiler::default().compile(b"name: fast_path\nsteps: []\n");

        assert!(matches!(result, Err(CompileError::EmptySteps)));
    }

    #[test]
    fn compiler_rejects_unsupported_top_level_fields() {
        let result = YamlCompiler::default()
            .compile(b"name: fast_path\nresult: {}\nsteps:\n  - finish:\n      result: 0\n");

        assert!(matches!(
            result,
            Err(CompileError::UnknownTopLevelField { .. })
        ));
    }

    #[test]
    fn compiler_rejects_backward_branch_targets() {
        let result = YamlCompiler::default().compile(
            b"name: fast_path\nsteps:\n  - choose:\n      condition: 0\n      on_true: 0\n      on_false: 1\n  - finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(CompileError::BackwardBranchTarget { .. })
        ));
    }

    #[test]
    fn compiler_rejects_aliases() {
        let result = YamlCompiler::default().compile(b"name: &n fast\ncopy: *n\n");

        assert!(matches!(result, Err(CompileError::AliasForbidden)));
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
}
