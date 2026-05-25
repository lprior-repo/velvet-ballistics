# Test Plan: vb-qi37.13.1 — cli: Define structured envelope schemas

## Summary

This repaired plan explicitly addresses every finding in `test-plan-review.md`: it raises unit-test density above the required 60 tests for 12 contract signatures, replaces placeholder assertions with concrete typed error payloads, fixes maximum constants, expands all 24 command/kind and payload-family scenarios, specifies exact stdout/stderr oracles, adds min/max binary boundaries, defines mutation targets for deletion/swap/default-return mutants, and names fuzz/Kani/static gates plus filesystem side-effect setup and cleanup.

- Behaviors identified: 46
- Contract signatures: 12
- Required unit density: `12 * 5 = 60`; planned unit tests: 84 minimum
- Trophy allocation target: 84 unit / 30 integration / 2 e2e / 8 static gates
- Proptest invariants: 12
- Fuzz targets: 4
- Kani harnesses: 7
- Mutation threshold: `cargo-mutants` must kill at least 90% of mutants in the schema constants, kind mapping, command mapping, text validation, diagnostic validation, and binary envelope modules.
- Assertion rule: no planned assertion may be only `is_ok()` or `is_err()`; every assertion must name exact values, exact bytes, exact parsed YAML fields, or exact `CliEnvelopeError` variant payload.

## Fixed Test Constants For This Bead

Downstream implementation must expose or share these constants with tests; changing a value requires a contract update and a test-plan update.

| Constant | Exact value | Use |
|---|---:|---|
| `CLI_TEXT_SCHEMA_VERSION` | `"velvet-ballistics/cli-output/v1"` | YAML/text envelope schema |
| `CLI_BINARY_SCHEMA_VERSION` | `1_u16` | Postcard envelope schema |
| `CLI_BINARY_MAGIC` | `0x5642_434C_u32` (`VBCL`) | binary header magic |
| `CLI_BINARY_HEADER_LEN` | `52_u32` | 4 + 2 + 2 + 4 + 4 + 32 + 4 |
| `MAX_CLI_PAYLOAD_BYTES` | `1_048_576_u32` | maximum Postcard payload |
| `MAX_DIAGNOSTICS` | `64_usize` | diagnostic entries per envelope |
| `MAX_DIAGNOSTIC_MESSAGE_BYTES` | `4_096_usize` | diagnostic message bytes |
| `MAX_CLI_STRING_BYTES` | `4_096_usize` | bounded CLI strings |
| `MAX_EVENT_COUNT` | `10_000_usize` | journal/event payload summaries |
| `MAX_REPAIR_HINT_BYTES` | `1_024_usize` | incident repair hints |
| guaranteed invalid `CliExitCode` byte | `9_u8` | current valid values are `0..=8` |

## 1. Behavior Inventory

1. `cli_schema_version` returns exactly `velvet-ballistics/cli-output/v1` when queried.
2. `binary_cli_schema_version` returns exactly `1_u16` when queried.
3. `kind_name` returns the exact stable PascalCase name for each of the 24 v1 kinds.
4. `kind_id` returns the exact stable ID `1..=24` for each v1 kind.
5. `kind_from_id` returns the exact kind for every ID `1..=24`.
6. `kind_from_id` returns `UnknownKindId { kind: 0 }` when ID is zero.
7. `kind_from_id` returns `UnknownKindId { kind: 25 }` when ID is first unassigned.
8. `kind_from_id` returns `UnknownKindId { kind: 65535 }` when ID is `u16::MAX`.
9. Text kind parsing returns `UnknownKindName { kind: "MadeUpReport" }` when the kind name is not closed vocabulary.
10. `command_for_kind` returns the canonical command spelling for every command-producing kind.
11. Command/kind validation rejects every disallowed pair class with `KindCommandMismatch { kind, command }`.
12. `build_text_envelope` constructs deterministic YAML-compatible envelopes for every payload family.
13. `build_text_envelope` preserves argument positions when kind, command, exit code, data, and diagnostics summary are supplied.
14. `build_text_envelope` rejects wrong command provenance with exact `KindCommandMismatch`.
15. `build_text_envelope` rejects unredacted secret taint with exact field name.
16. `build_diagnostic_report` constructs `DiagnosticReport` envelopes with exact diagnostic fields.
17. `build_diagnostic_report` accepts exactly 64 diagnostics and preserves all entries.
18. `build_diagnostic_report` rejects 65 diagnostics with `DiagnosticLimitExceeded { len: 65, max: 64 }`.
19. `validate_diagnostic_entry` accepts a 4096-byte message with no ANSI and valid taint.
20. `validate_diagnostic_entry` rejects a 4097-byte message with `MessageTooLong { len: 4097, max: 4096 }`.
21. `validate_diagnostic_entry` rejects ANSI SGR, bare ESC, unterminated CSI, and nested ANSI with `AnsiForbidden`.
22. `validate_text_envelope` rejects empty schema version with `EmptySchemaVersion`.
23. `validate_text_envelope` rejects v2 text schema with `UnsupportedTextSchemaVersion { found: "velvet-ballistics/cli-output/v2" }`.
24. `validate_text_envelope` rejects kind/payload mismatch with `KindPayloadMismatch { kind }`.
25. `validate_text_envelope` rejects numeric exit code `9` with `InvalidExitCode { code: 9 }`.
26. `validate_text_envelope` rejects ANSI in schema, kind, command, diagnostic, payload string, or repair hint with `AnsiForbidden`.
27. YAML machine stdout contains one exact data document and no full diagnostic document when warnings exist.
28. YAML machine stderr contains one exact diagnostic document and no success payload when warnings exist.
29. Empty stdout, empty stderr when diagnostics are required, and malformed YAML fail integration oracles.
30. `encode_postcard_envelope` encodes every kind ID with matching payload family, magic `0x5642434C`, version `1`, header length `52`, exact payload length, CRC32C, BLAKE3 digest, and Postcard payload.
31. `encode_postcard_envelope` accepts payload length exactly `1_048_576` bytes.
32. `encode_postcard_envelope` rejects payload length `1_048_577` bytes with `PayloadTooLarge { len: 1_048_577, max: 1_048_576 }`.
33. `encode_postcard_envelope` returns `PostcardEncodeFailed` when a public test-only serializable payload returns a serde serialization error.
34. `decode_postcard_envelope` rejects empty bytes with `UnexpectedEof`.
35. `decode_postcard_envelope` rejects a 51-byte header with `UnexpectedEof` before reading payload fields.
36. `decode_postcard_envelope` rejects header length `51` with `HeaderLengthMismatch { found: 51 }`.
37. `decode_postcard_envelope` rejects magic `0x0000_0000` with `BadMagic { found: 0 }`.
38. `decode_postcard_envelope` rejects binary version `0` with `MigrationRequired { from: 0, to: 1 }`.
39. `decode_postcard_envelope` rejects binary version `2` with `UnsupportedBinarySchemaVersion { version: 2 }`.
40. `decode_postcard_envelope` rejects binary kind ID `25` with `UnknownKindId { kind: 25 }`.
41. `decode_postcard_envelope` rejects payload length `1_048_577` before allocation with `PayloadTooLarge { len: 1_048_577, max: 1_048_576 }`.
42. `decode_postcard_envelope` rejects overflowing total lengths with `LengthOverflow`.
43. `decode_postcard_envelope` rejects corrupt header CRC with `HeaderChecksumMismatch`.
44. `decode_postcard_envelope` rejects truncated payload bytes with `UnexpectedEof`.
45. `decode_postcard_envelope` rejects BLAKE3 mismatch with `PayloadDigestMismatch`.
46. `decode_postcard_envelope` rejects valid-header/wrong-type payload bytes with `PostcardDecodeFailed`.

