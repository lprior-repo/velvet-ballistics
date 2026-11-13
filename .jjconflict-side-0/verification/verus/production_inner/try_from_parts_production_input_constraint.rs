#![forbid(unsafe_code)]

use super::input_types::{Bound, ExtraFields, Nullability, Presence, Secretness};
use super::{FiniteF64, InputConstraintIssue, InputSlotKind, SlotIdx, SymbolId, WorkflowError};

// Numeric handle projections expand the existing production numeric_id! macro with each identifier's declared scalar payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ListId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ObjectId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct BlobId(pub u64);

// Production `crates/vb_core/src/value/slot.rs:14-31` (named `SlotValue`; kind `enum_item`; owner `-`; trait `-`).
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SlotValue {
    Null,
    Bool(bool),
    I64(i64),
    F64(FiniteF64),
    Symbol(SymbolId),
    List(ListId),
    Object(ObjectId),
    Blob(BlobId),
}

// Production `crates/vb_core/src/value/slot.rs:33-33` (named `SlotValue`; kind `impl_item`; owner `-`; trait `Eq`).
impl Eq for SlotValue {}

// Production `crates/vb_core/src/input_schema.rs:12-19` (owned field projection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledInputField {
    /// Source field name, resolved against owned object key payloads.
    pub name: Box<str>,
    /// Field value kind, independent of any list element kind.
    pub kind: InputSlotKind,
    /// Retained flags, bounds, defaults, and nested fields.
    pub constraint: CompiledInputConstraint,
}

// Production `crates/vb_core/src/input_schema.rs:24-37` (owned default projection).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InputDefaultValue {
    /// Explicit null.
    Null,
    /// Boolean literal.
    Bool(bool),
    /// Signed integer literal.
    I64(i64),
    /// UTF-8 text.
    Text(Box<str>),
    /// Ordered list items.
    List(Box<[InputDefaultValue]>),
    /// Object fields in source order.
    Object(Vec<(Box<str>, InputDefaultValue)>),
}

// Production `crates/vb_core/src/workflow/input_constraint.rs:13-43` (named `CompiledInputConstraint`; kind `struct_item`; owner `-`; trait `-`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledInputConstraint {
    /// Presence requirement for this slot.
    pub presence: Presence,
    /// Nullability classification for this slot.
    pub nullability: Nullability,
    /// Scalar default filled when the slot is omitted (`None` when no
    /// materializable default was declared).
    pub default_value: Option<SlotValue>,
    /// Inclusive minimum for integer values (length bound for lists).
    pub min: Option<i64>,
    /// Inclusive maximum for integer values (length bound for lists).
    pub max: Option<i64>,
    /// Inclusive minimum text length (retained; enforced where payloads
    /// are visible).
    pub min_length: Option<i64>,
    /// Inclusive maximum text length (retained; enforced where payloads
    /// are visible).
    pub max_length: Option<i64>,
    /// Declared list element kind (`of`).
    pub element: Option<InputSlotKind>,
    /// Declared object `extra` policy.
    pub extra: ExtraFields,
    /// Secrecy classification for this slot.
    pub secretness: Secretness,
    /// Nested object field schemas (`fields`). `None` for non-object kinds.
    pub fields: Option<Box<[CompiledInputField]>>,
    /// Non-scalar default representation for text/list/object kinds.
    pub default_cold: Option<InputDefaultValue>,
}

// Production `crates/vb_core/src/workflow/input_constraint.rs:45-96` (named `CompiledInputConstraint`; kind `impl_item`; owner `-`; trait `-`).
impl CompiledInputConstraint {
    #[must_use]
    pub const fn required() -> Self {
        let mut constraint = Self::unconstrained();
        constraint.presence = Presence::Required;
        constraint
    }
    #[must_use]
    pub const fn unconstrained() -> Self {
        Self {
            presence: Presence::Optional,
            nullability: Nullability::NonNullable,
            default_value: None,
            min: None,
            max: None,
            min_length: None,
            max_length: None,
            element: None,
            extra: ExtraFields::Unspecified,
            secretness: Secretness::NonSecret,
            fields: None,
            default_cold: None,
        }
    }
    #[must_use]
    pub const fn requires_value(&self) -> bool {
        self.presence.is_required() && self.default_value.is_none() && self.default_cold.is_none()
    }
    #[must_use]
    pub const fn value_bound(&self) -> Bound {
        Bound::new(self.min, self.max)
    }
    #[must_use]
    pub const fn length_bound(&self) -> Bound {
        Bound::new(self.min_length, self.max_length)
    }
}

// Production `crates/vb_core/src/workflow/validate.rs:80-106` (named `validate_input_constraint`; kind `function_item`; owner `-`; trait `-`).
pub(super) fn validate_input_constraint(
    slot: SlotIdx,
    constraint: &CompiledInputConstraint,
) -> Result<(), WorkflowError> {
    let inconsistent =
        |issue: InputConstraintIssue| WorkflowError::InconsistentInputConstraint { slot, issue };
    if constraint.value_bound().inconsistency() {
        return Err(inconsistent(InputConstraintIssue::MinExceedsMax));
    }
    if constraint.length_bound().inconsistency() {
        return Err(inconsistent(
            InputConstraintIssue::MinLengthExceedsMaxLength,
        ));
    }
    if negative_length_bound(constraint) {
        return Err(inconsistent(InputConstraintIssue::NegativeLengthBound));
    }
    Ok(())
}

// Production negative-length predicate; both immutable accessor calls retain short-circuit order.
fn negative_length_bound(constraint: &CompiledInputConstraint) -> bool {
    constraint
        .length_bound()
        .lower()
        .is_some_and(|bound| bound < 0)
        || constraint
            .length_bound()
            .upper()
            .is_some_and(|bound| bound < 0)
}

// Production input-slot ordering decision, extracted without changing error precedence.
pub(super) fn validate_input_slot_order(
    input_slot: &super::CompiledInputSlot,
    prev_slot: Option<SlotIdx>,
) -> Result<(), WorkflowError> {
    let current = input_slot.slot;
    let current_value = current.get();
    if let Some(prev) = prev_slot {
        let prev_value = prev.get();
        if current_value == prev_value {
            return Err(WorkflowError::DuplicateInputSlot {
                slot: current,
                kind: input_slot.kind,
            });
        }
        if current_value < prev_value {
            return Err(WorkflowError::NonCanonicalInputSlotOrder {
                prev_slot: prev,
                curr_slot: current,
            });
        }
    }
    Ok(())
}
