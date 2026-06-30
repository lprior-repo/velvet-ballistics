# State 2 Codebase Map: vb-qi37.13.1

Bead: `vb-qi37.13.1`
Title: `cli: Define structured envelope schemas`
Scope: schema definition/map only. Do not flip existing emitters in this bead.

## Relevant Files

- `crates/velvet_ballistics/src/args.rs`: current CLI argument model. `OutputFormat` is `Text | Json | Jsonl`, while `EmitTarget` already includes `Yaml | Postcard` for compile artifacts. Most command variants carry `output: OutputFormat`.
- `crates/velvet_ballistics/src/main.rs`: central command dispatch and current output helpers. Help text advertises `--json|--jsonl`; many commands emit `serde_json::json!` values through `json_out`/`json_error`. This is the main future touchpoint for envelope adoption but should not be changed in this State 2 artifact.
- `crates/velvet_ballistics/src/agent_context.rs`: current machine-readable CLI schema is JSON-shaped and includes `schema_version`, `kind`, stdout/stderr policy, command metadata, and output flag vocabulary. This is the closest existing structured-output schema source, but its contract currently says `--json`/`--jsonl`, which conflicts with the v1 master contract.
- `crates/velvet_ballistics/src/exit_code.rs`: maps domain exit outcomes to process codes. Downstream envelope schemas should reference this as the source of `exit_code`/status semantics rather than inventing new numeric meanings.
- `crates/velvet_ballistics/src/commands_verify.rs`: verification report inputs and diagnostics surface. Master examples name `VerificationReport` and `DiagnosticReport`; contract work should inspect this module before defining those payload variants.
- `crates/velvet_ballistics/src/commands_status.rs`: runtime status report builder/formatter. Likely payload source for `StatusReport` or similar reporting envelope.
- `crates/velvet_ballistics/src/commands_ai_context.rs`: AI context packet builder and redaction helpers. Master examples name `AiContextPacket`; this module should provide the payload boundary.
- `crates/velvet_ballistics/src/commands_diff.rs`: diff report formatting. Master examples include `WorkflowDiff`; schema contract should align kind names and fields here.
- `crates/velvet_ballistics/src/commands_incident.rs`: incident report formatting and remediation suggestions. Master examples name `IncidentReport`; preserve taint/redaction expectations.
- `crates/velvet_ballistics/src/commands_journal.rs`: durable event/inspection/replay/trace formatting helpers. Likely source for `RunInspection`, `RunEvents`, `ReplayReport`, and trace/event payloads.
- `crates/velvet_ballistics/src/commands_workflow.rs`: workflow explanation, graph, simulation, and compile/reporting helpers. Master examples name `WorkflowExplanation`, `WorkflowGraph`, and `SimulationReport`.
- `crates/vb_storage/src/types.rs`: storage envelope precedent: `RecordEnvelope { magic, schema_version, record_kind, sequence }` and `RecordHeader` fields.
- `crates/vb_storage/src/codec.rs`: binary envelope precedent: encode/decode validates magic, schema version, record kind, header length, payload length, CRC32C, BLAKE3 digest, and Postcard payload before allocation/decoding.
- `velvet-ballistics-MASTER.md`: authoritative CLI contract. Important lines found: `--emit yaml` and `--emit postcard` are canonical v1 structured flags; JSON is not canonical for v1; every structured output has `schema_version` and `kind`; stdout is data only and stderr is diagnostics only.
- `scripts/check-agent-cli-contract.sh`: current source-contract guard. It still requires `"--json"` in CLI source and rejects certain non-agent-first vocabulary. This will need downstream updates when schemas/flags move to YAML/Postcard.

## Patterns To Reuse

- Reuse storage envelope discipline from `vb_storage`: explicit schema version, kind discriminator, bounded payload length, deterministic binary encoding, digest/checksum where bytes cross a durable or binary boundary.
- Reuse CLI `agent_context` shape for top-level metadata only: `schema_version`, `kind`, CLI name, language version, stdout/stderr policy, command vocabulary, and exit-code map.
- Reuse `CliExitCode` as the canonical process-code vocabulary in schemas instead of ad hoc success/error strings.
- Reuse command-specific builder modules as typed payload sources; avoid schema fields assembled by string formatting in `main.rs` where possible.
- Keep runtime core clean: schemas are cold CLI artifacts and must not enter `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`, or generated workflow code except when using the existing storage envelope as a design reference.

## Suspected Touchpoints For Later States

