# Contract Specification: vb-qi37.13.1 - cli: Define structured envelope schemas

## 1. Scope

Define the contract for typed structured CLI output envelope schemas for `velvet-ballistics` v1. This bead is schema-definition only: it may introduce or specify cold-path CLI schema types, kind vocabulary, version constants, validation rules, and mapping rules, but it must not flip existing command emitters from JSON/JSONL to YAML/Postcard and must not implement production code or tests in this State 3 artifact.

The canonical v1 structured output formats are:

- `--emit yaml` for machine-readable structured text.
- `--emit postcard` for compact machine-readable binary output where supported.

JSON/JSONL are not canonical v1 structured output formats. If retained temporarily, they are compatibility adapters only and must not define the schema source of truth.

## 2. Context Read

Inputs inspected:

- `.beads/vb-qi37.13.1/codebase-map.md`
- `crates/velvet_ballistics/src/args.rs`
- `crates/velvet_ballistics/src/main.rs` search results for current JSON helpers and help text
- `crates/velvet_ballistics/src/agent_context.rs`
- `crates/velvet_ballistics/src/exit_code.rs`
- `crates/velvet_ballistics/src/commands_verify.rs`
- `crates/velvet_ballistics/src/commands_status.rs`
- `crates/velvet_ballistics/src/commands_ai_context.rs`
- `crates/velvet_ballistics/src/commands_diff.rs`
- `crates/velvet_ballistics/src/commands_incident.rs`
- `crates/velvet_ballistics/src/commands_journal.rs`
- `crates/velvet_ballistics/src/commands_workflow.rs`
- `crates/vb_storage/src/types.rs`
- `crates/vb_storage/src/codec.rs`
- `velvet-ballistics-MASTER.md` search results for CLI and envelope constraints

## 3. Domain Terms

- Text envelope: deterministic YAML-serializable typed output wrapper for cold CLI reports.
- Binary envelope: Postcard payload plus fixed validated header for CLI machine output.
- Payload: typed command-specific report data contained by an envelope.
- Kind: stable discriminator identifying the payload family.
- Diagnostic: structured non-data information about validation, storage, runtime, redaction, or policy failures.
- Exit code: stable `CliExitCode` vocabulary from `exit_code.rs`, not an ad hoc integer space.
- Taint: explicit sensitivity marker used to prove diagnostics and reports do not leak secret data.
- Cold CLI schema: allowed outside runtime core; forbidden from entering `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`, or generated workflow code except by design reference to storage envelopes.

## 4. Assumptions

- Schema types will live in `crates/velvet_ballistics/src/` or a future cold CLI-support crate, not in runtime crates.
- `velvet-ballistics/cli-output/v1` is the canonical text schema version string.
- Binary CLI schema version is `1_u16` unless a later approved migration contract changes it.
- The existing `CliExitCode` discriminants remain canonical for process status and envelope status semantics.
- `agent-context` may remain JSON temporarily only as a compatibility exception until a downstream migration bead updates it.

## 5. Open Questions For Later States

- Which commands initially support `--emit postcard` versus YAML-only structured output?
- Should legacy `--json` and `--jsonl` be rejected, hidden, or retained as cold adapters during migration?
- Should `generated_at_ms` be omitted by default to preserve deterministic snapshots, or included only when explicitly requested?
- Should diagnostics be duplicated inside stdout envelopes, or should stdout envelopes only contain a diagnostic summary while full diagnostics stay on stderr?

This contract resolves the ambiguity as follows for v1: stdout envelopes may include `diagnostics_summary`, but full diagnostics belong to stderr diagnostic envelopes or text stderr. Machine stdout remains data-only.

## 6. Preconditions

### P1. Runtime-core isolation

Envelope schema definitions must be added only to `velvet_ballistics` cold CLI code or a cold CLI-support crate. They must not be added to `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`, or generated workflow code.

### P2. Stable version constants

Every structured text envelope must use schema version `velvet-ballistics/cli-output/v1`. Every binary envelope must use schema version `1_u16`.

### P3. Stable kind selection

Every envelope constructor must receive or derive exactly one `CliOutputKind` variant. Free-form kind strings are forbidden after validation.

### P4. Typed payload source

Payloads must be constructed from command-specific typed data or explicit typed adapters. New schema code must not assemble canonical payloads by free-form string formatting.

