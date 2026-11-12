use super::DomainPath;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceRoot {
    pub(crate) path: DomainPath,
}

impl WorkspaceRoot {
    #[must_use]
    pub fn new(path: &str) -> Self {
        Self {
            path: DomainPath::new(path),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InventoryScope {
    FirstPartyRustTests,
    Roots(Vec<String>),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TestFile {
    pub path: DomainPath,
}

impl TestFile {
    #[must_use]
    pub fn new(path: &str) -> Self {
        Self {
            path: DomainPath::new(path),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceText {
    Text(String),
    ReadFailed { operation: String },
    InvalidUtf8 { byte_offset: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Location {
    pub line: u32,
    pub column: u32,
}

impl Location {
    #[must_use]
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}
