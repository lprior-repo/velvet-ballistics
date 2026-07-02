---
kind: evidence_bundle
schema_version: "1.0.0"
---

# Evidence Bundle Format and Writers — Contract

## 1. Requirements

### R-001 Bundle Container
The evidence bundle is a single serialisable document that aggregates gate execution
evidence with metadata about the execution context, source/test mappings, release
artifacts, and bead linkage. It must be self-contained: all required fields must be
present for the bundle to be valid.

### R-002 Schema Versioning
The bundle carries a `schema_version` string in `major.minor` form (e.g. `"1.0"`).
On deserialization, any bundle whose `major` version exceeds the consumer's supported
major version is rejected (forward compatibility: minor bumps are accepted if they
introduce only optional fields).

### R-003 Bead Linkage
Every bundle identifies the bead that produced it via `linked_bead_id`. This field
is **required** and non-empty.

### R-004 Executor Context
Every bundle records who/what ran the gates: the agent name or process name, an
ISO-8601 timestamp, and the target machine hostname. All three sub-fields are
**required**.

### R-005 Gate Evidence Array
A bundle carries zero or more `GateEvidence` records (re-used from the existing
`evidence::tooling_and_gate_types` module) that describe actual command outcomes.
The array may be empty for bundles that are produced by writers before gate execution
finishes (staging bundles).

### R-006 Source/Test Mappings
The bundle records which source files are covered by which tests via
`SourceTestMapping` entries. Each entry maps a single source file path to a list of
test names (test harness or function names).

### R-007 Release Gate Artifacts
The bundle records metadata about release-gate artifacts via
`ReleaseGateArtifact` entries. Each entry includes name, path, a content digest, and
a type discriminator.

### R-008 Serialization Formats
The bundle MUST support three serialization formats:
- `Yaml` — human-readable, used for editorial review.
- `Json` — machine-readable, used by CI tooling.
- `Postcard` — binary, used for compact on-disk storage and CI artifact passing.

### R-009 Writers
Provide writer functions that serialise and write a bundle to a file path, returning
`Result<(), Error>`. The writer must create parent directories if they do not exist.

### R-010 Reader / Validator
Provide a reader function that deserialises a bundle from a file and a validator
function that performs fail-closed checks on the deserialised data. Missing required
fields trigger rejection.

### R-011 Path Helper
Provide `bundle_path(bead_id, format)` that returns `.evidence/<bead-id>/bundle.<ext>`
mirroring the existing `evidence_path(bead_id, gate_name)` convention.

### R-012 Postcard for Binary Paths
Per the master contract, Postcard is required for binary paths. The `Postcard`
variant in `EvidenceBundleFormat` must be used when serialising bundles that contain
binary paths (release artifact paths).

## 2. Assumptions

| # | Assumption |
|---|------------|
| A-001 | The `GateEvidence`, `GateStatus`, and `WhyFailed` types from `xtask/src/evidence/tooling_and_gate_types.rs` are available and serialisable with `serde`. |
| A-002 | The `Error` enum in `xtask/src/evidence/tooling_and_gate_types.rs` is the canonical error type for evidence operations and new variants may be added to it. |
| A-003 | `serde_saphyr` is available for YAML serialisation (already used in `write_evidence`). |
| A-004 | `serde_json` is available for JSON serialisation. |
| A-005 | `postcard` is available for binary serialisation (already a workspace dependency). |
| A-006 | The `contracts.rs` module's `ContractKind::EvidenceBundle` variant is the recognised kind value; this contract does not modify it. |
| A-007 | The existing `evidence_path()` function returns `.evidence/<bead-id>/<gate-name>.yaml`. The new `bundle_path()` follows the same directory convention. |
| A-008 | No YAML, JSON, or HTTP may appear in the **runtime core** (`vb_core`). The `xtask` crate is a build tool and is exempt from this restriction. |
| A-009 | All types must derive `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`. |
| A-010 | The `#[serde(rename_all = "snake_case")]` attribute is applied to enums and structs with string fields that appear in serialised output. |
| A-011 | Zero `unwrap`, `expect`, `panic`, `todo`, or `unimplemented` in new code. All fallible operations return `Result<T, Error>`. |
| A-012 | Schema version format is `major.minor` (e.g., `"1.0"`), NOT the `X.Y.Z` semver used by `parse_schema_version` in `contracts.rs`. A separate validator (`parse_bundle_schema_version`) is used for bundles. |