### P5. Exit code source

Any status or failure metadata must use `CliExitCode` variants and discriminants from `exit_code.rs`.

### P6. Bounded binary input

Any binary envelope decoding contract must receive a caller-provided maximum payload length before allocation or Postcard decoding.

### P7. Redaction before serialization

Any field with secret or uncertain taint must be redacted or summarized before it enters a YAML or Postcard envelope.

## 7. Postconditions

### Q1. Text envelope shape

Every successful YAML structured output is representable as:

- `schema_version: velvet-ballistics/cli-output/v1`
- `kind: <stable CliOutputKind name>`
- `command: <canonical command name>`
- `exit_code: <CliExitCode discriminant>`
- `data: <typed payload>`
- `diagnostics_summary: <bounded list or empty>`

### Q2. Diagnostic envelope shape

Every structured diagnostic is representable as:

- `schema_version: velvet-ballistics/cli-output/v1`
- `kind: DiagnosticReport`
- `command: <canonical command name if known>`
- `exit_code: <CliExitCode discriminant>`
- `diagnostics: <bounded list of typed DiagnosticEntry>`

### Q3. Binary envelope validation shape

Every Postcard CLI envelope has a fixed-size header conceptually equivalent to storage discipline:

- `magic_u32`
- `schema_version_u16`
- `kind_u16`
- `header_len_u32`
- `payload_len_u32`
- `payload_digest_blake3_256`
- `header_crc32c`
- `postcard payload bytes`

### Q4. Data-only stdout

Machine stdout contains only successful data envelopes or success report payloads. Full diagnostics are emitted to stderr as diagnostic envelopes or human text, never mixed into stdout data streams.

### Q5. Diagnostics-only stderr

Machine stderr contains only diagnostics. It must not carry successful data payloads.

### Q6. Deterministic field names

Schema field names are snake_case, stable, ASCII, and independent of Rust debug formatting.

### Q7. No ANSI in machine output

YAML/Postcard output and diagnostic envelopes contain no ANSI escape sequences.

### Q8. No implicit migration

Unknown, older, or newer binary schema versions never decode silently. They return typed migration or unsupported-version errors.

## 8. Invariants

### I1. Version invariant

`schema_version` is non-empty, stable, and equal to `velvet-ballistics/cli-output/v1` for text envelopes in v1.

### I2. Kind invariant

`kind` is a closed enum value and maps one-to-one to a stable numeric `kind_u16` for binary envelopes.

### I3. Command invariant

`command` is the canonical CLI command spelling, not an alias, debug string, or help sentence.

### I4. Exit invariant

`exit_code` equals the process exit code that would be returned for the same outcome.

### I5. Payload invariant

`data` contains exactly the typed payload for `kind`; `DiagnosticReport` uses `diagnostics` instead of `data`.

### I6. Bounded-resource invariant

All vectors, strings, diagnostics, and binary payloads have declared maximums. Decoders validate lengths before allocation.

### I7. Redaction invariant

Secret-tainted fields cannot appear unredacted in any envelope, diagnostic, repair hint, side-effect summary, or AI context packet.

### I8. Cold-path invariant

No schema dependency requires YAML, JSON, HTTP, or text routing in runtime core crates.

### I9. Determinism invariant

For identical inputs and no explicit timestamp option, envelopes serialize to semantically identical content with stable field ordering requirements for tests.

### I10. Railway invariant

All fallible schema construction, validation, encoding, and decoding operations return `Result<T, CliEnvelopeError>`.

## 9. Canonical Kind Vocabulary

The schema contract must define a closed `CliOutputKind` vocabulary with these v1 variants and stable binary IDs:

| Kind | Binary ID | Primary command/source |
| --- | ---: | --- |
| `VerificationReport` | 1 | `verify`, `commands_verify::VerifyOk`/`VerifyError` |
| `DiagnosticReport` | 2 | stderr diagnostics, parse/storage/runtime/report failures |
| `WorkflowExplanation` | 3 | `explain` |
| `WorkflowGraph` | 4 | `graph`, `commands_workflow::DotGraph` |
| `SimulationReport` | 5 | `simulate`, `commands_workflow::SimulationResult` |
| `SubmitRunResult` | 6 | `submit` |
| `RunInspection` | 7 | `inspect` |
| `RunEvents` | 8 | `events` |
| `ReplayReport` | 9 | `replay` |
| `TraceReport` | 10 | `trace`, `commands_journal::TraceEntry` |
| `RetryReport` | 11 | `retry`, `commands_journal::RetryAnalysis` |
| `ResumeReport` | 12 | `resume` |
| `AnswerReport` | 13 | `answer` |
| `IncidentReport` | 14 | `incident`, `commands_incident::IncidentReport` |
| `ActionList` | 15 | `action list` |
| `ActionDescription` | 16 | `action inspect` |
| `DoctorReport` | 17 | `doctor` |
| `AiContextPacket` | 18 | `ai-context`, `commands_ai_context` |
| `StatusReport` | 19 | `status`, `commands_status::CliStatus` |
| `WorkflowDiff` | 20 | `diff`, `commands_diff::DiffResult` |
| `CompileReport` | 21 | `compile` metadata only, not the compiled artifact bytes |
| `RunResult` | 22 | `run`, `run-compiled` |
| `BenchRunReport` | 23 | `bench-run` |
| `AgentContext` | 24 | `agent-context`, after compatibility migration |

No implementation may assign meaning to IDs outside this table without a versioned contract update.

## 10. Contract Signatures

These are contract-level signatures, not implementation instructions:

- `fn cli_schema_version() -> &'static str`
- `fn binary_cli_schema_version() -> u16`
- `fn kind_name(kind: CliOutputKind) -> &'static str`
- `fn kind_id(kind: CliOutputKind) -> u16`
- `fn kind_from_id(id: u16) -> Result<CliOutputKind, CliEnvelopeError>`
- `fn command_for_kind(kind: CliOutputKind) -> Result<CliCommandName, CliEnvelopeError>`
- `fn build_text_envelope<T>(kind: CliOutputKind, command: CliCommandName, exit_code: CliExitCode, data: T, diagnostics_summary: BoundedDiagnosticsSummary) -> Result<CliTextEnvelope<T>, CliEnvelopeError>`
- `fn build_diagnostic_report(command: Option<CliCommandName>, exit_code: CliExitCode, diagnostics: BoundedDiagnostics) -> Result<CliDiagnosticEnvelope, CliEnvelopeError>`
- `fn encode_postcard_envelope<T: serde::Serialize>(kind: CliOutputKind, payload: &T, max_payload_len: u32) -> Result<Vec<u8>, CliEnvelopeError>`
- `fn decode_postcard_envelope<T: serde::de::DeserializeOwned>(bytes: &[u8], expected_kind: CliOutputKind, max_payload_len: u32) -> Result<CliBinaryEnvelope<T>, CliEnvelopeError>`
- `fn validate_text_envelope<T>(envelope: &CliTextEnvelope<T>) -> Result<(), CliEnvelopeError>`
- `fn validate_diagnostic_entry(entry: &DiagnosticEntry) -> Result<(), CliEnvelopeError>`

## 11. Typed Error Taxonomy

The schema contract must define an exhaustive `CliEnvelopeError` taxonomy:

- `EmptySchemaVersion` - text schema version is empty.
- `UnsupportedTextSchemaVersion { found: String }` - text version is not supported.
- `MigrationRequired { from: u16, to: u16 }` - binary version is older than current and requires named migration.
- `UnsupportedBinarySchemaVersion { version: u16 }` - binary version is newer or not supported.
- `UnknownKindName { kind: String }` - text kind is not in the closed vocabulary.
- `UnknownKindId { kind: u16 }` - binary kind ID is not in the closed vocabulary.
- `KindCommandMismatch { kind: CliOutputKind, command: CliCommandName }` - envelope kind cannot be produced by the command.
- `KindPayloadMismatch { kind: CliOutputKind }` - payload type does not match the kind contract.
- `BadMagic { found: u32 }` - binary envelope magic is wrong.
- `HeaderLengthMismatch { found: u32 }` - binary header length is not the fixed v1 header length.
- `PayloadTooLarge { len: u32, max: u32 }` - payload exceeds caller-provided maximum.
- `LengthOverflow` - host integer conversion or header plus payload size calculation would overflow.
- `HeaderChecksumMismatch` - CRC32C validation failed.
- `PayloadDigestMismatch` - BLAKE3 payload digest validation failed.
- `UnexpectedEof` - envelope bytes are shorter than declared header or payload.
- `PostcardEncodeFailed` - payload could not be encoded.
- `PostcardDecodeFailed` - payload could not be decoded after header and digest validation.
- `DiagnosticLimitExceeded { len: usize, max: usize }` - diagnostic entry count exceeds the declared maximum.
- `MessageTooLong { len: usize, max: usize }` - diagnostic or string field exceeds the declared maximum.
- `AnsiForbidden` - machine output field contains ANSI escapes.
- `UnredactedTaint { field: &'static str }` - tainted field reaches an envelope without redaction proof.
- `InvalidExitCode { code: u8 }` - status value is outside `CliExitCode` vocabulary.
- `RuntimeCoreBoundaryViolation { crate_name: &'static str }` - schema dependency is placed in a forbidden runtime crate.