## 2. Trophy Allocation

| Behavior(s) | Layer | Tool | Rationale |
|---|---|---|---|
| 1-11 | Unit + proptest | `#[test]`, `proptest` | Closed vocabularies and mappings are pure and require exhaustive exact-value proof. |
| 12-26 | Unit + integration | `#[test]`, `serde_yaml`, real typed fixtures | Constructors/validators are pure, but serialized YAML shape must be tested through public schema APIs. |
| 27-29 | Integration + E2E | `assert_cmd`/repo CLI harness, parsed YAML oracle | stdout/stderr separation is an observable process boundary, not private logic. |
| 30-46 | Unit + integration + fuzz + Kani | `#[test]`, `proptest`, `cargo fuzz`, `kani` | Binary validation combines pure arithmetic, real codecs, adversarial bytes, and formal resource bounds. |
| Runtime-core isolation | Static | `scripts/check-cli-schema-boundaries.sh` planned gate | Boundary is a dependency/source policy and must fail before runtime. |
| No forbidden constructs | Static | `moon ci`, source lint | Repo policy forbids unsafe/panic-family constructs. |

The plan is intentionally unit-dense to satisfy the review mandate while keeping integration tests as the largest behavior-confidence layer for emitted documents and binary codec round trips.

## 3. Required Unit-Test Density By Contract Signature

Minimum: 60 unit tests. Planned names below are mandatory; integration/proptest/fuzz tests are additional.

