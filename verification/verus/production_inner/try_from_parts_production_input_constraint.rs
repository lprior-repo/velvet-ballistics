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

// Production `crates/vb_core/src/value.rs:129-149`
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

// Production `crates/vb_core/src/value.rs:151-151`
impl Eq for SlotValue {}

// Production `crates/vb_core/src/workflow/mod.rs:868-902`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledInputConstraint {
    pub presence: Presence,
    pub nullability: Nullability,
    pub default_value: Option<SlotValue>,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub min_length: Option<i64>,
    pub max_length: Option<i64>,
    pub element: Option<InputSlotKind>,
    pub extra: ExtraFields,
    pub secretness: Secretness,
}

// Production `crates/vb_core/src/workflow/mod.rs:904-954`
impl CompiledInputConstraint {
    #[must_use]
    pub const fn required() -> Self {
        Self {
            presence: Presence::Required,
            ..Self::unconstrained()
        }
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
        }
    }
    #[must_use]
    pub const fn requires_value(&self) -> bool {
        self.presence.is_required() && self.default_value.is_none()
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

// Production `crates/vb_core/src/workflow/mod.rs:1807-1827`
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