Every error path must be testable without panics and must map to a `CliExitCode` for CLI emission.

## 12. Payload Boundary Contracts

### VerificationReport

- Source: `commands_verify::VerifyOk` and `VerifyError` adapters.
- Must include digest hex, profile, passed checks, warnings, and failure details as diagnostics.
- Must not encode compiler debug strings as primary structured fields; debug text may appear only as redacted diagnostic messages.

### StatusReport

- Source: `commands_status::CliStatus`.
- Must include health, running, shutting_down, queue depth/capacity, active/max runs, trace capacity/dropped, step budget, and runtime policy.
- Numeric overlays are reported as provided; no silent clamping unless a later validation contract adds bounds.

### AiContextPacket

- Source: `commands_ai_context` packet builder.
- Must carry schema version and kind under the new envelope rather than internal JSON-only version `1` after migration.
- Must redact workflow/source/action context according to taint rules.
- Suggested commands must use canonical `--emit yaml`/`--emit postcard` vocabulary after migration.

### WorkflowDiff

- Source: `commands_diff::DiffResult`.
- Must represent diff entries with typed variants rather than raw unbounded dynamic values.
- Event summaries must use stable event type names and numeric IDs where available.

### IncidentReport

- Source: `commands_incident::IncidentReport`.
- Must preserve failure code, failure_found, failed_at_step, side effects, and repair hints.
- Repair hints must be bounded strings and must not leak secrets.

### TraceReport, RetryReport, ResumeReport, AnswerReport, RunInspection, RunEvents, ReplayReport

- Source: `commands_journal` and durable journal readers.
- Must preserve event order and sequence numbers.
- Must reject or summarize output that exceeds declared maximum event count or byte size.

### WorkflowGraph and SimulationReport

- Source: `commands_workflow::DotGraph` and `SimulationResult`.
- Must include counts and structured step/edge data.
- DOT text, if included, is payload data only and must be escaped, bounded, and free of ANSI.

## 13. Acceptance Criteria

- A downstream implementation can define typed CLI envelope structs/enums without touching runtime core crates.
- The exact text schema version and binary schema version are specified.
- The closed kind vocabulary and binary IDs are specified.
- Every fallible operation has a `Result<T, CliEnvelopeError>` contract.
- All structured output invariants include stdout/stderr separation, no ANSI, bounded resources, redaction, deterministic fields, and stable exit codes.
- Binary validation order follows storage precedent: header length first, magic/version/kind/family, payload length before allocation, CRC, exact payload slice, BLAKE3 digest, then Postcard decode.
- The contract explicitly states that existing emitters are not flipped in this bead.
- The contract names the tests/scenarios needed for later State 4+ work without implementing tests here.

## 14. Martin Fowler Given/When/Then Scenarios

### Scenario 1: YAML envelope for successful verification

Given a valid workflow and a successful verification result
When the result is wrapped as `VerificationReport` for `verify --emit yaml`
Then the envelope has schema version `velvet-ballistics/cli-output/v1`
And kind `VerificationReport`
And command `verify`
And exit code `0`
And stdout contains only envelope data
And stderr contains no successful payload data.

### Scenario 2: Diagnostic report for validation failure

Given a workflow that fails validation
When a structured diagnostic is built
Then the diagnostic envelope has kind `DiagnosticReport`
And exit code `ValidationFailed` / `1`
And each diagnostic has code, severity, message, optional path/span, taint, and remediation fields
And no unredacted tainted field is present.

### Scenario 3: Postcard decode rejects oversized payload before allocation