| Contract signature | Required unit test names |
|---|---|
| `cli_schema_version()` | `schema_constants_return_text_v1_when_queried`; `schema_version_is_non_empty_ascii_when_queried`; `schema_version_contains_no_ansi_when_queried`; `schema_version_uses_velvet_ballistics_spelling_when_queried`; `schema_version_rejects_mutated_literal_in_snapshot_gate` |
| `binary_cli_schema_version()` | `binary_schema_version_returns_one_when_queried`; `binary_schema_version_is_not_zero_when_queried`; `binary_schema_version_matches_decoder_current_version`; `binary_schema_version_matches_encoder_header`; `binary_schema_version_rejects_default_zero_mutant` |
| `kind_name(kind)` | `kind_name_returns_verification_report`; `kind_name_returns_diagnostic_report`; `kind_name_returns_agent_context`; `kind_names_are_unique_for_all_24_kinds`; `kind_name_contains_no_whitespace_or_ansi` |
| `kind_id(kind)` | `kind_id_returns_one_for_verification_report`; `kind_id_returns_two_for_diagnostic_report`; `kind_id_returns_twenty_four_for_agent_context`; `kind_ids_are_unique_for_all_24_kinds`; `kind_id_rejects_default_zero_mutant_by_table_snapshot` |
| `kind_from_id(id)` | `kind_from_id_returns_verification_report_for_one`; `kind_from_id_returns_status_report_for_nineteen`; `kind_from_id_returns_agent_context_for_twenty_four`; `kind_from_id_returns_unknown_kind_id_for_zero`; `kind_from_id_returns_unknown_kind_id_for_twenty_five`; `kind_from_id_returns_unknown_kind_id_for_65535` |
| `command_for_kind(kind)` | `command_for_kind_returns_verify_for_verification_report`; `command_for_kind_returns_status_for_status_report`; `command_for_kind_returns_ai_context_for_ai_context_packet`; `command_for_kind_returns_agent_context_for_agent_context`; `command_for_kind_table_matches_all_24_canonical_commands`; `command_for_kind_rejects_default_verify_mutant` |
| `build_text_envelope(...)` | `build_text_envelope_returns_exact_verification_report`; `build_text_envelope_returns_exact_status_report`; `build_text_envelope_returns_exact_workflow_diff`; `build_text_envelope_returns_exact_incident_report`; `build_text_envelope_returns_exact_ai_context_packet`; `build_text_envelope_returns_kind_command_mismatch_for_status_verify`; `build_text_envelope_returns_unredacted_taint_for_incident_side_effects`; `build_text_envelope_preserves_exit_code_argument`; `build_text_envelope_preserves_diagnostics_summary_argument`; `build_text_envelope_rejects_swapped_kind_command_arguments` |
| `build_diagnostic_report(...)` | `build_diagnostic_report_returns_exact_verify_validation_failure`; `build_diagnostic_report_accepts_zero_diagnostics`; `build_diagnostic_report_accepts_64_diagnostics`; `build_diagnostic_report_rejects_65_diagnostics`; `build_diagnostic_report_preserves_none_command`; `build_diagnostic_report_rejects_success_payload_default_mutant` |
| `encode_postcard_envelope(...)` | `encode_postcard_envelope_writes_exact_header_for_status`; `encode_postcard_envelope_writes_kind_id_for_each_24_kinds`; `encode_postcard_envelope_accepts_exact_max_payload`; `encode_postcard_envelope_rejects_payload_one_above_max`; `encode_postcard_envelope_returns_postcard_encode_failed_for_public_failing_serializer`; `encode_postcard_envelope_rejects_swapped_kind_payload_family` |
| `decode_postcard_envelope(...)` | `decode_postcard_envelope_returns_status_for_valid_status_bytes`; `decode_postcard_envelope_returns_unexpected_eof_for_empty_bytes`; `decode_postcard_envelope_returns_unexpected_eof_for_51_bytes`; `decode_postcard_envelope_returns_header_length_mismatch_for_51_declared_len`; `decode_postcard_envelope_returns_bad_magic_for_zero_magic`; `decode_postcard_envelope_returns_migration_required_for_version_zero`; `decode_postcard_envelope_returns_unsupported_binary_schema_version_for_version_two`; `decode_postcard_envelope_returns_unknown_kind_id_for_twenty_five`; `decode_postcard_envelope_returns_payload_too_large_for_1048577`; `decode_postcard_envelope_returns_length_overflow_for_u32_max_lengths`; `decode_postcard_envelope_returns_header_checksum_mismatch_for_crc_flip`; `decode_postcard_envelope_returns_unexpected_eof_for_truncated_payload`; `decode_postcard_envelope_returns_payload_digest_mismatch_for_payload_flip`; `decode_postcard_envelope_returns_postcard_decode_failed_for_wrong_type_payload`; `decode_postcard_envelope_rejects_ok_default_mutant` |
| `validate_text_envelope(...)` | `validate_text_envelope_accepts_exact_valid_status`; `validate_text_envelope_returns_empty_schema_version_for_empty_string`; `validate_text_envelope_returns_unsupported_text_schema_version_for_v2`; `validate_text_envelope_returns_unknown_kind_name_for_made_up_report`; `validate_text_envelope_returns_kind_payload_mismatch_for_incident_status_payload`; `validate_text_envelope_returns_invalid_exit_code_for_nine`; `validate_text_envelope_returns_ansi_forbidden_for_schema_field`; `validate_text_envelope_returns_ansi_forbidden_for_payload_field`; `validate_text_envelope_returns_unredacted_taint_for_secret_field`; `validate_text_envelope_rejects_ok_default_mutant` |
| `validate_diagnostic_entry(entry)` | `validate_diagnostic_entry_accepts_4096_byte_message`; `validate_diagnostic_entry_returns_message_too_long_for_4097_byte_message`; `validate_diagnostic_entry_returns_ansi_forbidden_for_sgr`; `validate_diagnostic_entry_returns_ansi_forbidden_for_bare_escape`; `validate_diagnostic_entry_returns_ansi_forbidden_for_unterminated_csi`; `validate_diagnostic_entry_returns_ansi_forbidden_for_nested_ansi`; `validate_diagnostic_entry_returns_unredacted_taint_for_secret_remediation`; `validate_diagnostic_entry_rejects_ok_default_mutant` |

## 4. BDD Scenarios

### Behavior: Schema constants are stable
Test functions: `schema_constants_return_text_v1_when_queried`, `binary_schema_version_returns_one_when_queried`

Given: the public schema constants API is available.
When: `cli_schema_version()` and `binary_cli_schema_version()` are called.
Then: the exact returned values are `"velvet-ballistics/cli-output/v1"` and `1_u16`.

### Behavior: Kind table is exact and exhaustive
Test function: `kind_table_returns_exact_names_and_ids_when_all_v1_kinds_are_queried`

Given: all v1 `CliOutputKind` variants.
When: `kind_name(kind)`, `kind_id(kind)`, and `kind_from_id(id)` are called.
Then: the exact mapping is `VerificationReport=1`, `DiagnosticReport=2`, `WorkflowExplanation=3`, `WorkflowGraph=4`, `SimulationReport=5`, `SubmitRunResult=6`, `RunInspection=7`, `RunEvents=8`, `ReplayReport=9`, `TraceReport=10`, `RetryReport=11`, `ResumeReport=12`, `AnswerReport=13`, `IncidentReport=14`, `ActionList=15`, `ActionDescription=16`, `DoctorReport=17`, `AiContextPacket=18`, `StatusReport=19`, `WorkflowDiff=20`, `CompileReport=21`, `RunResult=22`, `BenchRunReport=23`, `AgentContext=24`.

### Behavior: Unknown kind IDs and names are rejected
Test functions: `kind_from_id_returns_unknown_kind_id_for_zero`, `kind_from_id_returns_unknown_kind_id_for_twenty_five`, `kind_from_id_returns_unknown_kind_id_for_65535`, `validate_text_envelope_returns_unknown_kind_name_for_made_up_report`

