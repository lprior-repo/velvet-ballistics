#![forbid(unsafe_code)]
//! Input slot constraint primitives: bounds and field policies.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Inclusive bounded range for integer / length constraints.
///
/// Replaces the scattered `min` / `max` / `min_length` / `max_length`
/// option soup with a single total type whose invariants (ordering,
/// non-negativity) are checked in one place. `Bound::UNBOUNDED` is the
/// legacy `(None, None)` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Bound {
    /// Inclusive lower bound.
    pub min: Option<i64>,
    /// Inclusive upper bound.
    pub max: Option<i64>,
}

impl Bound {
    /// Fully unbounded range.
    pub const UNBOUNDED: Self = Self { min: None, max: None };

    /// Constructs a bounded range.
    #[must_use]
    pub const fn new(min: Option<i64>, max: Option<i64>) -> Self {
        Self { min, max }
    }

    /// Lower bound, when declared.
    #[must_use]
    pub const fn lower(self) -> Option<i64> {
        self.min
    }

    /// Upper bound, when declared.
    #[must_use]
    pub const fn upper(self) -> Option<i64> {
        self.max
    }

    /// Whether no bound is declared on either end.
    #[must_use]
    pub const fn is_unbounded(self) -> bool {
        self.min.is_none() && self.max.is_none()
    }

    /// Whether `value` satisfies both declared ends.
    #[must_use]
    pub const fn contains(self, value: i64) -> bool {
        match self.min {
            Some(min) if value < min => return false,
            _ => {}
        }
        match self.max {
            Some(max) if value > max => return false,
            _ => {}
        }
        true
    }

    /// Structural inconsistency, when this range cannot hold any value.
    #[must_use]
    pub const fn inconsistency(self) -> bool {
        match (self.min, self.max) {
            (Some(min), Some(max)) => min > max,
            _ => false,
        }
    }
}

/// Object `extra` policy for a compiled input slot.
///
/// Replaces the legacy `allow_extra: Option<bool>` triple-state. The
/// encoding is frozen: it serializes exactly as the legacy `Option<bool>`
/// (`Unspecified` as `None`, `Allow` as `Some(true)`, `Reject` as
/// `Some(false)`) so canonical digests and artifact bytes are unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtraFields {
    /// No `extra` policy was declared.
    Unspecified,
    /// Additional object fields are allowed.
    Allow,
    /// Additional object fields are rejected.
    Reject,
}

impl ExtraFields {
    /// Whether extra fields are explicitly allowed.
    #[must_use]
    pub const fn allows_extra(self) -> bool {
        matches!(self, Self::Allow)
    }

    /// Whether extra fields are explicitly rejected.
    #[must_use]
    pub const fn rejects_extra(self) -> bool {
        matches!(self, Self::Reject)
    }

    /// Legacy `Option<bool>` encoding.
    #[must_use]
    pub const fn as_option(self) -> Option<bool> {
        match self {
            Self::Unspecified => None,
            Self::Allow => Some(true),
            Self::Reject => Some(false),
        }
    }

    /// Decode the legacy `Option<bool>` encoding.
    #[must_use]
    pub const fn from_option(value: Option<bool>) -> Self {
        match value {
            None => Self::Unspecified,
            Some(true) => Self::Allow,
            Some(false) => Self::Reject,
        }
    }
}

impl From<Option<bool>> for ExtraFields {
    /// Converts the legacy `allow_extra: Option<bool>` encoding.
    fn from(value: Option<bool>) -> Self {
        Self::from_option(value)
    }
}

impl Serialize for ExtraFields {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_option().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ExtraFields {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Option::<bool>::deserialize(deserializer).map(Self::from_option)
    }
}

#[cfg(kani)]
impl kani::Arbitrary for ExtraFields {
    fn any() -> Self {
        match kani::any::<u8>() % 3 {
            0 => Self::Unspecified,
            1 => Self::Allow,
            _ => Self::Reject,
        }
    }
}

/// Secrecy classification for a compiled input slot.
///
/// Replaces the legacy `secret: bool` flag with a named decision. The
/// encoding is frozen: it serializes exactly as the legacy boolean
/// (`NonSecret` as `false`, `Secret` as `true`) so canonical digests and
/// artifact bytes are unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Secretness {
    /// No secrecy marker is declared.
    NonSecret,
    /// Author-declared secrecy marker (matches compile-time taint facts).
    Secret,
}

impl Secretness {
    /// Whether the slot is marked secret.
    #[must_use]
    pub const fn is_secret(self) -> bool {
        matches!(self, Self::Secret)
    }

    /// Legacy boolean encoding (`secret`).
    #[must_use]
    pub const fn as_bool(self) -> bool {
        self.is_secret()
    }

    /// Decode the legacy `secret` boolean.
    #[must_use]
    pub const fn from_bool(secret: bool) -> Self {
        if secret {
            Self::Secret
        } else {
            Self::NonSecret
        }
    }
}

impl From<bool> for Secretness {
    /// Converts the legacy `secret` boolean flag.
    fn from(secret: bool) -> Self {
        Self::from_bool(secret)
    }
}

impl Serialize for Secretness {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(self.as_bool())
    }
}

impl<'de> Deserialize<'de> for Secretness {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        bool::deserialize(deserializer).map(Self::from_bool)
    }
}

#[cfg(kani)]
impl kani::Arbitrary for Secretness {
    fn any() -> Self {
        if kani::any() { Self::Secret } else { Self::NonSecret }
    }
}