- Add a CLI schema module under `crates/velvet_ballistics/src/`, likely `cli_envelope.rs` or `output_envelope.rs`, containing typed envelope structs/enums and kind constants.
- Extend `args.rs` so reporting commands can express canonical `--emit yaml|postcard` output selection without confusing it with compile artifact `EmitTarget`.
- Update `main.rs` output helpers to serialize typed envelopes to YAML/Postcard later; do not do this in the schema-definition bead unless the contract explicitly expands scope.
- Update `agent_context.rs` so the advertised agent contract says canonical structured output is `--emit yaml`, binary machine output is `--emit postcard`, and JSON/JSONL are legacy or future cold adapters only if retained.
- Update `scripts/check-agent-cli-contract.sh` after the contract is approved so it requires `--emit yaml`, `--emit postcard`, `schema_version`, `kind`, stdout/stderr separation, and no `--json` canonical vocabulary.
- Consider separating compile artifact emission (`compile --emit ir|rust|yaml|postcard --out`) from reporting-command output emission (`reporting command --emit yaml|postcard`) to avoid enum naming collisions.

## Candidate Envelope Schema Families

- Text/YAML envelope: `{ schema_version: "velvet-ballistics/cli-output/v1", kind: <Kind>, command: <canonical command>, generated_at_ms?: u64, data: <typed payload>, diagnostics: [] }`.
- Diagnostic envelope: `{ schema_version, kind: "DiagnosticReport", diagnostics: [{ code, severity, message, path?, span?, taint?, remediation? }], exit_code }`.
- Binary/Postcard envelope: reuse storage-style header concepts for CLI machine output where supported: magic, schema_version_u16, kind_u16, header_len, payload_len, payload_digest_blake3_256, header_crc32c, then postcard payload.
- Kind vocabulary should be explicit and stable. Master examples include `VerificationReport`, `DiagnosticReport`, `WorkflowExplanation`, `WorkflowGraph`, `SimulationReport`, `SubmitRunResult`, `RunInspection`, `RunEvents`, `ReplayReport`, `IncidentReport`, `ActionList`, `ActionDescription`, `DoctorReport`, and `AiContextPacket`.

## Test Locations

- `crates/velvet_ballistics/tests/cli_integration.rs`: primary end-to-end CLI output assertions. Currently tests `--json`/`--jsonl`, stdout/stderr separation, action list/inspect JSON, and AI context JSON parsing.
- `crates/velvet_ballistics/src/main_tests.rs`: parser and command behavior unit tests. Current tests assert `OutputFormat::Json` from `--json` and include suggested AI command strings with `--json`.
- `scripts/check-agent-cli-contract.sh`: shell-level contract gate for source literals and forbidden vocabulary.
- Storage precedent tests may live around `crates/vb_storage/src/codec.rs` unit tests; contract should ask implementers to mirror envelope validation style for CLI Postcard output, not import storage record kinds directly.

## Risks And Dependencies

- Current CLI and tests are JSON/JSONL-centric while the master contract requires YAML/Postcard. This bead should define target schemas and migration touchpoints, not convert all emitters.
- `--emit` currently means compile target in `compile`; using the same flag for reporting output needs a careful type split to avoid parsing regressions.
- JSON may remain in code as a cold adapter only if downstream contract explicitly allows it; v1 canonical language must be YAML/Postcard.
- Diagnostics must preserve stdout/stderr separation and must not leak secret-tainted failure details. Any schema must carry redaction/taint fields or define how redaction is proven.
- Postcard output must be bounded before allocation/decoding. If a CLI binary envelope is introduced, copy the storage decode order principles rather than building unbounded buffers.
- Do not move schema types into runtime crates. CLI output schemas belong to `crates/velvet_ballistics` or a cold CLI-support crate only.

## Next-State Notes For rust-contract

- Define invariants for all structured CLI outputs: non-empty `schema_version`, stable `kind`, data-only stdout, diagnostics-only stderr, deterministic field names, bounded payloads, and no ANSI in machine output.
- Decide whether YAML envelopes include `diagnostics` inline, stderr-only diagnostics, or both with a strict rule. The master requires diagnostics and stdout/stderr separation, so the contract must remove ambiguity.
- Specify exact schema version string and numeric binary schema version. Suggested text schema version from master examples: `velvet-ballistics/cli-output/v1`.
- Specify the canonical kind enum and map each reporting command to one kind.
- Specify whether `agent-context` itself remains JSON temporarily or becomes an envelope kind. If temporary, write a compatibility exception with an expiry/downstream bead.
- Specify no production/test implementation in this bead unless later state explicitly changes scope; downstream beads should cover diagnostics/exit code schemas and emitter conversion.