Given: IDs `0`, `25`, and `65535`, and text kind `"MadeUpReport"`.
When: kind lookup or text-envelope validation runs.
Then: exact results are `Err(CliEnvelopeError::UnknownKindId { kind: 0 })`, `Err(CliEnvelopeError::UnknownKindId { kind: 25 })`, `Err(CliEnvelopeError::UnknownKindId { kind: 65535 })`, and `Err(CliEnvelopeError::UnknownKindName { kind: "MadeUpReport".to_owned() })`.

### Behavior: Command mapping covers all 24 kinds
Test function: `command_for_kind_table_matches_all_24_canonical_commands`

Given: every v1 kind.
When: `command_for_kind(kind)` is called.
Then: exact canonical mappings are `VerificationReport -> verify`, `DiagnosticReport -> diagnostic`, `WorkflowExplanation -> explain`, `WorkflowGraph -> graph`, `SimulationReport -> simulate`, `SubmitRunResult -> submit`, `RunInspection -> inspect`, `RunEvents -> events`, `ReplayReport -> replay`, `TraceReport -> trace`, `RetryReport -> retry`, `ResumeReport -> resume`, `AnswerReport -> answer`, `IncidentReport -> incident`, `ActionList -> action list`, `ActionDescription -> action inspect`, `DoctorReport -> doctor`, `AiContextPacket -> ai-context`, `StatusReport -> status`, `WorkflowDiff -> diff`, `CompileReport -> compile`, `RunResult -> run`, `BenchRunReport -> bench-run`, `AgentContext -> agent-context`.

### Behavior: Disallowed command/kind pairs fail
Test functions: `build_text_envelope_returns_kind_command_mismatch_for_status_verify`, `command_mapping_rejects_alias_debug_and_help_sentence_commands`

Given: `(StatusReport, verify)`, `(VerificationReport, status)`, `(ActionDescription, action list)`, alias `actions`, debug string `Command::Status`, and help sentence `show current status`.
When: envelope construction or command validation runs.
Then: exact pair-mismatch errors are `KindCommandMismatch { kind: StatusReport, command: Verify }`, `KindCommandMismatch { kind: VerificationReport, command: Status }`, and `KindCommandMismatch { kind: ActionDescription, command: ActionList }`.
And: non-canonical command strings are rejected by the public command/kind validation boundary with these exact errors and rejected strings:

| Rejected command string | Exact required error |
|---|---|
| `actions` | `Err(CliEnvelopeError::UnknownKindName { kind: "actions".to_owned() })` |
| `Command::Status` | `Err(CliEnvelopeError::UnknownKindName { kind: "Command::Status".to_owned() })` |
| `show current status` | `Err(CliEnvelopeError::UnknownKindName { kind: "show current status".to_owned() })` |

### Behavior: Valid text envelopes exist for every payload family
Test function: `build_text_envelope_returns_exact_document_for_each_payload_family`

Given: deterministic typed fixtures for all 24 payload families with no timestamp and empty diagnostics summary.
When: each fixture is wrapped by `build_text_envelope` and serialized to YAML.
Then: each YAML document has field order and values exactly `schema_version`, `kind`, `command`, `exit_code`, `data`, `diagnostics_summary`; `schema_version` is `velvet-ballistics/cli-output/v1`; `exit_code` is `0`; `kind` and `command` match the 24-row table above; `diagnostics_summary` is `[]`; `data` equals the fixture payload with snake_case field names.

### Behavior: Text envelope validators return exact typed errors
Test functions: `validate_text_envelope_returns_empty_schema_version_for_empty_string`, `validate_text_envelope_returns_unsupported_text_schema_version_for_v2`, `validate_text_envelope_returns_kind_payload_mismatch_for_incident_status_payload`, `validate_text_envelope_returns_invalid_exit_code_for_nine`, `validate_text_envelope_returns_unredacted_taint_for_secret_field`

Given: invalid text envelopes with schema `""`, schema `"velvet-ballistics/cli-output/v2"`, kind `IncidentReport` carrying a `StatusReport` payload, exit byte `9`, and secret field `side_effects` containing `"token=abc123"` without redaction proof.
When: `validate_text_envelope` runs.
Then: exact errors are `EmptySchemaVersion`, `UnsupportedTextSchemaVersion { found: "velvet-ballistics/cli-output/v2".to_owned() }`, `KindPayloadMismatch { kind: CliOutputKind::IncidentReport }`, `InvalidExitCode { code: 9 }`, and `UnredactedTaint { field: "side_effects" }`.

### Behavior: Diagnostic report construction and bounds are exact
Test functions: `build_diagnostic_report_returns_exact_verify_validation_failure`, `build_diagnostic_report_accepts_64_diagnostics`, `build_diagnostic_report_rejects_65_diagnostics`

Given: diagnostic entries with code `VB_VALIDATION_001`, severity `error`, message `workflow step has no action`, path `/steps/0`, span `{ start: 4, end: 12 }`, taint `Public`, remediation `add an action id`, command `verify`, and exit `CliExitCode::ValidationFailed`.
When: `build_diagnostic_report` is called with 1, 64, and 65 entries.
Then: 1 and 64 entries produce a `DiagnosticReport` with exit code `1` and all entries preserved in order; 65 entries returns `Err(CliEnvelopeError::DiagnosticLimitExceeded { len: 65, max: 64 })`.

### Behavior: Diagnostic entry string/ANSI/taint checks are exact
Test functions: `validate_diagnostic_entry_accepts_4096_byte_message`, `validate_diagnostic_entry_returns_message_too_long_for_4097_byte_message`, ANSI tests named in Section 3

Given: messages of exactly 4096 ASCII bytes, exactly 4097 ASCII bytes, `"\u{1b}[31mred\u{1b}[0m"`, `"\u{1b}"`, `"\u{1b}[31"`, `"x\u{1b}[31m\u{1b}[0my"`, and secret remediation `"password=hunter2"`.
When: `validate_diagnostic_entry` runs.
Then: 4096 bytes are accepted and preserved; 4097 bytes returns `MessageTooLong { len: 4097, max: 4096 }`; every ANSI case returns `AnsiForbidden`; secret remediation returns `UnredactedTaint { field: "remediation" }`.

