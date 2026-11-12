#![cfg(kani)]
//! Schema generators cover two nested levels and up to two children per node.
use super::*;
use crate::input_schema::{CompiledInputField, InputDefaultValue};
use crate::workflow::source_metadata::{CompiledSecretRequirement, WorkflowSourceMetadata};

pub(super) fn kind() -> InputSlotKind {
    match kani::any::<u8>() {
        0 => InputSlotKind::Null,
        1 => InputSlotKind::Bool,
        2 => InputSlotKind::I64,
        3 => InputSlotKind::F64,
        4 => InputSlotKind::Symbol,
        5 => InputSlotKind::List,
        6 => InputSlotKind::Object,
        7 => InputSlotKind::Blob,
        _ => InputSlotKind::Any,
    }
}

fn width() -> u8 {
    let count: u8 = kani::any();
    kani::assume(count <= 2);
    count
}

fn value(depth: u8) -> InputDefaultValue {
    let choice: u8 = kani::any();
    match choice {
        0 => InputDefaultValue::Null,
        1 => InputDefaultValue::Bool(kani::any()),
        2 => InputDefaultValue::I64(kani::any()),
        3 => InputDefaultValue::Text(kani_step_name(kani::any())),
        4 if depth > 0 => InputDefaultValue::List(
            (0..width())
                .map(|_| value(depth.saturating_sub(1)))
                .collect(),
        ),
        5 if depth > 0 => InputDefaultValue::Object(
            (0..width())
                .map(|_| (kani_step_name(kani::any()), value(depth.saturating_sub(1))))
                .collect(),
        ),
        _ => InputDefaultValue::Null,
    }
}

pub(super) fn constraint(depth: u8) -> CompiledInputConstraint {
    let (min, max) = ordered_optional_bounds();
    let (min_length, max_length) = ordered_optional_length_bounds();
    CompiledInputConstraint {
        presence: kani::any(),
        nullability: kani::any(),
        secretness: kani::any(),
        default_value: kani::any(),
        min,
        max,
        min_length,
        max_length,
        element: kani::any::<bool>().then(kind),
        extra: kani::any(),
        default_cold: kani::any::<bool>().then(|| value(2)),
        fields: if depth > 0 && kani::any() {
            Some(
                (0..width())
                    .map(|_| CompiledInputField {
                        name: kani_step_name(kani::any()),
                        kind: kind(),
                        constraint: constraint(depth.saturating_sub(1)),
                    })
                    .collect(),
            )
        } else {
            None
        },
    }
}

pub(super) fn metadata() -> Option<Box<WorkflowSourceMetadata>> {
    if !kani::any::<bool>() {
        return None;
    }
    let count = width();
    let symbols = (0..count).map(kani_step_name).collect();
    let requirements: Box<[_]> = if kani::any::<bool>() && count > 0 {
        let alias: u8 = kani::any();
        let requirement: u8 = kani::any();
        kani::assume(alias < count && requirement < count);
        Box::new([CompiledSecretRequirement::new(
            SymbolId::new(u32::from(alias)),
            SymbolId::new(u32::from(requirement)),
        )])
    } else {
        Box::new([])
    };
    match WorkflowSourceMetadata::try_new(symbols, requirements) {
        Ok(metadata) => Some(Box::new(metadata)),
        Err(_) => {
            kani::assume(false);
            None
        }
    }
}