## 3. Invariants

| ID | Invariant |
|----|-----------|
| INV-001 | Every `EvidenceBundle` must identify the command (gate_name + command string), executor context, observed outcome (status), and linked bead ID. If any of these are absent on deserialization, the bundle is rejected. |
| INV-002 | The schema version string must be parseable as `major.minor` where both parts are non-negative integers without leading zeros. On deserialization, if parsing fails the bundle is rejected. |
| INV-003 | A bundle is forward-compatible: a consumer supporting `major.N` must accept any bundle whose major version equals `N` and whose minor version is `>= N.minor`, provided all required fields for the consumer's minor version are present. |
| INV-004 | A bundle is self-contained: `linked_bead_id` is non-empty, `executor_context` has all three sub-fields non-empty, and all `GateEvidence` entries have `status` set. |
| INV-005 | Missing required fields trigger rejection (fail-closed). The validator MUST return an `Error::MissingRequiredField` for every absent required field. |
| INV-006 | Postcard-serialised bundles must not lose information compared to their YAML/JSON counterparts: deserialising a Postcard bundle yields an in-memory value byte-identical to deserialising the equivalent YAML/JSON bundle. |
| INV-007 | The `validate_bundle` function MUST check every required field and return a `Vec<Error>` (one entry per missing field). An empty vec means valid; a non-empty vec means rejected. |

## 4. Type / Domain Model

### 4.1 `EvidenceBundle`

```rust
/// Top-level evidence bundle container.
///
/// Self-contained: all required fields must be present.
/// Rejected by the validator if any required field is missing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceBundle {
    /// Schema version in major.minor form, e.g. "1.0".
    /// Required. Parsed and validated before use.
    pub schema_version: String,

    /// Who/what ran the gates (agent name, timestamp, machine).
    /// Required. All three sub-fields must be non-empty.
    pub executor_context: ExecutorContext,

    /// The bead that produced this bundle.
    /// Required. Must be non-empty.
    pub linked_bead_id: String,

    /// Gate execution evidence records.
    /// May be empty (staging bundles).
    pub gates: Vec<GateEvidence>,

    /// Source file → test name coverage mappings.
    /// May be empty.
    pub source_test_mappings: Vec<SourceTestMapping>,

    /// Release-gate artifact metadata.
    /// May be empty.
    pub release_artifacts: Vec<ReleaseGateArtifact>,
}
```

### 4.2 `ExecutorContext`

```rust
/// Metadata about the execution that produced the bundle.
///
/// All three sub-fields are required.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExecutorContext {
    /// Agent name or process name that ran the gates, e.g. "claude", "opencode", "manual".
    pub agent: String,

    /// ISO-8601 UTC timestamp of execution, e.g. "2025-01-15T10:30:00Z".
    pub timestamp: String,

    /// Machine hostname or CI runner identifier.
    pub machine: String,
}
```

### 4.3 `SourceTestMapping`

```rust
/// Maps a single source file path to the test names that cover it.
///
/// `source_path` is required and must be a non-empty string.
/// `tests` may be empty (documented knowledge, not yet exercised by tests).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SourceTestMapping {
    /// Source file path relative to workspace root, e.g. "crates/vb_core/src/lib.rs".
    pub source_path: String,

    /// Test names (harness or function) that exercise this source file.
    pub tests: Vec<String>,
}
```

### 4.4 `ReleaseGateArtifact`

