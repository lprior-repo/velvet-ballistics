#![forbid(unsafe_code)]
//! Compiled input slot declarations: branch targets and slot kinds.

use super::CompiledInputConstraint;
use crate::ids::{ExprIdx, SlotIdx, StepIdx};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Expression branch target used by final choose IR scaffolding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExprBranch {
    /// Expression condition index.
    pub condition: ExprIdx,
    /// Target node when the condition is true.
    pub target: StepIdx,
}

/// Materialized boolean-slot branch target used by final choose IR scaffolding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotBranch {
    /// Boolean condition slot.
    pub condition: SlotIdx,
    /// Target node when the condition is true.
    pub target: StepIdx,
}

/// Classification of a compiled input slot by its runtime value shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum InputSlotKind {
    /// Slot holds a null value.
    Null,
    /// Slot holds a boolean value.
    Bool,
    /// Slot holds a 64-bit signed integer value.
    I64,
    /// Slot holds a 64-bit float value.
    F64,
    /// Slot holds an interned symbol.
    Symbol,
    /// Slot holds a list handle.
    List,
    /// Slot holds an object handle.
    Object,
    /// Slot holds a blob handle.
    Blob,
    /// Slot accepts any recognized value shape (schema wildcard).
    Any,
}

/// A compiled input slot: numeric index plus production classification.
///
/// Source names are intentionally excluded; the compiler emits only numeric
/// references so the runtime never pays for string lookups on hot paths.
/// The `kind` field describes the runtime value shape held at that slot index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledInputSlot {
    /// Numeric slot index.
    pub slot: SlotIdx,
    /// Runtime value shape classification for this slot.
    pub kind: InputSlotKind,
    /// Retained author-declared schema constraints for this slot.
    ///
    /// Serde `default` tolerates pre-constraint artifacts the same way
    /// `WorkflowParts::input_slots` tolerates pre-v1 artifacts.
    #[serde(default)]
    pub constraint: CompiledInputConstraint,
}

/// Presence requirement for a compiled input slot.
///
/// Replaces the legacy `optional: bool` flag so the decision is named at
/// every use. The encoding is frozen: it serializes exactly as the legacy
/// boolean (`Required` as `false`, `Optional` as `true`) so canonical
/// digests and artifact bytes are unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Presence {
    /// The slot must be supplied (unless a default is declared).
    Required,
    /// The slot may be omitted at submission.
    Optional,
}

impl Presence {
    /// Whether the slot may be omitted.
    #[must_use]
    pub const fn is_optional(self) -> bool {
        matches!(self, Self::Optional)
    }

    /// Whether the slot must be supplied.
    #[must_use]
    pub const fn is_required(self) -> bool {
        matches!(self, Self::Required)
    }

    /// Legacy boolean encoding (`optional`).
    #[must_use]
    pub const fn as_bool(self) -> bool {
        self.is_optional()
    }

    /// Decode the legacy `optional` boolean.
    #[must_use]
    pub const fn from_bool(optional: bool) -> Self {
        if optional {
            Self::Optional
        } else {
            Self::Required
        }
    }
}

impl From<bool> for Presence {
    /// Converts the legacy `optional` boolean flag.
    fn from(optional: bool) -> Self {
        Self::from_bool(optional)
    }
}

impl Serialize for Presence {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(self.as_bool())
    }
}

impl<'de> Deserialize<'de> for Presence {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        bool::deserialize(deserializer).map(Self::from_bool)
    }
}

#[cfg(kani)]
impl kani::Arbitrary for Presence {
    fn any() -> Self {
        if kani::any() {
            Self::Optional
        } else {
            Self::Required
        }
    }
}

/// Nullability classification for a compiled input slot.
///
/// Replaces the legacy `nullable: bool` flag. The encoding is frozen: it
/// serializes exactly as the legacy boolean (`NonNullable` as `false`,
/// `Nullable` as `true`) so canonical digests and artifact bytes are
/// unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nullability {
    /// Null values are rejected for non-null kinds.
    NonNullable,
    /// An explicit null value is accepted for non-null kinds.
    Nullable,
}

impl Nullability {
    /// Whether explicit null is accepted.
    #[must_use]
    pub const fn allows_null(self) -> bool {
        matches!(self, Self::Nullable)
    }

    /// Legacy boolean encoding (`nullable`).
    #[must_use]
    pub const fn as_bool(self) -> bool {
        self.allows_null()
    }

    /// Decode the legacy `nullable` boolean.
    #[must_use]
    pub const fn from_bool(nullable: bool) -> Self {
        if nullable {
            Self::Nullable
        } else {
            Self::NonNullable
        }
    }
}

impl From<bool> for Nullability {
    /// Converts the legacy `nullable` boolean flag.
    fn from(nullable: bool) -> Self {
        Self::from_bool(nullable)
    }
}

impl Serialize for Nullability {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(self.as_bool())
    }
}

impl<'de> Deserialize<'de> for Nullability {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        bool::deserialize(deserializer).map(Self::from_bool)
    }
}

#[cfg(kani)]
impl kani::Arbitrary for Nullability {
    fn any() -> Self {
        if kani::any() {
            Self::Nullable
        } else {
            Self::NonNullable
        }
    }
}
