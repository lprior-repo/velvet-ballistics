/// The twelve MASTER §77.17 boundaries. Input and output schemas share one
/// boundary, matching the spec's "input/output schemas" item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum CompatBoundary {
    /// Compiled workflow IR versions.
    IrVersion = 0,
    /// Action ABI versions.
    ActionAbi = 1,
    /// Input and output schema versions.
    InputOutputSchema = 2,
    /// Policy input schema versions.
    PolicyInputSchema = 3,
    /// Signal schema versions.
    SignalSchema = 4,
    /// Journal event versions.
    JournalEventVersion = 5,
    /// Snapshot versions.
    SnapshotVersion = 6,
    /// Debug map versions.
    DebugMapVersion = 7,
    /// Resource contract versions.
    ResourceContract = 8,
    /// Incident bundle versions.
    IncidentBundle = 9,
    /// SDK ABI versions.
    SdkAbi = 10,
    /// Artifact envelope versions.
    ArtifactEnvelope = 11,
}

impl CompatBoundary {
    /// All twelve boundaries, in spec order.
    pub const ALL: [CompatBoundary; 12] = [
        CompatBoundary::IrVersion,
        CompatBoundary::ActionAbi,
        CompatBoundary::InputOutputSchema,
        CompatBoundary::PolicyInputSchema,
        CompatBoundary::SignalSchema,
        CompatBoundary::JournalEventVersion,
        CompatBoundary::SnapshotVersion,
        CompatBoundary::DebugMapVersion,
        CompatBoundary::ResourceContract,
        CompatBoundary::IncidentBundle,
        CompatBoundary::SdkAbi,
        CompatBoundary::ArtifactEnvelope,
    ];

    /// Stable name used in rule-digest encoding and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::IrVersion => "ir_version",
            Self::ActionAbi => "action_abi",
            Self::InputOutputSchema => "input_output_schema",
            Self::PolicyInputSchema => "policy_input_schema",
            Self::SignalSchema => "signal_schema",
            Self::JournalEventVersion => "journal_event_version",
            Self::SnapshotVersion => "snapshot_version",
            Self::DebugMapVersion => "debug_map_version",
            Self::ResourceContract => "resource_contract",
            Self::IncidentBundle => "incident_bundle",
            Self::SdkAbi => "sdk_abi",
            Self::ArtifactEnvelope => "artifact_envelope",
        }
    }
}