```rust
/// Metadata for a release-gate artifact.
///
/// All fields are required. `digest` encodes the algorithm prefix, e.g.
/// "sha256:a1b2c3...".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleaseGateArtifact {
    /// Human-readable artifact name, e.g. "vb_ui_snapshot_wasm".
    pub name: String,

    /// File path or URI where the artifact is stored.
    /// For binary paths, Postcard serialisation is required (INV-007).
    pub path: String,

    /// Content digest with algorithm prefix, e.g. "sha256:a1b2c3d4...".
    pub digest: String,

    /// Artifact type discriminator.
    #[serde(rename = "type")]
    pub artifact_type: ArtifactType,
}

/// Discriminator for release-gate artifact kinds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Binary,
    Documentation,
    Snapshot,
    Checksum,
    Other,
}
```

### 4.5 `EvidenceBundleFormat`

```rust
/// Serialization format for evidence bundle output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceBundleFormat {
    /// Human-readable YAML (via serde_saphyr).
    Yaml,
    /// Machine-readable JSON (via serde_json).
    Json,
    /// Binary, compact Postcard (required for binary paths per master contract).
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
```

## 5. Error Enum Extensions

The following variants are added to the existing `Error` enum in
`xtask/src/evidence/tooling_and_gate_types.rs`:

```rust
// New variants appended to the existing Error enum:

/// Schema version string could not be parsed as major.minor.
SchemaVersionParseFailed { version: String },

/// A required bundle field was missing on deserialisation.
MissingRequiredField { field: String },

/// Bundle-level serialisation failed for the chosen format.
BundleSerializationFailed { format: String, cause: String },
```

Each new variant must implement `Display` consistent with existing patterns:
```rust
Error::SchemaVersionParseFailed { version } => write!(f, "Schema version parse failed: '{version}'"),
Error::MissingRequiredField { field } => write!(f, "Missing required field: '{field}'"),
Error::BundleSerializationFailed { format, cause } => write!(f, "Bundle serialization ({format}) failed: {cause}"),
```

## 6. Public API Surface

### 6.1 Schema Version Validator

```rust
/// Parse a bundle schema version in major.minor form.
/// Returns the original string on success.
///
/// Format: "major.minor" where both parts are non-negative integers
/// without leading zeros (except "0" itself).
///
/// # Errors
/// Returns `Error::SchemaVersionParseFailed` if the format is invalid.
pub fn parse_bundle_schema_version(s: &str) -> Result<String, Error>;
```

### 6.2 Path Helper

