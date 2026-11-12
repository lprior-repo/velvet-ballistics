# Contract-as-Data Suite: contract.md

## Summary

Machine-readable contract schemas in CUE, an xtask `contracts` subcommand for discovery and validation, and enforcement of `schema_version`/`kind` invariants, integrated with existing `GateEvidence`/`GateStatus` pipeline.

## Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-001 | CUE schemas in `contracts/` for CLI envelopes, UI tokens, accepted artifacts, evidence bundles, diagnostics, gate outputs | Required |
| REQ-002 | xtask `contracts` subcommand walks `contracts/`, validates `schema_version` and `kind` fields, runs `cue vet`, reports pass/fail with `GateEvidence` | Required |
| REQ-003 | Every CUE contract file must declare `schema_version` (string, semver-like) and `kind` (enum of 6 types) | Required |
| REQ-004 | Contract-discovery gate integrates with existing `GateEvidence`/`GateStatus` in `xtask/src/evidence/tooling_and_gate_types.rs` | Required |
| REQ-005 | `schema_version` monotonicity: upgrading a schema must never decrease its version | Required |
| REQ-006 | `kind` completeness: every file in `contracts/` must have a recognized `kind` value | Required |
| REQ-007 | CUE files must pass `cue vet` with zero errors before acceptance | Required |
| REQ-008 | Discovery output is deterministic: sorted by file path, sorted diagnostics | Recommended |
| REQ-009 | `--json` flag on `xtask contracts` produces JSON output compatible with moon task consumers | Recommended |

## Assumptions

1. `contracts/` directory is workspace-root-relative.
2. CUE schemas use `package validation` (consistent with `.beads/schemas/`).
3. `cue` CLI is available on the CI machine (installed via `just install-cue`).
4. Existing `vb_validate` crate's `ValidationError` and `ValidationResult` types define the error taxonomy contract-discovery may report.
5. Contract files are CUE-native (`.cue` extension). YAML is legacy-only (`contracts/invariants.yaml`).

## Domain Model

### ContractFile

```
ContractFile {
    path: PathBuf          // Relative to workspace root, under contracts/
    schema_version: string  // "1.0.0" format
    kind: ContractKind      // Enum
    content: bytes          // Raw file content
    vet_errors: [string]    // From `cue vet`
}
```

### ContractKind (enum, closed set)

```
ContractKind: "cli_envelope" | "ui_tokens" | "accepted_artifacts" | "evidence_bundle" | "diagnostics" | "gate_output"
```

### DiscoveryReport

```
DiscoveryReport {
    files: [ContractFile]
    errors: [ValidationError]
    summary: ReportSummary
}
```

### ReportSummary

```
ReportSummary {
    total: u32
    valid: u32
    invalid: u32
    errors_by_kind: map[ContractKind] u32
    version_violations: [VersionViolation]
}
```

### VersionViolation

```
VersionViolation {
    file: PathBuf
    expected: string
    actual: string
    detail: string  // "monotonicity breach" or "unrecognized format"
}
```

### GateIntegration

```
// Maps DiscoveryReport to GateEvidence
gate_evidence_from_report(report: DiscoveryReport) -> GateEvidence
// Uses existing GateEvidence { kind, gate_name, command, exit_code, log, status, why_failed }
// kind = "contract-discovery"
// gate_name = "contracts"
// status = GateStatus::Pass if report.invalid == 0 else GateStatus::Fail
```

## Invariants

### INV-001: schema_version required
Every `.cue` file under `contracts/` must have a top-level `schema_version` field.
- **Verifier**: CUE schema `#ContractMeta` requires `schema_version: string`
- **Proof obligation**: `OBL-001` (proptest)

### INV-002: kind required and closed
Every `.cue` file under `contracts/` must have a top-level `kind` field whose value is one of the 6 enum members.
- **Verifier**: CUE schema `#ContractMeta` constrains `kind` to closed set
- **Proof obligation**: `OBL-002` (proptest)

### INV-003: cue vet passes
Every file in `contracts/` must pass `cue vet` with exit code 0.
- **Verifier**: `cue vet` executed by xtask subcommand
- **Proof obligation**: `OBL-003` (Kani harness on vet exit-code parsing)

