//! Bounded validation of cold schema trees before cloning or analysis.
use super::{CompiledInputConstraint, InputConstraintIssue as Issue, InputSlotKind, WorkflowParts};
use crate::input_schema::{MAX_INPUT_SCHEMA_DEPTH, apply_input_schema_defaults, default_value};
use crate::limits::{MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE, MAX_VALUES_PER_RUN};
use crate::{InputDefaultValue, SlotIdx, Taint, ValueStore};

pub(super) fn validate(
    parts: &WorkflowParts,
    slot: SlotIdx,
    kind: InputSlotKind,
    root: &CompiledInputConstraint,
) -> Result<(), Issue> {
    let mut pending = Vec::new();
    pending.try_reserve(1).map_err(|_| Issue::SchemaCapacity)?;
    pending.push((kind, root, 0usize));
    let mut visited = 0usize;
    while let Some((kind, schema, depth)) = pending.pop() {
        visited = visited.checked_add(1).ok_or(Issue::SchemaCapacity)?;
        if depth > MAX_INPUT_SCHEMA_DEPTH || visited > MAX_VALUES_PER_RUN {
            return Err(Issue::SchemaCapacity);
        }
        super::validate_input_constraint(slot, schema).map_err(|error| match error {
            super::WorkflowError::InconsistentInputConstraint { issue, .. } => issue,
            _ => Issue::SchemaCapacity,
        })?;
        validate_kind(kind, schema)?;
        if schema.default_cold.is_some() && schema.default_value.is_some() {
            return Err(Issue::ConflictingDefaults);
        }
        if let Some(fields) = schema.fields.as_deref() {
            if kind != InputSlotKind::Object {
                return Err(Issue::InvalidFieldSchema);
            }
            let next = depth.checked_add(1).ok_or(Issue::SchemaCapacity)?;
            if fields.len() > MAX_OBJECT_FIELDS_PER_VALUE {
                return Err(Issue::SchemaCapacity);
            }
            unique_names(
                fields.iter().map(|field| field.name.as_ref()),
                fields.len(),
                Issue::InvalidFieldSchema,
            )?;
            if pending
                .len()
                .checked_add(fields.len())
                .is_none_or(|count| count > MAX_VALUES_PER_RUN.saturating_sub(visited))
            {
                return Err(Issue::SchemaCapacity);
            }
            pending
                .try_reserve(fields.len())
                .map_err(|_| Issue::SchemaCapacity)?;
            for field in fields {
                if field.name.is_empty() {
                    return Err(Issue::InvalidFieldSchema);
                }
                pending.push((field.kind, &field.constraint, next));
            }
        }
        if let Some(value) = &schema.default_cold {
            validate_tree(value)?;
        }
    }
    // All schema trees are now bounded; cloning a declaration cannot recurse
    // through an unvalidated artifact-controlled depth.
    validate_defaults(parts, slot, kind, root)
}

fn validate_kind(kind: InputSlotKind, schema: &CompiledInputConstraint) -> Result<(), Issue> {
    if ((schema.min.is_some() || schema.max.is_some())
        && !matches!(kind, InputSlotKind::I64 | InputSlotKind::List))
        || ((schema.min_length.is_some() || schema.max_length.is_some())
            && kind != InputSlotKind::Symbol)
        || (schema.element.is_some() && kind != InputSlotKind::List)
        || (schema.allow_extra.is_some() && kind != InputSlotKind::Object)
    {
        return Err(Issue::KeywordKindMismatch);
    }
    if kind == InputSlotKind::List
        && [schema.min, schema.max]
            .into_iter()
            .flatten()
            .any(|bound| bound < 0)
    {
        return Err(Issue::NegativeLengthBound);
    }
    Ok(())
}