### Behavior: stdout/stderr integration oracles are exact
Test functions: `cli_stdout_matches_exact_yaml_document_when_status_has_warning`, `cli_stderr_matches_exact_diagnostic_yaml_when_status_has_warning`, `cli_machine_output_fails_when_stdout_empty`, `cli_machine_output_fails_when_yaml_malformed`

Given: a deterministic `status --emit yaml` fixture with health `ok`, running `true`, shutting_down `false`, queue depth `2`, queue capacity `64`, active runs `1`, max runs `4`, trace capacity `1024`, trace dropped `0`, step budget `1000`, runtime policy `default`, and warning diagnostic `VB_STATUS_001` message `trace buffer near capacity`.
When: the machine-output integration harness captures stdout/stderr bytes.
Then: stdout bytes parse as exactly one YAML document with `kind: StatusReport`, `command: status`, `exit_code: 0`, the exact data values above, and `diagnostics_summary: [{ code: VB_STATUS_001, severity: warning }]`; stderr bytes parse as exactly one diagnostic YAML document with `kind: DiagnosticReport`, `command: status`, `exit_code: 0`, and full diagnostic message `trace buffer near capacity`; empty stdout, missing stderr diagnostic, extra second document, human log prefix, ANSI byte `0x1B`, or malformed YAML fails the oracle.

### Behavior: Binary encoding covers all 24 payload families
Test function: `encode_postcard_envelope_writes_kind_id_for_each_24_kinds`

Given: one minimal typed payload fixture for each v1 kind and `max_payload_len = 1_048_576`.
When: `encode_postcard_envelope(kind, &payload, 1_048_576)` is called.
Then: header bytes decode to magic `0x5642434C`, version `1`, the exact kind ID from the 24-row table, header length `52`, payload length equal to the Postcard payload byte count, CRC32C matching the protected header bytes, and BLAKE3 matching the exact payload slice.

### Behavior: Postcard encode failure is public-API testable
Test function: `encode_postcard_envelope_returns_postcard_encode_failed_for_public_failing_serializer`

Given: a test-only public fixture type `FailingSerializePayload` compiled only under `cfg(test)` that implements `serde::Serialize` by returning `serde::ser::Error::custom("intentional postcard failure")` from `serialize`.
When: `encode_postcard_envelope(CliOutputKind::StatusReport, &FailingSerializePayload, 1_048_576)` is called through the public API.
Then: the result is exactly `Err(CliEnvelopeError::PostcardEncodeFailed)`; the test does not call private helpers.

### Behavior: Binary decoder boundary errors are exact and ordered
Test functions: all `decode_postcard_envelope_returns_*` names in Section 3

Given: valid status envelope bytes, then one mutation at a time: empty bytes, 51 total bytes, declared header length 51, magic `0`, version `0`, version `2`, kind `25`, payload len `1_048_577`, declared header length `u32::MAX` plus payload `u32::MAX`, CRC flip, declared payload 1 byte longer than actual, payload-byte flip with unchanged digest, and valid digest for payload bytes of the wrong Rust type.
When: `decode_postcard_envelope::<StatusPayload>(&bytes, StatusReport, 1_048_576)` runs.
Then: exact errors are, respectively, `UnexpectedEof`, `UnexpectedEof`, `HeaderLengthMismatch { found: 51 }`, `BadMagic { found: 0 }`, `MigrationRequired { from: 0, to: 1 }`, `UnsupportedBinarySchemaVersion { version: 2 }`, `UnknownKindId { kind: 25 }`, `PayloadTooLarge { len: 1_048_577, max: 1_048_576 }`, `LengthOverflow`, `HeaderChecksumMismatch`, `UnexpectedEof`, `PayloadDigestMismatch`, and `PostcardDecodeFailed`.

Validation order required by every decoder test: (1) enough bytes for fixed header, (2) declared header length equals 52, (3) magic, (4) schema version, (5) kind ID, (6) expected kind/payload family, (7) payload length <= max before allocation, (8) checked total-length arithmetic, (9) CRC32C header checksum, (10) exact payload slice exists, (11) BLAKE3 payload digest, (12) Postcard decode.

### Behavior: Runtime-core boundary has isolated side effects
Test function: `boundary_gate_reports_runtime_core_boundary_violation_for_temp_vb_runtime_fixture`

Given: the test creates temporary directory `target/test-fixtures/vb-qi37-13-1/runtime-boundary`, writes a minimal crate file `crates/vb_runtime/src/cli_schema_boundary_violation.rs` inside that temp fixture containing `use velvet_ballistics::cli_schema::CliTextEnvelope;`, and does not touch real workspace source.
When: the public boundary validator is run against `target/test-fixtures/vb-qi37-13-1/runtime-boundary`.
Then: it returns exactly `Err(CliEnvelopeError::RuntimeCoreBoundaryViolation { crate_name: "vb_runtime" })`; cleanup removes `target/test-fixtures/vb-qi37-13-1/runtime-boundary` after the test.
And: the separate static-gate shell test for `scripts/check-cli-schema-boundaries.sh target/test-fixtures/vb-qi37-13-1/runtime-boundary` must exit nonzero and print `vb_runtime: CLI schema dependency forbidden`, but that shell-text assertion does not satisfy or replace the required `CliEnvelopeError::RuntimeCoreBoundaryViolation` scenario above.

## 5. Exhaustive Command/Kind And Payload Matrix

