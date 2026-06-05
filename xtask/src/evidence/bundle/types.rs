// Bundle type definitions.
//
// Types are in scope from include! directives in evidence.rs:
// Error, GateEvidence, GateStatus, GateStatusKind, Path, PathBuf, Serialize, Deserialize

// ── Bundle Container ─────────────────────────────────────────────────────────

/// Top-level evidence bundle container.
///
/// Self-contained: all required fields must be present.
/// Rejected by the validator if any required field is missing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceBundle {
    /// Schema version in major.minor form, e.g. "1.0".
    pub schema_version: String,
    /// Who/what ran the gates (agent name, timestamp, machine).
    pub executor_context: ExecutorContext,
    /// The bead that produced this bundle.
    pub linked_bead_id: String,
    /// Gate execution evidence records. May be empty (staging bundles).
    pub gates: Vec<GateEvidence>,
    /// Source file -> test name coverage mappings. May be empty.
    pub source_test_mappings: Vec<SourceTestMapping>,
    /// Release-gate artifact metadata. May be empty.
    pub release_artifacts: Vec<ReleaseGateArtifact>,
}

// ── Executor Context ─────────────────────────────────────────────────────────

/// Metadata about the execution that produced the bundle.
///
/// All three sub-fields are required.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExecutorContext {
    /// Agent name or process name that ran the gates.
    pub agent: String,
    /// ISO-8601 UTC timestamp of execution.
    pub timestamp: String,
    /// Machine hostname or CI runner identifier.
    pub machine: String,
}

// ── Source/Test Mappings ─────────────────────────────────────────────────────

/// Maps a single source file path to the test names that cover it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SourceTestMapping {
    /// Source file path relative to workspace root.
    pub source_path: String,
    /// Test names (harness or function) that exercise this source file.
    pub tests: Vec<String>,
}

// ── Release Gate Artifacts ────────────────────────────────────────────────────

/// Metadata for a release-gate artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleaseGateArtifact {
    /// Human-readable artifact name.
    pub name: String,
    /// File path or URI where the artifact is stored.
    pub path: String,
    /// Content digest with algorithm prefix, e.g. "sha256:a1b2c3d4...".
    pub digest: String,
    /// Artifact type discriminator (field serialised as "type").
    #[serde(rename = "type")]
    pub artifact_type: ArtifactType,
}

/// Discriminator for release-gate artifact kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Benchmark,
    Coverage,
    Mutation,
    SupplyChain,
    Miri,
    Clippy,
    Fmt,
}

// ── Serialization Format ─────────────────────────────────────────────────────

/// Serialization format for evidence bundle output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceBundleFormat {
    /// Human-readable YAML-compatible evidence document.
    Yaml,
    /// Machine-readable JSON (via serde_json).
    Json,
    /// Binary, compact Postcard.
    Postcard,
}

impl EvidenceBundleFormat {
    /// File extension for this format.
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Postcard => "postcard",
        }
    }
}

/// Convert format to a display string.
pub fn format_to_string(format: EvidenceBundleFormat) -> String {
    match format {
        EvidenceBundleFormat::Yaml => "yaml".to_string(),
        EvidenceBundleFormat::Json => "json".to_string(),
        EvidenceBundleFormat::Postcard => "postcard".to_string(),
    }
}

// ── Bundle Path Helpers ───────────────────────────────────────────────────────

/// Construct the bundle file path for a given bead and format.
///
/// Path is `.evidence/<bead-id>/bundle.<ext>`.
pub fn bundle_path(bead_id: &str, format: EvidenceBundleFormat) -> PathBuf {
    PathBuf::from(".evidence")
        .join(bundle_path_component(bead_id))
        .join(format!("bundle.{}", format.extension()))
}

fn bundle_path_component(bead_id: &str) -> String {
    let component: String = bead_id
        .chars()
        .map(|ch| match ch {
            '/' | '\\' => '_',
            other => other,
        })
        .collect();

    if component.is_empty() {
        String::from("unknown")
    } else {
        component
    }
}
