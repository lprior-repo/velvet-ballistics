# Contract Specification: verify Hero Command and VerificationReport Certificates

## Context

- **Feature**: Implement `verify` as the master-doc primary operator gate for release readiness and artifact admission, emitting structured `VerificationReport` certificates.
- **Domain terms**:
  - `VerificationReport` — structured certificate with profile, artifact evidence, replay evidence, durability evidence, repair-hint, and exit-code evidence
  - `VerifyProfile` — `Quick | Standard | Full` (existing, from `args.rs`)
  - `VerifyOk` — existing success result with `digest_hex`, `checks`, `warnings`
  - `VerifyError` — existing error enum: `YamlParse`, `Compile`, `IrValidation`, `BudgetPolicy`
  - `VerificationProof` — from `vb_storage::admission`: `digest`, `gate_count`, `durable`, `warnings`
  - `AcceptedArtifact` — from `vb_storage::admission`: `digest`, `ir`, `verification`, `accepted_at_seq`, `required_capabilities`
  - `CliExitCode` — stable exit codes: `Success=0`, `ValidationFailed=1`, `VerificationFailed=2`, `CompileFailed=3`, `RuntimeFailed=4`, `StorageError=5`, `IpcError=6`, `ActionPolicyError=7`, `ReplayDivergence=8`
  - Repair hint — human-actionable guidance citing concrete failing gate and bead evidence reference where available
- **Assumptions**:
  - Existing `run_verification` in `commands_verify.rs` is the foundation; this bead extends it to full certificate-grade output
  - The `quick` profile performs only YAML parse and compilation (no IR validation or budget checks)
  - The `standard` profile adds IR validation gates
  - The `full` profile adds budget computation and boundedness policy checks, failing closed on violations
  - Storage admission gates (`submit_artifact`, `admit_compiled_artifact`) already exist and are discoverable
- **Open questions**:
  - Whether `--format json` output requires a top-level `VerificationReport` envelope or fits within the existing `{"success":true,...}` shape
  - ~~Whether `strict` profile durability evidence requires journal existence proof or is inferred from `Strict` durability mode flag~~ — **Resolved**: `strict` profile durability evidence is the `Strict` flag itself; `journal_written == false` confirms verify is read-only and that no journal record was created. The durability mode flag is sufficient evidence; no journal existence proof is required for verify's static-analysis contract.

## Preconditions

- **PRE-001**: `auth_required = false` — verify requires no authentication; any operator may invoke it.
- **PRE-002**: `required_inputs = [workflow_path]` — the only required input is a path to a workflow YAML file; no database path is required for the quick/standard profiles.
- **PRE-003**: `system_state` — the verifier/admission/storage gates must be discoverable at runtime; if `vb_validate`, `vb_compile`, or `vb_storage` symbols are unavailable, verify returns `CliExitCode::VerificationFailed` with a diagnostic message and exits cleanly (no panic).

## Postconditions

- **POST-001**: `verify` emits a `VerificationReport` in both text and machine-readable (`--format json`) modes. The report contains:
  - `profile` — the profile name applied (`quick`, `standard`, or `full`)
  - `artifact` — workflow source digest (hex), compiled IR digest (hex), node count
  - `replay` — replay evidence: which gates passed (names), sequence of gate names in execution order
  - `durability` — durability mode evidence: whether strict/journaled/none policy was checked and the result
  - `repair_hint` — for each failing gate, a concrete hint citing the gate name and suggested fix; empty when all gates pass
  - `exit_code` — the documented `u8` exit code value that will be returned (stable, never -1)
- **POST-002**: The `full` profile **fails closed** — if durability evidence is missing (no journal record for a strict-mode workflow) or replay evidence is incomplete (action ABI mismatch), verify exits with `CliExitCode::VerificationFailed` and emits a non-empty `repair_hint`.
- **POST-003**: Repair hints cite concrete failing gates by name (e.g., `"BudgetPolicy"`, `"IrValidation"`) and, where a related bead exists, the bead identifier (e.g., `"see vb-qi37.10.3"`). Repair hints never contain raw stack frames or panics.

## Invariants