| Kind | ID | Command | Valid payload fixture | Required tests |
|---|---:|---|---|---|
| VerificationReport | 1 | `verify` | digest/profile/checks/warnings | construct YAML, encode Postcard, reject command `status` |
| DiagnosticReport | 2 | `diagnostic` | typed diagnostics | construct stderr YAML, encode Postcard, reject data field |
| WorkflowExplanation | 3 | `explain` | steps/explanation | construct YAML, encode Postcard, reject command `graph` |
| WorkflowGraph | 4 | `graph` | nodes/edges/DOT bounded | construct YAML, encode Postcard, reject ANSI DOT |
| SimulationReport | 5 | `simulate` | simulated steps/result | construct YAML, encode Postcard, reject event count 10001 |
| SubmitRunResult | 6 | `submit` | run id/durability | construct YAML, encode Postcard, reject command `run` |
| RunInspection | 7 | `inspect` | run state/current step | construct YAML, encode Postcard, reject command `events` |
| RunEvents | 8 | `events` | ordered sequence numbers | construct YAML, encode Postcard, reject 10001 events |
| ReplayReport | 9 | `replay` | divergence/result | construct YAML, encode Postcard, preserve sequence order |
| TraceReport | 10 | `trace` | trace entries | construct YAML, encode Postcard, reject 10001 traces |
| RetryReport | 11 | `retry` | retry analysis | construct YAML, encode Postcard, reject overlong reason 4097 |
| ResumeReport | 12 | `resume` | resumed run state | construct YAML, encode Postcard, reject command `run` |
| AnswerReport | 13 | `answer` | step/value summary | construct YAML, encode Postcard, redact value secrets |
| IncidentReport | 14 | `incident` | failure/side effects/hints | construct YAML, encode Postcard, reject unredacted side effects |
| ActionList | 15 | `action list` | action IDs/names | construct YAML, encode Postcard, reject command `action inspect` |
| ActionDescription | 16 | `action inspect` | action detail | construct YAML, encode Postcard, reject command `action list` |
| DoctorReport | 17 | `doctor` | health checks | construct YAML, encode Postcard, reject ANSI check text |
| AiContextPacket | 18 | `ai-context` | redacted AI context | construct YAML, encode Postcard, require `--emit yaml/postcard` suggestions |
| StatusReport | 19 | `status` | health/queues/runtime | construct YAML, encode Postcard, stdout/stderr integration |
| WorkflowDiff | 20 | `diff` | typed diff variants | construct YAML, encode Postcard, reject dynamic raw JSON value |
| CompileReport | 21 | `compile` | metadata only | construct YAML, encode Postcard, reject artifact bytes as data |
| RunResult | 22 | `run` | run outcome | construct YAML, encode Postcard, reject `run-compiled` mismatch unless explicitly aliased by contract update |
| BenchRunReport | 23 | `bench-run` | benchmark metadata only | construct YAML, encode Postcard, reject unmeasured speed claim |
| AgentContext | 24 | `agent-context` | migrated context | construct YAML, encode Postcard, mark JSON as compatibility only |

## 6. Proptest Invariants

1. `kind_from_id(kind_id(kind)) == kind` for every generated kind; invalid IDs outside `1..=24` return exact `UnknownKindId`.
2. Kind names are unique, ASCII, non-empty, ANSI-free, and equal to the 24-row snapshot.
3. Every allowed `(kind, command)` pair validates; every generated disallowed pair returns `KindCommandMismatch { kind, command }`.
4. Text envelopes without explicit timestamps serialize to semantically identical YAML for repeated calls.
5. Text envelope field names are always snake_case and never Rust debug strings.
6. Diagnostic vectors of length `0..=64` preserve all entries; length `65` returns `DiagnosticLimitExceeded { len: 65, max: 64 }`.
7. Strings of `0..=4096` bytes without ANSI validate unchanged; 4097 bytes returns `MessageTooLong { len: 4097, max: 4096 }`.
8. Secret or uncertain taint without redaction proof always returns `UnredactedTaint { field }`; public/redacted fields preserve exact redacted value.
9. Postcard encode/decode round trip preserves kind, version, payload, payload length, CRC, and digest for every payload family.
10. Payload length `1_048_576` succeeds; `1_048_577` returns exact `PayloadTooLarge` before allocation/decode.
11. Multiple binary mutations return the earliest ordered error from the 12-step order in Section 4.
12. Event-bearing payloads preserve order and sequence numbers for `0..=10_000` events and reject 10,001 events with exactly `MessageTooLong { len: 10001, max: 10000 }` under this bead's error taxonomy.

## 7. Fuzz Targets

Run scope: `cargo +nightly fuzz run <target> -- -max_total_time=60 -timeout=5 -rss_limit_mb=512` in CI smoke; nightly/manual campaign target is 1 hour per target. Crashing inputs must be committed under `fuzz/corpus/<target>/regression-*`.

### Fuzz Target: `decode_postcard_envelope`
Input type: arbitrary bytes plus expected kind chosen from IDs 1, 2, 19, 24 and max chosen from `0`, `1`, `52`, `1_048_576`.
Risk: panic, unchecked index, allocation bomb, validation-order bypass, CRC/digest confusion.
Corpus seeds: empty bytes; 51 bytes; valid minimal status; valid max payload header; bad magic 0; version 0; version 2; kind 25; payload 1_048_577; header len 51; CRC flip; payload flip; truncated payload; wrong-type valid digest payload.

### Fuzz Target: `validate_text_envelope_yaml_boundary`
Input type: arbitrary UTF-8/bytes parsed as YAML through public adapter.
Risk: panic, duplicate-key confusion, schema bypass, ANSI leakage, unbounded allocation.
Corpus seeds: valid status YAML; valid diagnostic YAML; empty document; duplicate `kind`; non-snake-case `schemaVersion`; schema `""`; v2 schema; `MadeUpReport`; exit `9`; ANSI in schema; ANSI in data; secret taint.

