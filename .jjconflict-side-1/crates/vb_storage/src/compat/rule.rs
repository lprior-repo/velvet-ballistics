/// The rule that decided a classification. This is what the decision
/// record's rule digest covers, distinct from the decision code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum CompatRule {
    /// Held and required versions are equal.
    IdenticalVersions = 0,
    /// Same major line; the consumer is newer than the data.
    SameMajorNewerConsumer = 1,
    /// Same major line; the consumer is older than the data.
    SameMajorOlderConsumer = 2,
    /// Major lines differ and a migration path is declared.
    MajorChangeMigrationAvailable = 3,
    /// Major lines differ with no migration path but a replay allowance.
    MajorChangeReplayOnly = 4,
    /// Major lines differ with neither migration nor replay allowance.
    MajorChangeUnsupported = 5,
}

impl CompatRule {
    /// Stable name used in rule-digest encoding and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::IdenticalVersions => "identical_versions",
            Self::SameMajorNewerConsumer => "same_major_newer_consumer",
            Self::SameMajorOlderConsumer => "same_major_older_consumer",
            Self::MajorChangeMigrationAvailable => "major_change_migration_available",
            Self::MajorChangeReplayOnly => "major_change_replay_only",
            Self::MajorChangeUnsupported => "major_change_unsupported",
        }
    }
}
