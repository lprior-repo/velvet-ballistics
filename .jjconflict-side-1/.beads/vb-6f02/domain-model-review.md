# Domain Model Review: contracts-as-data

## Review of ContractFile

**Assessment**: The `ContractFile` struct correctly models the minimal metadata required per contract file.

**Fields**:
- `path: PathBuf` — relative path under `contracts/`. Uses `PathBuf` (not `&str`) because the xtask discovery walks the filesystem and produces owned paths.
- `schema_version: String` — semver-like string. Must be validated against pattern `^\d+\.\d+\.\d+$`.
- `kind: ContractKind` — closed enum. Prevents invalid kinds at the type level.
- `vet_errors: Vec<String>` — captures all `cue vet` errors. Empty vec means file passes vet.

**Concerns**: None. This matches the pattern used by `GateEvidence` in `tooling_and_gate_types.rs` (owned `PathBuf` and `Vec<String>`).

## Review of ContractKind

**Assessment**: Closed enum with exactly 6 values is correct. Matches the 6 schema categories in the bead scope.

**Bind to vb_validate**: `ContractKind` values correspond to CUE schema files. The `kind` field in CUE schemas must match one of these enum values. Invalid values are caught at parse time with `ValidationError::InvalidKind`.

**No-unwrap guarantee**: `ContractKind::try_from_str` should return `Result<Self, ValidationError>` (not `unwrap` or `expect`). The 6 values are exhaustive at the enum definition site, so pattern matching is complete.

## Review of DiscoveryReport

**Assessment**: `DiscoveryReport` aggregates per-file results into a summary. The structure mirrors `GateEvidence` semantics:
- `files` — per-file results
- `errors` — crate-level errors (different from per-file vet errors)
- `summary` — high-level counts

**Bind to vb_validate**: `errors: Vec<ValidationError>` reuses the existing error type, ensuring that contract-discovery errors flow through the same diagnostic pipeline as workflow validation errors.

## Review of ReportSummary

**Assessment**: Counts are `u32` (not `usize`) for JSON serialization compatibility. `errors_by_kind` is a `HashMap` — this is acceptable because:
1. The map has at most 6 entries (closed enum size)
2. It's only used in JSON output, not hot path

**Concern**: `errors_by_kind` should use `BTreeMap` instead of `HashMap` for deterministic ordering. `BTreeMap` serializes to sorted JSON keys, satisfying INV-005.

**Fix**: Change `errors_by_kind` to `BTreeMap<ContractKind, u32>` (requires `Ord` impl on `ContractKind`).

## Review of VersionViolation

**Assessment**: The struct captures the before/after version comparison. `expected` and `actual` are strings (not parsed semver tuples) to avoid complex parsing in the violation record. The `detail` field explains the nature of the violation.

**Bind to vb_validate**: Maps to `ValidationError::VersionMonotonicityBreach`. The `file` field corresponds to the `.cue` file path, `expected` is the stored previous version from manifest, `actual` is the current file's `schema_version`.

**No-unwrap guarantee**: No parsing on violation creation — strings are stored as-is. Parsing happens earlier in the discovery pipeline.

## Review of GateEvidence Integration

**Assessment**: The mapping from `DiscoveryReport` to `GateEvidence` is straightforward:
- `kind = "contract-discovery"` — new gate category
- `gate_name = "contracts"` — subcommand name
- `exit_code = if invalid == 0 { 0 } else { 1 }` — simple pass/fail
- `status = if invalid == 0 { Pass } else { Fail }` — mirrors exit code
- `why_failed` — populated only on `Fail`, with hint pointing to `cargo xtask contracts --json`

**Bind to existing code**: `GateEvidence` and `GateStatus` are already defined in `xtask/src/evidence/tooling_and_gate_types.rs`. The conversion function `gate_evidence_from_report` is pure (no I/O), making it trivially testable.

**Concern**: The `log` field of `GateEvidence` should point to `.evidence/contracts/last_run.log` (created by the xtask command). This matches the existing pattern where `log` is a `PathBuf` to a local log file.

## Review of CUE Schema Templates

**Assessment**: All 6 CUE schemas share a `#ContractMeta` base type requiring `schema_version` and `kind`. This enforces INV-001 and INV-002 at the schema level (not just at the Rust level). The `manifest.cue` provides a registry that the xtask can use for monotonicity checking.

**Concern**: The `#ContractMeta` type should include a `last_validated` field (ISO8601 string) so the manifest tracks when each contract was last checked. This is used by `VersionViolation` detection.

**Recommendation**: Add `last_validated: string` to `#ContractMeta` and set it to `"2026-05-17T00:00:00Z"` as placeholder.

## Review of vb_validate Integration

**Assessment**: Adding 4 new `ValidationError` variants to the existing enum is the right approach. These variants:
1. Follow the existing naming convention (SCREAMING_SNAKE_CASE error codes)
2. Use the existing `#[error(...)]` derive pattern from `thiserror`
3. Carry the same field types as used in the domain model (`String`, `PathBuf`)

**Concern**: The `ValidationError` enum already has 70+ variants. Adding 4 more is fine for now, but a long-term migration to a new `ContractError` enum separate from `WorkflowValidationError` would be cleaner. For this bead, keeping them together is acceptable because:
1. Contract-discovery is an xtask command (cold path)
2. The bead scope is "quality" not "runtime"
3. Adding a new crate would be over-engineering

## Final Verdict

**PASS with 2 fixes required**:

1. Change `ReportSummary.errors_by_kind` from `HashMap` to `BTreeMap` for deterministic JSON output (INV-005).
2. Add `last_validated` field to `#ContractMeta` CUE schema for manifest tracking.

Both fixes are in the contract.md CUE templates and Rust types. Implementation should include them.