### Fuzz Target: `validate_diagnostic_entry`
Input type: arbitrary diagnostic entry struct from fuzz bytes.
Risk: overlong strings, invalid severity, invalid taint, ANSI escape variants, path/span edge cases.
Corpus seeds: 4096-byte message; 4097-byte message; SGR ANSI; bare ESC; unterminated CSI; nested ANSI; secret remediation; empty code.

### Fuzz Target: `cli_machine_output_splitter`
Input type: arbitrary stdout/stderr byte pairs for machine-output oracle.
Risk: accepting empty output, malformed YAML, stdout diagnostics, stderr success data, ANSI/human text leakage.
Corpus seeds: exact status stdout/stderr pair; empty stdout; empty stderr; malformed stdout YAML; two stdout docs; diagnostic in stdout; status payload in stderr; ANSI-prefixed text.

## 8. Kani Harnesses

1. `kani_kind_id_bijection`: prove 24 variants map to unique IDs and IDs 1..=24 map back. Bound: enum cardinality 24 plus IDs 0..=25.
2. `kani_command_kind_totality`: prove each kind has exactly one canonical command and no default `verify` return for non-verify kinds. Bound: 24 kinds.
3. `kani_header_length_arithmetic`: prove total length calculations either equal exact checked sum or return `LengthOverflow`. Bound: symbolic `u32` header/payload lengths.
4. `kani_payload_bound_precedes_allocation`: prove `payload_len > max` returns `PayloadTooLarge` before allocation/decode state. Bound: symbolic `u32` payload/max.
5. `kani_validation_order`: prove ordered phase enum returns first failing phase in exact 12-step order. Bound: 12 phases.
6. `kani_diagnostic_count_bound`: prove count `65` returns `DiagnosticLimitExceeded { len: 65, max: 64 }`, while `64` does not. Bound: counts 0..=65 plus `usize::MAX` model.
7. `kani_no_unchecked_index_header_parse`: prove parser never indexes beyond bytes length for lengths 0..=52. Bound: byte slice length 0..=52.

## 9. Mutation Testing Checkpoints

Command scope: `cargo mutants --package velvet-ballistics --file crates/velvet_ballistics/src/cli_schema.rs --file crates/velvet_ballistics/src/cli_schema_binary.rs --file crates/velvet_ballistics/src/cli_schema_diagnostics.rs --minimum-test-timeout 20 --timeout-multiplier 3`. Required kill rate: >= 90%.

- Deleted `cli_schema_version` branch or changed literal killed by schema constant tests and YAML snapshot tests.
- `binary_cli_schema_version` default `0` killed by version tests and decode version tests.
- `kind_name` returning `""`, debug string, or `VerificationReport` for all variants killed by 24-row table snapshot.
- `kind_id` returning `0`, `1` for all variants, swapped IDs, or off-by-one IDs killed by ID table, bijection proptest, and Kani.
- `kind_from_id` returning `Ok(Default::default())` for invalid IDs killed by ID 0/25/65535 tests.
- `command_for_kind` returning default `verify` for all kinds killed by 24-row command table and `command_for_kind_rejects_default_verify_mutant`.
- Deleted command mismatch branch killed by disallowed pair tests.
- Swapped `kind` and `command` arguments in `build_text_envelope` killed by `build_text_envelope_rejects_swapped_kind_command_arguments`.
- Dropped `exit_code` argument or defaulted to success killed by `build_text_envelope_preserves_exit_code_argument`.
- Dropped diagnostics summary argument killed by `build_text_envelope_preserves_diagnostics_summary_argument`.
- `build_diagnostic_report` returning empty/default report killed by exact diagnostic field assertions.
- Diagnostic limit `>` changed to `>=`, `<`, or deleted killed by 64-accepts/65-rejects tests and Kani.
- Message length fencepost changed from `> 4096` to `>= 4096` killed by 4096-accepts/4097-rejects tests.
- ANSI detector deletion or SGR-only detection killed by SGR, bare ESC, unterminated CSI, nested ANSI tests.
- Redaction check deletion killed by `UnredactedTaint { field: "side_effects" }` and remediation tests.
- `validate_text_envelope` returning `Ok(Default::default())` killed by every exact error variant scenario.
- Unsupported text version accepted killed by v2 exact-error test.
- Invalid exit code accepted killed by `InvalidExitCode { code: 9 }` test.
- `encode_postcard_envelope` default header or swapped kind ID killed by all-24-kind binary header tests.
- Payload max check deleted or fencepost changed killed by exact max success and 1_048_577 failure tests.
- Postcard serialization error swallowed killed by public failing serializer test.
- `decode_postcard_envelope` returning `Ok(Default::default())` killed by malformed input exact-error tests.
- Header length check deleted killed by `HeaderLengthMismatch { found: 51 }`.
- Magic/version/kind validation deleted or reordered killed by validation-order tests/proptest/Kani.
- CRC and BLAKE3 checks swapped killed by distinct `HeaderChecksumMismatch` and `PayloadDigestMismatch` tests.
- EOF checks deleted killed by empty, 51-byte, and truncated-payload tests.
- Decode-before-digest mutation killed by wrong-type payload and digest mismatch ordering tests.
- Runtime-boundary static gate default success killed by temp `vb_runtime` fixture test.

## 10. Static Gates And Resource/Side-Effect Checks

1. `moon ci` from repository root remains the canonical gate.
2. `scripts/check-cli-schema-boundaries.sh crates` must fail if `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`, or generated workflow code imports CLI schema, YAML, JSON, HTTP, or text-routing dependencies for this feature.
3. Source lint must prove no new `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` in schema modules.
4. Dependency gate must prove `serde_yaml` and `postcard` are cold CLI dependencies only, not runtime core dependencies.
5. Allocation sentinel tests for oversized payload must use a public test allocator counter or decode-state hook; setup resets the counter before decode and cleanup restores the default state after each test.
6. Filesystem fixtures live only under `target/test-fixtures/vb-qi37-13-1/`; every integration test removes its fixture directory on success and on failure via test tempdir drop.
7. Coverage gate: `cargo llvm-cov --package velvet-ballistics --tests --fail-under-lines 90` must include every `CliEnvelopeError` variant path.
8. No test writes bead state, Dolt state, or `.beads` runtime databases.

