# codebase-map.md — vb-ypnk Evidence Bundle Format

## Source Checkout
`/home/lewis/src/velvet-ballistics`

## Isolated Workspace
`/home/lewis/src/velvet-work/go-skill-vb-ypnk`

## Crates & Modules Affected

### xtask (primary)
- **xtask/src/lib.rs** — exports `evidence` module, command registry, routing, status
- **xtask/src/evidence.rs** — include-all for evidence module (16 submodules)
- **xtask/src/evidence/tooling_and_gate_types.rs** — `GateEvidence`, `GateStatus`, `WhyFailed`, `Error` enum, false-pass diagnostics, UI release lanes
- **xtask/src/evidence/persistence.rs** — `write_evidence()` (YAML), `evidence_path()`, `validate_evidence_dir()`, `explain_failure()`
- **xtask/src/evidence_gate.rs** (17.8K) — gate orchestration, runs moon ci, captures evidence
- **xtask/src/evidence/release_contract.rs** — contract discovery, schema version, cue vet
- **xtask/src/evidence/release_validation.rs** — release-level validation
- **xtask/src/evidence/release_model.rs** — release bead/model types
- **xtask/src/evidence/artifact_facts.rs** — artifact fact collection
- **xtask/src/evidence/raw_documents.rs** — raw document parsing
- **xtask/src/evidence/parsed_documents.rs** — parsed document types
- **xtask/src/evidence/fixture_parsers.rs** — fixture parsing
- **xtask/src/evidence/profile_runner.rs** — profile execution
- **xtask/src/evidence/release_rendering.rs** — release rendering
- **xtask/src/evidence/release_validators.rs** — release validators
- **xtask/src/evidence/error_profile_domain.rs** — error profile types
- **xtask/src/evidence/negative_fixtures.rs** — negative test fixtures
- **xtask/src/evidence/tests.rs** — evidence module tests
- **xtask/src/contracts.rs** — contract discovery (ContractKind includes `EvidenceBundle`)

### .evidence/ (data directory)
- `.evidence/<bead-id>/<gate-name>.yaml` — current evidence file layout
- `.evidence/` root — evidence root directory

## Key Existing Types

### GateEvidence (tooling_and_gate_types.rs:116)
```rust
pub struct GateEvidence {
    pub kind: String,         // category (fmt, clippy, miri, etc.)
    pub gate_name: String,    // specific gate
    pub command: String,      // full command string
    pub exit_code: i32,       // numeric exit code
    pub log: PathBuf,         // path to log file
    pub status: GateStatus,   // Pass | Fail | Skipped { reason }
    pub why_failed: Option<WhyFailed>, // diagnostic on failure
}
```

### write_evidence (persistence.rs:126)
```rust
pub fn write_evidence(evidence: &GateEvidence, path: &Path) -> Result<()>
```
Writes a single `GateEvidence` to YAML file.

### evidence_path (persistence.rs:110)
```rust
pub fn evidence_path(bead_id: &str, gate_name: &str) -> PathBuf
```
Returns `.evidence/<bead-id>/<gate-name>.yaml`

### Error enum (tooling_and_gate_types.rs:165)
```rust
pub enum Error {
    GateTimeout { gate, duration_secs },
    GateFailed { gate, exit_code, log },
    MissingEvidence { gate, path },
    EvidenceWriteFailed { gate, path, cause },
    SubcommandNotFound { name },
    BeadDirectoryCreationFailed { bead, cause },
    YamlSerializationFailed { gate, cause },
    UpstreamMoonFailed { task, cause },
    UpstreamJustFailed { recipe, cause },
}
```

## Dependency Scope
- **xtask** Cargo.toml — may need new deps for bundle format
- Workspace Cargo.toml — check for serde-saphyr version

## Gap Analysis (What the Bead Adds)
1. **EvidenceBundle** — a container type that wraps multiple `GateEvidence` records plus:
   - `schema_version: String` (for forward compatibility)
   - `executor_context: String` (who/what ran the gates)
   - `linked_bead: String` (bead ID linkage)
   - `source_test_mappings: Vec<SourceTestMapping>` (source→test coverage)
   - `release_gate_artifacts: Vec<ReleaseGateArtifact>` (release-specific artifacts)
2. **Bundle writers** — serialization beyond YAML (JSON, postcard for binary paths)
3. **Bead-scoped bundle directory** — `.evidence/<bead-id>/bundle.{json,yaml,postcard}`
4. **Validation** — fail-closed: missing required bundle fields rejected

## Risk Tags
- **integration** — new types must integrate with existing `evidence_gate.rs` flow
- **contract** — `ContractKind::EvidenceBundle` already exists in contracts.rs — must align
- **testing** — new bundle types need unit tests + property tests
- **CI** — bundle format must survive `moon ci`