```rust
/// Construct the bundle file path for a given bead and format.
///
/// Path is `.evidence/<bead-id>/bundle.<ext>` mirroring the existing
/// `evidence_path` convention.
///
/// # Example
/// ```
/// // .evidence/vb-abc123/bundle.yaml
/// bundle_path("vb-abc123", EvidenceBundleFormat::Yaml)
/// ```
pub fn bundle_path(bead_id: &str, format: EvidenceBundleFormat) -> PathBuf;
```

### 6.3 Writer

```rust
/// Serialise and write an `EvidenceBundle` to disk.
///
/// # Arguments
/// * `bundle` — The bundle to serialise.
/// * `path` — Target file path (use `bundle_path` to construct).
/// * `format` — Serialization format.
///
/// # Errors
/// Returns `Error::BundleSerializationFailed` if serialisation fails.
/// Returns `Error::EvidenceWriteFailed` if file write fails.
///
/// Creates parent directories if they do not exist.
pub fn write_bundle(bundle: &EvidenceBundle, path: &Path, format: EvidenceBundleFormat) -> Result<()>;
```

### 6.4 Reader

```rust
/// Deserialise an `EvidenceBundle` from a file.
///
/// # Errors
/// Returns `Error::BundleSerializationFailed` if the file contents cannot
/// be deserialised into an `EvidenceBundle` for the given format.
/// Returns `Error::EvidenceWriteFailed` if the file cannot be read.
pub fn read_bundle(path: &Path, format: EvidenceBundleFormat) -> Result<EvidenceBundle>;
```

### 6.5 Validator (Fail-Closed)

```rust
/// Validate a deserialised bundle's required fields.
///
/// Returns an empty vec if the bundle is valid.
/// Returns one `Error::MissingRequiredField` per absent required field.
/// This is the fail-closed gate: any non-empty result rejects the bundle.
///
/// Checked fields:
/// - `schema_version` is non-empty and parseable
/// - `linked_bead_id` is non-empty
/// - `executor_context.agent` is non-empty
/// - `executor_context.timestamp` is non-empty
/// - `executor_context.machine` is non-empty
/// - every `GateEvidence.status` variant is set (not default)
pub fn validate_bundle(bundle: &EvidenceBundle) -> Result<Vec<Error>>;
```

## 7. File Location

All new types and functions live in a new module:

```
xtask/src/evidence/bundle.rs          # Bundle types, writer, reader, validator
xtask/src/evidence.rs                 # Updated to include: include!("evidence/bundle.rs");
```

The module is private to `evidence` (no `pub mod bundle` in lib.rs) — the types
are re-exported from `evidence` if callers need them, following the existing pattern
where `GateEvidence` is used via `crate::evidence::GateEvidence`.

## 8. Verification Layers

| Layer | Tool | Obligation | Rationale |
|-------|------|------------|-----------|
| L1 | Kani | `write_bundle` does not panic for any valid serialisable bundle | Proves non-panic on the write path |
| L2 | Kani | `read_bundle` does not panic when the file contains well-formed but unexpected fields | Proves non-panic on the read path |
| L3 | Kani | `validate_bundle` returns empty vec iff all required fields are non-empty | Proves validator correctness |
| L4 | Kani | `parse_bundle_schema_version` correctly rejects leading zeros and malformed input | Proves version parsing |
| L5 | Property tests (proptest) | Round-trip: serialise → deserialise yields byte-identical in-memory value for Yaml, Json, and Postcard | Proves INV-006 |
| L6 | Property tests (proptest) | `validate_bundle` rejects bundles with empty `linked_bead_id` | Proves fail-closed |
| L7 | Property tests (proptest) | `bundle_path` produces deterministic paths for the same inputs | Proves path determinism |

## 9. Initial Proof Obligations

| ID | Proof Target | Description |
|----|-------------|-------------|
| OBL-001 | `parse_bundle_schema_version` | For any input string `s`, if `parse_bundle_schema_version(s).is_ok()`, then `s` matches `^(0|[1-9][0-9])\.(0|[1-9][0-9])$`. |
| OBL-002 | `validate_bundle` | For any bundle `b`, `validate_bundle(&b).is_empty()` iff `!b.schema_version.is_empty() && !b.linked_bead_id.is_empty() && !b.executor_context.agent.is_empty() && !b.executor_context.timestamp.is_empty() && !b.executor_context.machine.is_empty()`. |
| OBL-003 | `write_bundle` | For any serialisable bundle `b` and any valid path `p`, `write_bundle(&b, p, fmt)` does not panic and returns `Ok(())` or a descriptive `Error`. |
| OBL-004 | `read_bundle` | For any well-formed serialised bundle on disk at path `p` in format `fmt`, `read_bundle(p, fmt)` returns `Ok(bundle)` where `bundle` is a valid `EvidenceBundle`. |
| OBL-005 | Round-trip | For any valid `EvidenceBundle` `b`, `read_bundle(write_bundle(&b, p, fmt), fmt).unwrap()` produces a bundle equal to `b` in all fields (structural equality). |

## 10. Forward Compatibility Notes

- Minor version bumps may add optional fields to `EvidenceBundle`, `SourceTestMapping`,
  or `ReleaseGateArtifact`. These fields MUST be annotated with `#[serde(skip_serializing_if = "Option::is_none")]`
  and default to `None` via `#[serde(default)]`.
- Major version bumps require a new struct or a tagged union. The validator MUST reject
  bundles whose major version exceeds the consumer's supported major version.
- The schema version validator (`parse_bundle_schema_version`) enforces the `major.minor`
  format; future versions may extend it to accept `major.minor.patch` if needed.