Given binary envelope bytes declaring `payload_len_u32` greater than `max_payload_len`
When the decoder validates the header
Then it returns `PayloadTooLarge { len, max }`
And it does not allocate payload storage
And it does not attempt Postcard decode.

### Scenario 4: Unknown binary kind is rejected

Given a binary envelope with an unassigned `kind_u16`
When the decoder validates the kind
Then it returns `UnknownKindId`
And no payload bytes are decoded.

### Scenario 5: Older binary schema requires migration

Given a binary envelope with schema version less than `1_u16`
When the decoder validates the version
Then it returns `MigrationRequired`
And it does not implicitly upgrade the payload.

### Scenario 6: Newer binary schema is unsupported

Given a binary envelope with schema version greater than `1_u16`
When the decoder validates the version
Then it returns `UnsupportedBinarySchemaVersion`
And it does not decode the payload.

### Scenario 7: Diagnostic stream separation

Given a command that produces both data and warnings
When machine output is requested
Then stdout carries the data envelope only
And stderr carries full diagnostics only
And the stdout envelope carries at most a bounded diagnostic summary.

### Scenario 8: Agent context migration contract

Given current `agent-context` advertises `--json` and `--jsonl`
When the schema migration is implemented
Then canonical structured output vocabulary changes to `--emit yaml` and `--emit postcard`
And JSON is identified only as temporary compatibility or removed by a downstream bead.

### Scenario 9: Runtime core boundary is protected

Given a proposed schema type added to `vb_runtime`
When the boundary check is run
Then it fails with `RuntimeCoreBoundaryViolation`
And the type must move to cold CLI code.

### Scenario 10: Tainted incident report is redacted

Given an incident report with a side-effect detail marked secret-tainted
When the report is enveloped
Then the emitted side effect is redacted or summarized
And an unredacted secret value causes `UnredactedTaint`.

## 15. Proof Obligations

- Prove by compile-time module boundaries or source checks that envelope schemas do not enter runtime core crates.
- Prove every kind maps to exactly one binary ID and every binary ID maps back to exactly one kind.
- Prove every command that emits structured output maps to one allowed kind.
- Prove every `CliEnvelopeError` variant has a corresponding contract verification scenario.
- Prove binary decode validates payload length before allocation and before Postcard decode.
- Prove BLAKE3 payload digest and CRC32C header checksum failures are distinct.
- Prove `CliExitCode` envelope values match process exit status values.
- Prove ANSI escape sequences are rejected for machine output.
- Prove tainted fields cannot cross the serialization boundary without redaction evidence.
- Prove Moon CI remains the canonical gate for downstream implementation.

## 16. Out-of-Scope Boundaries

- Do not implement envelope structs, encoders, decoders, parsers, or tests in this State 3 artifact.
- Do not change existing `--json`/`--jsonl` behavior in this bead.
- Do not rewrite `main.rs` emitters in this bead.
- Do not change command parsing semantics in `args.rs` in this bead.
- Do not add YAML, JSON, HTTP, or CLI schema dependencies to runtime core crates.
- Do not modify storage record formats; storage envelopes are a design precedent only.
- Do not make performance claims without measured benchmark evidence.

## 17. Risk Notes

- Current CLI code is JSON/JSONL-centric, so schema adoption will require staged parser, help-text, agent-context, and integration-test migration.
- `--emit` currently denotes compile artifact target; reporting output emission needs a separate type split to avoid parser regressions.
- Current command modules still use `serde_json::Value` for some report internals; downstream work must replace or wrap these with typed payloads before claiming full schema compliance.
- `commands_workflow.rs` currently contains unchecked-convenience patterns such as fallback conversion behavior; downstream implementation must preserve repo constraints and avoid new unwrap/expect/panic/todo/unimplemented/dbg/unsafe.
- Including timestamps in envelopes can break deterministic tests; default envelopes should avoid generated time unless explicitly requested.
- Diagnostic duplication between stdout summaries and stderr full reports can confuse agents unless the separation rule is enforced exactly.

## 18. Exit Criteria For This Bead

- `.beads/vb-qi37.13.1/contract.md` exists.
- The artifact is non-empty.
- The artifact contains scope, invariants, preconditions, postconditions, typed error taxonomy, acceptance criteria, Fowler Given/When/Then scenarios, proof obligations, out-of-scope boundaries, and risk notes.