fn validate_tree(root: &InputDefaultValue) -> Result<(), Issue> {
    let mut pending = Vec::new();
    pending.try_reserve(1).map_err(|_| Issue::SchemaCapacity)?;
    pending.push((root, 0usize));
    let mut visited = 0usize;
    while let Some((value, depth)) = pending.pop() {
        visited = visited.checked_add(1).ok_or(Issue::SchemaCapacity)?;
        if depth > MAX_INPUT_SCHEMA_DEPTH || visited > MAX_VALUES_PER_RUN {
            return Err(Issue::SchemaCapacity);
        }
        let next = depth.checked_add(1).ok_or(Issue::SchemaCapacity)?;
        match value {
            InputDefaultValue::List(items) => {
                if items.len() > MAX_LIST_ITEMS_PER_VALUE {
                    return Err(Issue::SchemaCapacity);
                }
                if pending
                    .len()
                    .checked_add(items.len())
                    .is_none_or(|count| count > MAX_VALUES_PER_RUN.saturating_sub(visited))
                {
                    return Err(Issue::SchemaCapacity);
                }
                pending
                    .try_reserve(items.len())
                    .map_err(|_| Issue::SchemaCapacity)?;
                pending.extend(items.iter().map(|value| (value, next)));
            }
            InputDefaultValue::Object(fields) => {
                if fields.len() > MAX_OBJECT_FIELDS_PER_VALUE {
                    return Err(Issue::SchemaCapacity);
                }
                unique_names(
                    fields.iter().map(|(key, _)| key.as_ref()),
                    fields.len(),
                    Issue::InvalidDefault,
                )?;
                if pending
                    .len()
                    .checked_add(fields.len())
                    .is_none_or(|count| count > MAX_VALUES_PER_RUN.saturating_sub(visited))
                {
                    return Err(Issue::SchemaCapacity);
                }
                pending
                    .try_reserve(fields.len())
                    .map_err(|_| Issue::SchemaCapacity)?;
                for (_, value) in fields {
                    pending.push((value, next));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn unique_names<'a>(
    names: impl Iterator<Item = &'a str>,
    len: usize,
    duplicate: Issue,
) -> Result<(), Issue> {
    let mut sorted = Vec::new();
    sorted
        .try_reserve_exact(len)
        .map_err(|_| Issue::SchemaCapacity)?;
    sorted.extend(names);
    sorted.sort_unstable();
    if sorted.windows(2).any(|pair| pair.first() == pair.last()) {
        return Err(duplicate);
    }
    Ok(())
}
fn validate_defaults(
    parts: &WorkflowParts,
    slot: SlotIdx,
    kind: InputSlotKind,
    root: &CompiledInputConstraint,
) -> Result<(), Issue> {
    let mut pending = vec![(kind, root)];
    while let Some((kind, schema)) = pending.pop() {
        if let Some(fields) = schema.fields.as_deref() {
            pending
                .try_reserve(fields.len())
                .map_err(|_| Issue::SchemaCapacity)?;
            pending.extend(fields.iter().map(|field| (field.kind, &field.constraint)));
        }
        if schema.default_value.is_none() && schema.default_cold.is_none() {
            continue;
        }
        let mut store = ValueStore::new();
        if let Some(metadata) = &parts.source_metadata {
            super::source_metadata::seed_source_symbols(metadata, &mut store)
                .map_err(|_| Issue::InvalidDefault)?;
        }
        let mut checkpoint = store.checkpoint_v1();
        let value = default_value(schema, &mut checkpoint, Taint::Clean)
            .map_err(|_| Issue::InvalidDefault)?
            .ok_or(Issue::InvalidDefault)?;
        apply_input_schema_defaults(kind, schema, value, Taint::Clean, &mut checkpoint)
            .map_err(|_| Issue::InvalidDefault)?;
        let declaration = super::CompiledInputSlot {
            slot,
            kind,
            constraint: schema.clone(),
        };
        crate::input_validation::validate_input_value(
            &declaration,
            value,
            parts.symbols_count,
            Some(&checkpoint),
        )
        .map_err(|_| Issue::InvalidDefault)?;
    }
    Ok(())
}
