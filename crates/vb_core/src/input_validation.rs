//! Bounded checks for retained input contracts before acceptance.

use crate::input_schema::MAX_INPUT_SCHEMA_DEPTH;
use crate::limits::MAX_VALUES_PER_RUN;
use crate::value::SlotValue;
use crate::value_store::ValueStoreCheckpointV1;
use crate::workflow::{CompiledInputConstraint, CompiledInputSlot, InputSlotKind};

/// A supplied value does not satisfy its retained input declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InputValueIssue {
    /// Value and declared kind disagree.
    Kind,
    /// Number or payload length exceeds its bounds.
    Bounds,
    /// An arena reference cannot be resolved from owned payloads.
    Payload,
    /// A list member has the wrong kind.
    Element,
    /// A closed object contains an undeclared field.
    ExtraField,
    /// A required or defaulted field has not been materialized.
    RequiredField,
    /// Input traversal exceeds the path bound.
    DepthExceeded,
    /// Traversal capacity or allocation was exhausted.
    Capacity,
}

/// Classifies a value without resolving arena references.
#[must_use]
pub const fn input_value_kind(value: SlotValue) -> InputSlotKind {
    match value {
        SlotValue::Null => InputSlotKind::Null,
        SlotValue::Bool(_) => InputSlotKind::Bool,
        SlotValue::I64(_) => InputSlotKind::I64,
        SlotValue::F64(_) => InputSlotKind::F64,
        SlotValue::Symbol(_) => InputSlotKind::Symbol,
        SlotValue::List(_) => InputSlotKind::List,
        SlotValue::Object(_) => InputSlotKind::Object,
        SlotValue::Blob(_) => InputSlotKind::Blob,
    }
}

struct Check<'a> {
    kind: InputSlotKind,
    constraint: &'a CompiledInputConstraint,
    value: SlotValue,
    depth: usize,
}