## 11. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| schema text constant | no input | exact `"velvet-ballistics/cli-output/v1"` | unit |
| schema binary constant | no input | exact `1_u16` | unit |
| kind happy path | IDs 1..=24 | exact matching `CliOutputKind` | unit/proptest/Kani |
| kind error zero | ID 0 | `Err(UnknownKindId { kind: 0 })` | unit |
| kind error first unassigned | ID 25 | `Err(UnknownKindId { kind: 25 })` | unit/fuzz |
| kind error max | ID 65535 | `Err(UnknownKindId { kind: 65535 })` | unit |
| unknown text kind | `MadeUpReport` | `Err(UnknownKindName { kind: "MadeUpReport" })` | unit/fuzz |
| command happy path | all 24 allowed pairs | exact canonical command table | unit |
| command mismatch | representative disallowed pairs | exact `KindCommandMismatch { kind, command }` | unit/proptest |
| text envelope happy path | all 24 valid payloads | exact YAML field values | integration |
| text empty version | `""` | `Err(EmptySchemaVersion)` | unit |
| text unsupported version | `"velvet-ballistics/cli-output/v2"` | `Err(UnsupportedTextSchemaVersion { found: "velvet-ballistics/cli-output/v2".to_owned() })` | unit/proptest |
| text payload mismatch | incident kind/status payload | `Err(KindPayloadMismatch { kind: IncidentReport })` | unit |
| invalid exit code | byte 9 | `Err(InvalidExitCode { code: 9 })` | unit |
| ANSI strings | SGR/bare ESC/CSI/nested | `Err(AnsiForbidden)` | unit/proptest/fuzz |
| secret taint | unredacted side effects | `Err(UnredactedTaint { field: "side_effects" })` | unit/proptest |
| diagnostics count min | 0 entries | exact empty diagnostics | unit |
| diagnostics count max | 64 entries | exact 64 entries preserved | unit/Kani |
| diagnostics count over | 65 entries | `Err(DiagnosticLimitExceeded { len: 65, max: 64 })` | unit/proptest/Kani |
| message max | 4096 bytes | exact message preserved | unit |
| message over | 4097 bytes | `Err(MessageTooLong { len: 4097, max: 4096 })` | unit/proptest/fuzz |
| stdout exact | status warning | one exact data YAML doc | integration/e2e |
| stderr exact | status warning | one exact diagnostic YAML doc | integration/e2e |
| stdout malformed | empty/malformed/ANSI | oracle failure naming stream | integration/fuzz |
| binary encode all kinds | 24 valid payloads | exact header/digest/CRC/payload | integration |
| payload exact max | len 1_048_576 | encode/decode succeeds with exact len | unit/integration |
| payload one over | len 1_048_577 | `Err(PayloadTooLarge { len: 1_048_577, max: 1_048_576 })` | unit/Kani |
| encode failure | public failing serializer | `Err(PostcardEncodeFailed)` | unit |
| empty bytes | `[]` | `Err(UnexpectedEof)` | unit/fuzz |
| short header | 51 bytes | `Err(UnexpectedEof)` | unit/fuzz/Kani |
| header len mismatch | declared 51 | `Err(HeaderLengthMismatch { found: 51 })` | unit/fuzz |
| bad magic | `0x00000000` | `Err(BadMagic { found: 0 })` | unit/fuzz |
| old version | 0 | `Err(MigrationRequired { from: 0, to: 1 })` | unit |
| new version | 2 | `Err(UnsupportedBinarySchemaVersion { version: 2 })` | unit |
| unknown binary kind | 25 | `Err(UnknownKindId { kind: 25 })` | unit/fuzz |
| arithmetic overflow | `u32::MAX` lengths | `Err(LengthOverflow)` | unit/Kani |
| CRC corrupt | protected header flip | `Err(HeaderChecksumMismatch)` | unit/fuzz |
| truncated payload | declared > actual | `Err(UnexpectedEof)` | unit/fuzz |
| digest corrupt | payload flip | `Err(PayloadDigestMismatch)` | unit/fuzz |
| wrong type payload | valid digest/wrong type | `Err(PostcardDecodeFailed)` | integration/fuzz |
| runtime boundary error variant | temp `vb_runtime` import through public boundary validator | `Err(RuntimeCoreBoundaryViolation { crate_name: "vb_runtime" })` and fixture cleanup | unit/static |
| runtime boundary shell gate | temp `vb_runtime` import through source scanner script | nonzero exit plus exact text `vb_runtime: CLI schema dependency forbidden` and fixture cleanup | static |

## 12. Acceptance Evidence Required From Test Writer

- `moon ci` passes.
- At least 84 named unit tests from Section 3 exist and run.
- All 24 kind/command/payload rows have at least one valid YAML construction test and one valid Postcard encode/decode test.
- Every `CliEnvelopeError` variant has at least one exact assertion with concrete payload values.
- Fuzz smoke artifacts show 60 seconds per target and saved regression corpus paths.
- Kani artifacts show all 7 harnesses pass with the stated bounds.
- `cargo-mutants` report for the command scope in Section 9 shows >= 90% killed mutants.
- Static gates prove runtime-core isolation and no forbidden panic/resource constructs.

## Open Questions

None for State 4 planning. If implementation cannot expose the constants or public test-only failing serializer described here, that is an API design failure to fix before tests are written, not a reason to weaken assertions.
