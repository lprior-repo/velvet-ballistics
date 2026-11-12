/// Semver-shaped schema version. Single-number wire versions (for example
/// journal schema versions) map to `major` with minor and patch zero via
/// [`SchemaVersion::from_major`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Major line. A change here breaks wire reading without migration.
    pub major: u16,
    /// Minor line. Additive change within a major line.
    pub minor: u16,
    /// Patch line. No structural change.
    pub patch: u16,
}

/// Current compiled-IR schema version. Admission stamps it onto accepted
/// artifacts; the admission/dispatch gates classify against it.
pub const CURRENT_IR_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl SchemaVersion {
    /// Builds a version triple.
    #[must_use]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Builds a version from a single wire number (minor and patch zero).
    #[must_use]
    pub const fn from_major(major: u16) -> Self {
        Self {
            major,
            minor: 0,
            patch: 0,
        }
    }
}