/// Validates complete values, including every nested payload reference.
///
/// Text lengths are UTF-8 bytes. Symbol references require backing payloads.
/// Defaults must already
/// be materialized by the owner of the input checkpoint.
pub fn validate_input_value(
    declaration: &CompiledInputSlot,
    value: SlotValue,
    _symbols_count: u32,
    checkpoint: Option<&ValueStoreCheckpointV1>,
) -> Result<(), InputValueIssue> {
    let any = CompiledInputConstraint::unconstrained();
    let mut pending = Vec::new();
    push(
        &mut pending,
        Check {
            kind: declaration.kind,
            constraint: &declaration.constraint,
            value,
            depth: 0,
        },
    )?;
    let mut visited = 0usize;
    while let Some(check) = pending.pop() {
        visited = visited.checked_add(1).ok_or(InputValueIssue::Capacity)?;
        if visited > MAX_VALUES_PER_RUN {
            return Err(InputValueIssue::Capacity);
        }
        if check.depth > MAX_INPUT_SCHEMA_DEPTH {
            return Err(InputValueIssue::DepthExceeded);
        }
        if check.value == SlotValue::Null && check.constraint.nullable {
            continue;
        }
        if check.kind != InputSlotKind::Any && check.kind != input_value_kind(check.value) {
            return Err(InputValueIssue::Kind);
        }
        match check.value {
            SlotValue::I64(number) => {
                check_bounds(number, check.constraint.min, check.constraint.max)?
            }
            SlotValue::Symbol(id) => {
                if let Some(cp) = checkpoint {
                    let index = usize::try_from(id.get()).map_err(|_| InputValueIssue::Payload)?;
                    let text = cp.symbols.get(index).ok_or(InputValueIssue::Payload)?;
                    check_length(
                        text.len(),
                        check.constraint.min_length,
                        check.constraint.max_length,
                    )?;
                } else {
                    return Err(InputValueIssue::Payload);
                }
            }
            SlotValue::List(id) => {
                let cp = checkpoint.ok_or(InputValueIssue::Payload)?;
                let index = usize::try_from(id.get()).map_err(|_| InputValueIssue::Payload)?;
                let items = cp.lists.get(index).ok_or(InputValueIssue::Payload)?;
                check_length(items.len(), check.constraint.min, check.constraint.max)?;
                let kind = check.constraint.element.unwrap_or(InputSlotKind::Any);
                for value in items.iter() {
                    if kind != InputSlotKind::Any && kind != input_value_kind(*value) {
                        return Err(InputValueIssue::Element);
                    }
                    push(
                        &mut pending,
                        Check {
                            kind,
                            constraint: &any,
                            value: *value,
                            depth: next_depth(check.depth)?,
                        },
                    )?;
                }
            }
            SlotValue::Object(id) => {
                let cp = checkpoint.ok_or(InputValueIssue::Payload)?;
                let index = usize::try_from(id.get()).map_err(|_| InputValueIssue::Payload)?;
                let fields = cp.objects.get(index).ok_or(InputValueIssue::Payload)?;
                let declared = check.constraint.fields.as_deref().unwrap_or_default();
                for field in declared {
                    let present = fields
                        .iter()
                        .any(|item| name(cp, item.key) == Ok(field.name.as_ref()));
                    if !present
                        && (!field.constraint.optional
                            || field.constraint.default_value.is_some()
                            || field.constraint.default_cold.is_some())
                    {
                        return Err(InputValueIssue::RequiredField);
                    }
                }
                for field in fields.iter() {
                    let key = name(cp, field.key)?;
                    let schema = declared.iter().find(|schema| schema.name.as_ref() == key);
                    if schema.is_none() && check.constraint.allow_extra == Some(false) {
                        return Err(InputValueIssue::ExtraField);
                    }
                    let (kind, constraint) = schema.map_or((InputSlotKind::Any, &any), |schema| {
                        (schema.kind, &schema.constraint)
                    });
                    push(
                        &mut pending,
                        Check {
                            kind,
                            constraint,
                            value: field.value,
                            depth: next_depth(check.depth)?,
                        },
                    )?;
                }
            }
            SlotValue::Blob(id) => {
                let cp = checkpoint.ok_or(InputValueIssue::Payload)?;
                let index = usize::try_from(id.get()).map_err(|_| InputValueIssue::Payload)?;
                cp.blobs.get(index).ok_or(InputValueIssue::Payload)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Checks scalar transport values against the exact immutable source payloads.
pub fn validate_scalar_input_value(
    declaration: &CompiledInputSlot,
    value: SlotValue,
    workflow: &crate::CompiledWorkflow,
) -> Result<(), InputValueIssue> {
    if workflow.source_metadata().is_some_and(|metadata| {
        metadata.secret_requirements().iter().any(|binding| {
            binding.slot() == Some(declaration.slot)
                && value != SlotValue::Symbol(binding.requirement())
        })
    }) {
        return Err(InputValueIssue::Payload);
    }
    if matches!(value, SlotValue::Symbol(_)) {
        let metadata = workflow.source_metadata().ok_or(InputValueIssue::Payload)?;
        let mut store = crate::ValueStore::new();
        crate::workflow::source_metadata::seed_source_symbols(metadata, &mut store)
            .map_err(|_| InputValueIssue::Payload)?;
        validate_input_value(
            declaration,
            value,
            workflow.symbols_count(),
            Some(&store.checkpoint_v1()),
        )
    } else {
        validate_input_value(declaration, value, workflow.symbols_count(), None)
    }
}

fn name(cp: &ValueStoreCheckpointV1, key: crate::SymbolId) -> Result<&str, InputValueIssue> {
    let index = usize::try_from(key.get()).map_err(|_| InputValueIssue::Payload)?;
    cp.symbols
        .get(index)
        .map(Box::as_ref)
        .ok_or(InputValueIssue::Payload)
}

fn next_depth(depth: usize) -> Result<usize, InputValueIssue> {
    depth.checked_add(1).ok_or(InputValueIssue::DepthExceeded)
}

fn push<'a>(pending: &mut Vec<Check<'a>>, check: Check<'a>) -> Result<(), InputValueIssue> {
    if pending.len() >= MAX_VALUES_PER_RUN {
        return Err(InputValueIssue::Capacity);
    }
    pending
        .try_reserve(1)
        .map_err(|_| InputValueIssue::Capacity)?;
    pending.push(check);
    Ok(())
}

fn check_length(length: usize, min: Option<i64>, max: Option<i64>) -> Result<(), InputValueIssue> {
    let length = i64::try_from(length).map_err(|_| InputValueIssue::Bounds)?;
    check_bounds(length, min, max)
}

fn check_bounds(value: i64, min: Option<i64>, max: Option<i64>) -> Result<(), InputValueIssue> {
    if min.is_some_and(|low| value < low) || max.is_some_and(|high| value > high) {
        Err(InputValueIssue::Bounds)
    } else {
        Ok(())
    }
}
