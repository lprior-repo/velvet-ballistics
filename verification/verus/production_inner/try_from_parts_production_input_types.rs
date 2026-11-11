#![forbid(unsafe_code)]

// Input policy declarations and numeric bound accessors; serde/Kani impls are outside this projection.

// Production `crates/vb_core/src/workflow/mod.rs:524-536`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Presence {
    Required,
    Optional,
}

// Production `crates/vb_core/src/workflow/mod.rs:538-566`
impl Presence {
    #[must_use]
    pub const fn is_optional(self) -> bool {
        matches!(self, Self::Optional)
    }
    #[must_use]
    pub const fn is_required(self) -> bool {
        matches!(self, Self::Required)
    }
    #[must_use]
    pub const fn as_bool(self) -> bool {
        self.is_optional()
    }
    #[must_use]
    pub const fn from_bool(optional: bool) -> Self {
        if optional {
            Self::Optional
        } else {
            Self::Required
        }
    }
}

// Production `crates/vb_core/src/workflow/mod.rs:594-606`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nullability {
    NonNullable,
    Nullable,
}

// Production `crates/vb_core/src/workflow/mod.rs:608-630`
impl Nullability {
    #[must_use]
    pub const fn allows_null(self) -> bool {
        matches!(self, Self::Nullable)
    }
    #[must_use]
    pub const fn as_bool(self) -> bool {
        self.allows_null()
    }
    #[must_use]
    pub const fn from_bool(nullable: bool) -> Self {
        if nullable {
            Self::Nullable
        } else {
            Self::NonNullable
        }
    }
}

// Production `crates/vb_core/src/workflow/mod.rs:724-738`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtraFields {
    Unspecified,
    Allow,
    Reject,
}

// Production `crates/vb_core/src/workflow/mod.rs:740-772`
impl ExtraFields {
    #[must_use]
    pub const fn allows_extra(self) -> bool {
        matches!(self, Self::Allow)
    }
    #[must_use]
    pub const fn rejects_extra(self) -> bool {
        matches!(self, Self::Reject)
    }
    #[must_use]
    pub const fn as_option(self) -> Option<bool> {
        match self {
            Self::Unspecified => None,
            Self::Allow => Some(true),
            Self::Reject => Some(false),
        }
    }
    #[must_use]
    pub const fn from_option(value: Option<bool>) -> Self {
        match value {
            None => Self::Unspecified,
            Some(true) => Self::Allow,
            Some(false) => Self::Reject,
        }
    }
}

// Production `crates/vb_core/src/workflow/mod.rs:804-816`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Secretness {
    NonSecret,
    Secret,
}

// Production `crates/vb_core/src/workflow/mod.rs:818-840`
impl Secretness {
    #[must_use]
    pub const fn is_secret(self) -> bool {
        matches!(self, Self::Secret)
    }
    #[must_use]
    pub const fn as_bool(self) -> bool {
        self.is_secret()
    }
    #[must_use]
    pub const fn from_bool(secret: bool) -> Self {
        if secret {
            Self::Secret
        } else {
            Self::NonSecret
        }
    }
}

// Production `crates/vb_core/src/workflow/mod.rs:658-670`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bound {
    pub min: Option<i64>,
    pub max: Option<i64>,
}

// Production `crates/vb_core/src/workflow/mod.rs:672-722`
impl Bound {
    pub const UNBOUNDED: Self = Self {
        min: None,
        max: None,
    };
    #[must_use]
    pub const fn new(min: Option<i64>, max: Option<i64>) -> Self {
        Self { min, max }
    }
    #[must_use]
    pub const fn lower(self) -> Option<i64> {
        self.min
    }
    #[must_use]
    pub const fn upper(self) -> Option<i64> {
        self.max
    }
    #[must_use]
    pub const fn is_unbounded(self) -> bool {
        self.min.is_none() && self.max.is_none()
    }
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
    #[must_use]
    pub const fn inconsistency(self) -> bool {
        match (self.min, self.max) {
            (Some(min), Some(max)) => min > max,
            _ => false,
        }
    }
}
