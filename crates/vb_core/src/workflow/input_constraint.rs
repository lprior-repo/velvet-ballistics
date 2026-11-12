#![forbid(unsafe_code)]
//! Retained per-slot numeric constraints and owned cold schema payloads.

use super::{Bound, ExtraFields, InputSlotKind, Nullability, Presence, Secretness};
use crate::value::SlotValue;
use serde::{Deserialize, Serialize};

/// Retained per-slot input schema constraints.
///
/// Scalar fields remain numeric. Nested fields and composite defaults retain
/// owned cold payloads; `unconstrained()` preserves legacy kind-only behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    #[serde(default)]
    pub fields: Option<Box<[super::CompiledInputField]>>,
    /// Non-scalar default representation for text/list/object kinds.
    #[serde(default)]
    pub default_cold: Option<super::InputDefaultValue>,
}

impl CompiledInputConstraint {
    /// Plain required slot with no further constraints.
    ///
    /// This is what the compiler emits for bare shorthand schemas such as
    /// `count: number`: the value must be supplied, with kind-only checking.
    #[must_use]
    pub const fn required() -> Self {
        let mut constraint = Self::unconstrained();
        constraint.presence = Presence::Required;
        constraint
    }

    /// Legacy-compatible constraint: kind-only check, nothing required.
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

    /// Whether submission must supply a value for this slot.
    ///
    /// A declared scalar or composite default satisfies a missing input, so only slots
    /// that are neither optional nor defaulted require a value.
    #[must_use]
    pub const fn requires_value(&self) -> bool {
        self.presence.is_required() && self.default_value.is_none() && self.default_cold.is_none()
    }

    /// Value bound over `min` / `max`.
    #[must_use]
    pub const fn value_bound(&self) -> Bound {
        Bound::new(self.min, self.max)
    }

    /// Length bound over `min_length` / `max_length`.
    #[must_use]
    pub const fn length_bound(&self) -> Bound {
        Bound::new(self.min_length, self.max_length)
    }
}

impl Default for CompiledInputConstraint {
    /// Legacy-compatible default: same as `unconstrained()`.
    ///
    /// This keeps `#[serde(default)]` on pre-constraint artifacts
    /// permissive instead of flipping them to required.
    fn default() -> Self {
        Self::unconstrained()
    }
}

/// One structural inconsistency in a retained input constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum InputConstraintIssue {
    /// `min` exceeds `max`.
    MinExceedsMax,
    /// `min_length` exceeds `max_length`.
    MinLengthExceedsMaxLength,
    /// A length bound is negative.
    NegativeLengthBound,
    /// Cold schema or default traversal exceeded its bounded capacity.
    SchemaCapacity,
    /// Object field declarations are duplicate, empty, or attached to another kind.
    InvalidFieldSchema,
    /// Scalar and owned defaults cannot both be declared.
    ConflictingDefaults,
    /// A retained default fails its declared schema.
    InvalidDefault,
    /// A retained schema keyword is incompatible with its declared kind.
    KeywordKindMismatch,
}