### INV-004: schema_version monotonicity
When a file's `schema_version` is updated, the new version must be strictly greater than the previous version (major/minor/patch comparison).
- **Verifier**: xtask stores previous versions in `.beads/contracts/manifest.json`
- **Proof obligation**: `OBL-004` (Verus spec on semver comparison)

### INV-005: deterministic output
Discovery output is sorted by file path. Diagnostics within each file are sorted by error code.
- **Verifier**: Sort in xtask code before reporting
- **Proof obligation**: `OBL-005` (proptest on sorted output)

### INV-006: GateEvidence parity
Every contract-discovery run produces exactly one `GateEvidence` record. If the gate fails, `why_failed` is populated.
- **Verifier**: `gate_evidence_from_report` always returns `Ok(GateEvidence)`
- **Proof obligation**: `OBL-006` (Kani proof on GateEvidence construction)

## Type/Domain Model (Rust)

### ContractKind enum (binds to vb_validate ValidationError)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractKind {
    CliEnvelope,
    UiTokens,
    AcceptedArtifacts,
    EvidenceBundle,
    Diagnostics,
    GateOutput,
}

impl ContractKind {
    pub const fn all_values() -> &'static [Self] {
        &[
            Self::CliEnvelope,
            Self::UiTokens,
            Self::AcceptedArtifacts,
            Self::EvidenceBundle,
            Self::Diagnostics,
            Self::GateOutput,
        ]
    }
}
```

### ContractFile struct

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractFile {
    pub path: PathBuf,
    pub schema_version: String,
    pub kind: ContractKind,
    pub vet_errors: Vec<String>,
}
```

### DiscoveryReport struct

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryReport {
    pub files: Vec<ContractFile>,
    pub errors: Vec<ValidationError>,
    pub summary: ReportSummary,
}
```

### ReportSummary struct

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total: u32,
    pub valid: u32,
    pub invalid: u32,
    pub errors_by_kind: std::collections::HashMap<ContractKind, u32>,
    pub version_violations: Vec<VersionViolation>,
}
```

### VersionViolation struct

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionViolation {
    pub file: PathBuf,
    pub expected: String,
    pub actual: String,
    pub detail: String,
}
```

## Verification Layers

| Obligation | Verifier | Phase | Notes |
|------------|----------|-------|-------|
| OBL-001 (schema_version required) | proptest | Test | Generate random `.cue` content, assert `schema_version` present |
| OBL-002 (kind closed set) | proptest + Kani | Test + Formal | Exhaustive enum check via Kani; random content via proptest |
| OBL-003 (cue vet passes) | Kani | Formal | Harness on `parse_vet_output` — verify exit-code mapping never panics |
| OBL-004 (version monotonicity) | Verus | Formal | Spec on `compare_semver` function; proof binds to `VersionViolation` |
| OBL-005 (deterministic output) | proptest | Test | Shuffle input paths, assert output is identical |
| OBL-006 (GateEvidence parity) | Kani | Formal | Exhaustive proof: `gate_evidence_from_report` always returns `Ok(_)` |
| OBL-007 (no YAML in core) | forbidden-scan | CI | xtask is tooling, not runtime core — but verify `contracts/` is excluded |
| OBL-008 (cue vet zero errors) | CI gate | CI | moon task `contracts` must return exit 0 for CI pass |

## CUE Schema Templates

### contracts/cli_envelope.cue

```cue
package validation

#ContractMeta: {
	schema_version: string
	kind: "cli_envelope"
}

#CLIEnvelope: #ContractMeta & {
	command: string
	args: [...string]
	exit_codes: [...number]
}
```

### contracts/ui_tokens.cue

```cue
package validation

#ContractMeta: {
	schema_version: string
	kind: "ui_tokens"
}

#UITokens: #ContractMeta & {
	token_set: string
	properties: {
		[name: string]: {
			type: "color" | "spacing" | "typography" | "shadow" | "radius"
			value: string
		}
	}
}
```

### contracts/accepted_artifacts.cue

```cue
package validation

#ContractMeta: {
	schema_version: string
	kind: "accepted_artifacts"
}

#AcceptedArtifacts: #ContractMeta & {
	artifact_types: [...string]
	metadata_required: [...string]
}
```

### contracts/evidence_bundle.cue

```cue
package validation

