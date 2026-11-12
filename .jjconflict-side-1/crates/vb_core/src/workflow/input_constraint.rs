#![forbid(unsafe_code)]
//! Retained per-slot input schema constraints as bounded numerics.

use super::{Bound, ExtraFields, InputSlotKind, Nullability, Presence, Secretness};
use crate::value::SlotValue;
use serde::{Deserialize, Serialize};

/// Retained per-slot input schema constraints as bounded numerics.
///
/// Every field is `Copy` with no heap allocation so input-slot tables stay
/// numeric end to end. `unconstrained()` reproduces exact legacy behavior
/// (kind-only check, no presence requirement) for hand-built or
/// pre-constraint artifacts; only compiler-emitted slots carry strict
/// constraints. Nested object `fields` sub-schemas and non-scalar defaults
/// are not representable here; their full-fidelity identity is carried by
/// `WorkflowParts::semantic_source_identity` instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl CompiledInputConstraint {
    /// Plain required slot with no further constraints.
    ///
    /// This is what the compiler emits for bare shorthand schemas such as
    /// `count: number`: the value must be supplied, with kind-only checking.
    #[must_use]
    pub const fn required() -> Self {
        Self {
            presence: Presence::Required,
            ..Self::unconstrained()
        }
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
        }
    }

    /// Whether submission must supply a value for this slot.
    ///
    /// A declared scalar default satisfies a missing input, so only slots
    /// that are neither optional nor defaulted require a value.
    #[must_use]
    pub const fn requires_value(&self) -> bool {
        self.presence.is_required() && self.default_value.is_none()
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
}
