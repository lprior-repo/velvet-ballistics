use super::InventoryError;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DomainPath(pub(crate) String);

impl DomainPath {
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self(value.to_owned())
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindingId(pub(crate) String);

impl FindingId {
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self(value.to_owned())
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportLocation(pub(crate) String);

impl ReportLocation {
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self(value.to_owned())
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnerName(pub(crate) String);

impl OwnerName {
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportAction(pub(crate) String);

impl ReportAction {
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExceptionReason(pub(crate) String);

impl ExceptionReason {
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExceptionScope(pub(crate) String);

impl ExceptionScope {
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BehaviorEvidence(pub(crate) String);

impl BehaviorEvidence {
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseLabel(pub(crate) String);

impl CaseLabel {
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseEvidence(pub(crate) Vec<String>);

impl CaseEvidence {
    pub fn new(values: Vec<String>) -> Result<Self, InventoryError> {
        if values.is_empty() {
            Err(InventoryError::AmbiguousCaseLabel {
                label: String::new(),
                behavior: None,
                case_count: 0,
            })
        } else {
            Ok(Self(values))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationImprovementClaim(pub(crate) String);

impl MutationImprovementClaim {
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self(value.to_owned())
    }
}