#ContractMeta: {
	schema_version: string
	kind: "evidence_bundle"
}

#EvidenceBundle: #ContractMeta & {
	gates_required: [...string]
	evidence_shape: {
		gate_name: string
		exit_code: number
		status: "passed" | "failed" | "skipped"
	}
}
```

### contracts/diagnostics.cue

```cue
package validation

#ContractMeta: {
	schema_version: string
	kind: "diagnostics"
}

#Diagnostics: #ContractMeta & {
	error_codes: [...string]
	render_format: "text" | "json"
}
```

### contracts/gate_output.cue

```cue
package validation

#ContractMeta: {
	schema_version: string
	kind: "gate_output"
}

#GateOutput: #ContractMeta & {
	gate_kind: string
	gate_name: string
	status: "pass" | "fail" | "skipped"
	why_failed?: {
		hint: string
		repair_command: string
	}
}
```

### contracts/manifest.cue (registry of all contracts)

```cue
package validation

// Master manifest tracking all contract files and their versions
contract_registry: {
	[...string]: {
		path: string
		schema_version: string
		kind: "cli_envelope" | "ui_tokens" | "accepted_artifacts" | "evidence_bundle" | "diagnostics" | "gate_output"
		last_validated: string  // ISO8601
	}
}
```

## Integration with xtask Evidence Pipeline

### Existing types (from `tooling_and_gate_types.rs`):

```rust
// GateEvidence { kind, gate_name, command, exit_code, log, status, why_failed }
// GateStatus { Pass, Fail, Skipped { reason } }
// Error { GateTimeout, GateFailed, MissingEvidence, EvidenceWriteFailed, SubcommandNotFound, ... }
```

### New xtask command (cli.rs addition):

```rust
#[command(name = "contracts")]
Contracts {
    #[arg(long, default_value = "contracts")]
    dir: String,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    check: bool,  // If true, fail on any invalid contract
}
```

### Discovery flow:

1. `Contracts` command walks `contracts/` directory
2. For each `.cue` file:
   a. Parse top-level `schema_version` and `kind` fields
   b. Run `cue vet` on the file
   c. Collect vet errors
3. Build `DiscoveryReport`
4. Convert to `GateEvidence`:
   - `kind = "contract-discovery"`
   - `gate_name = "contracts"`
   - `command = "cargo xtask contracts --dir contracts"`
   - `exit_code = 0` if valid, `1` if any invalid
   - `status = Pass` if `invalid == 0`, else `Fail`
   - `why_failed = Some(...)` if gate failed

## Integration with vb_validate

The `vb_validate` crate's `ValidationError` enum already covers pattern-matching error codes. Contract-discovery adds:

```rust
// In vb_validate/src/lib.rs ValidationError enum:
#[error("MISSING_SCHEMA_VERSION")]
MissingSchemaVersion,

#[error("INVALID_KIND: {kind}")]
InvalidKind { kind: String },

#[error("CUE_VET_FAILED: {file}")]
CueVetFailed { file: String },

#[error("VERSION_MONOTONICITY_BREACH: {file} expected {expected} got {actual}")]
VersionMonotonicityBreach { file: String, expected: String, actual: String },
```

These map directly to `ContractFile.vet_errors` entries and `ReportSummary.version_violations`.

## Files to Create

1. `contracts/cli_envelope.cue`
2. `contracts/ui_tokens.cue`
3. `contracts/accepted_artifacts.cue`
4. `contracts/evidence_bundle.cue`
5. `contracts/diagnostics.cue`
6. `contracts/gate_output.cue`
7. `contracts/manifest.cue`
8. `xtask/src/contracts.rs` — new module for discovery
9. `xtask/src/cli.rs` — add `Contracts` command variant
10. `xtask/src/lib.rs` — export `contracts` module
11. `crates/vb_validate/src/lib.rs` — add 4 new `ValidationError` variants

## Follow-up Work

- Formal proofs for OBL-004 (Verus on semver comparison)
- CUE schema for existing `.beads/schemas/` directory (schema_version + kind on bead implementation schemas)
- moon task definition in `.moon/tasks.yaml` for `contracts`
- CI gate in moon configuration to run `cargo xtask contracts --check` on pull requests