- **INV-001**: Stable exit codes — the exit code returned by `verify` does not vary based on output format (`Text` vs `Json` vs `Jsonl`). The same failure always produces the same exit code.
- **INV-002**: Parity between human and machine output — for any given verify invocation, the set of failing gates reported in text mode is identical to the set reported in JSON mode. There is no gate that appears only in one format.
- **INV-003**: No panic propagation — `verify` must not let any raw panic, stack trace, or `unwrap` failure surface to the operator. All panics are caught, classified as `CliExitCode::VerificationFailed`, and reported as a clean diagnostic message.
- **INV-004**: Output completeness — when `--format json` is specified, the emitted JSON is valid UTF-8, parseable by a standard JSON parser, and contains all certificate fields without omission or truncation.

## Error Taxonomy

Each error variant maps to exactly one `CliExitCode`:

| Error variant | Exit code | JSON field | Text prefix |
|---|---|---|---|
| `YamlParse(String)` | `ValidationFailed` (1) | `"error"` with parse detail | `"YAML parse error: ..."` |
| `Compile(Vec<String>)` | `ValidationFailed` (1) | `"errors": [...]` | `"compile error: ..."` per error |
| `IrValidation(String)` | `VerificationFailed` (2) | `"error"` | `"IR validation failed: ..."` |
| `BudgetPolicy(String)` | `VerificationFailed` (2) | `"error"` | `"budget policy violation: ..."` |
| `StorageError(String)` | `StorageError` (5) | `"error"` | `"storage error: ..."` |
| `ReplayDivergence(String)` | `ReplayDivergence` (8) | `"error"` | `"replay divergence: ..."` |

The error taxonomy is **exhaustive**: there is no error variant that maps to multiple exit codes, and there is no `CliExitCode` that can be produced by verify but is not listed above.

## Contract Signatures

All fallible functions use `Result<T, Error>`:

```rust
// In main.rs (CLI entry point — the cmd_verify target used in BDD scenarios)
pub fn cmd_verify(
    workflow_path: &Path,
    profile: VerifyProfile,
    format: OutputFormat,
) -> CliExitCode;

// In commands_verify.rs
pub(crate) fn run_verification(
    text: &str,
    bytes: &[u8],
    profile: VerifyProfile,
) -> Result<VerifyOk, VerifyError>;

pub(crate) fn exit_code_for_error(err: &VerifyError) -> CliExitCode;

// New in this bead — certificate assembly
pub(crate) fn assemble_verification_report(
    result: &VerifyOk,
    profile: VerifyProfile,
    source_bytes: &[u8],
) -> VerificationReport;

pub(crate) fn repair_hint_for_error(err: &VerifyError, profile: VerifyProfile) -> Vec<RepairHint>;
```

`VerificationReport` and `RepairHint` are new types defined in `commands_verify.rs`:

```rust
pub(crate) struct VerificationReport {
    pub profile: &'static str,
    pub artifact: ArtifactEvidence,
    pub replay: ReplayEvidence,
    pub durability: DurabilityEvidence,
    pub repair_hints: Vec<RepairHint>,
    pub exit_code: u8,
}

pub(crate) struct ArtifactEvidence {
    pub source_digest_hex: String,
    pub ir_digest_hex: String,
    pub node_count: u16,
    pub passed_checks: Vec<&'static str>,
}

pub(crate) struct ReplayEvidence {
    pub gates_passed: Vec<&'static str>,
    pub gate_sequence: Vec<&'static str>,
    pub replay_safe: bool,
}

pub(crate) struct DurabilityEvidence {
    pub profile: VerifyProfile,
    pub durable: bool,
    pub journal_written: bool,
}

pub(crate) struct RepairHint {
    pub gate: &'static str,
    pub hint: String,
    pub bead_reference: Option<&'static str>,
}
```

## Non-goals

- verify does not execute the workflow — it is a static analysis gate only.
- verify does not write to the journal or produce `AcceptedArtifact` records; that is the domain of `submit`.
- verify does not prove semantic equivalence between IR and generated Rust; that is a separate codegen bead.
- verify does not perform runtime action dispatch or wait/ask suspension — those require a running shard.
- JSON output is machine-readable but is not a stable serialization format for programmatic consumption; `postcard` binary output is the stable machine format.
