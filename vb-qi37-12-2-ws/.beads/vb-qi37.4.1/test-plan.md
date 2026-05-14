# Test Plan: vb-qi37.4.1 - accepted artifact envelope and runtime admission

## Summary

This repaired plan explicitly addresses every finding in `.beads/vb-qi37.4.1/test-plan-review.md`: unit density is raised above the 25-test trait-inclusive floor; placeholder/vague assertions are removed; max/min boundary BDD scenarios are named exactly as mandated; the in-memory-store escape hatch is removed; deterministic overflow/resource checks, CLI/integration oracles, mutation targets, static panic/resource checks, and Holzmann test-body constraints are added.

- Public contract boundaries in scope: 5 (`encode_accepted_artifact_v1`, `decode_accepted_artifact_v1`, `validate_accepted_artifact_v1`, `AcceptedArtifactStore::load_accepted_artifact`, `admit_artifact_run_v1`).
- Required unit-density floor: 25 unit tests (5x per boundary, counting trait load boundary). Planned unit scenarios: 42 minimum before property/fuzz/Kani tests.
- Trophy allocation: 42 unit / 18 integration / 2 e2e-acceptance / 6 static-command gates.
- Proptest invariants: 10.
- Fuzz targets: 5.
- Kani harnesses: 6.
- Mutation threshold: `cargo-mutants` must kill >=90% of non-equivalent mutants in accepted-artifact envelope, real store-load, and admission modules.
- Assertion rule: every planned assertion must compare exact values or exact typed variants. No `is_ok()`/`is_err()` assertions are acceptable.
- Holzmann test constraint: no loops in handwritten test bodies. Use `rstest` case tables, table-driven macro expansion, or proptest strategies for repeated EOF/gate/list cases. Side-effectful helpers must advertise temp storage/journal creation and cleanup.

## 1. Behavior Inventory

1. Accepted artifact encoder returns a compiled-IR storage envelope when the artifact is semantically valid.
2. Accepted artifact encoder accepts a minimal non-empty artifact when required fields are at their lower valid bounds.
3. Accepted artifact encoder accepts exactly maximum bounded artifact when every bounded field is at its valid maximum.
4. Accepted artifact encoder rejects invalid semantic artifacts with the exact `ArtifactEnvelopeError` variant when preconditions fail.
5. Accepted artifact decoder returns the original semantic artifact when envelope bytes are intact.
6. Accepted artifact decoder accepts exactly maximum payload length when payload length equals the configured maximum.
7. Accepted artifact decoder rejects forged overflowing payload length before allocation or slicing when the header advertises an impossible/over-bound length.
8. Accepted artifact decoder rejects bad magic when magic is not `MAGIC_COMPILED_ARTIFACT`.
9. Accepted artifact decoder rejects unsupported schema when schema is newer than supported.
10. Accepted artifact decoder rejects migration-required schema when schema is older than supported.
11. Accepted artifact decoder rejects bad record kind when kind is not `RecordKind::CompiledIr`.
12. Accepted artifact decoder rejects header length mismatch when header length is not `RECORD_HEADER_LEN`.
13. Accepted artifact decoder rejects checksum mismatch when CRC32C header checksum is invalid.
14. Accepted artifact decoder rejects payload digest mismatch when payload bytes are corrupted.
15. Accepted artifact decoder rejects oversized payload when payload length is `max + 1`.
16. Accepted artifact decoder rejects truncated bytes when header or payload ends early.
17. Accepted artifact decoder rejects invalid postcard payload when payload is not `AcceptedArtifactV1`.
18. Accepted artifact validator returns `ValidatedAcceptedArtifact` when all v1 semantics hold.
19. Accepted artifact validator accepts warning gates 1 and 15 when warnings are tied to accepted gates.
20. Accepted artifact validator accepts empty optional lists and exactly maximum lists when duplicate-free and bounded.
21. Accepted artifact validator rejects unsupported artifact version when version is not `velvet.artifact/v1`.
22. Accepted artifact validator rejects unsupported workflow version when language is not `velvet-ballastics/v1`.
23. Accepted artifact validator rejects empty workflow name when workflow name is empty.
24. Accepted artifact validator rejects invalid workflow name when name/scope validation fails.
25. Accepted artifact validator rejects empty IR when `ir_bytes` is empty.
26. Accepted artifact validator rejects IR digest mismatch when `ir_digest != blake3(ir_bytes)`.
27. Accepted artifact validator rejects storage key mismatch when storage key differs from artifact `ir_digest`.
28. Accepted artifact validator rejects zero digest when any required digest field is all zeroes.
29. Accepted artifact validator rejects invalid gate count when proof gate count is not 15.
30. Accepted artifact validator rejects failed verification gate when any of the 15 gates failed.
31. Accepted artifact validator rejects missing proof flag when bounded/taint-safe/retry-safe/durable/replayable is false.
32. Accepted artifact validator rejects invalid warning gate when warning gate is outside `1..=15`.
33. Accepted artifact validator rejects duplicate capability when artifact capabilities repeat.
34. Accepted artifact validator rejects duplicate secret when required secret identifiers repeat.
35. Accepted artifact validator rejects duplicate action ID when an idempotency list repeats.
36. Accepted artifact validator rejects bound exceedance when bounded fields exceed maxima.
37. Real compiled-IR store loads a validated artifact when keyspace key equals `ir_digest` and payload is `postcard(AcceptedArtifactV1)`.
38. Real compiled-IR store rejects legacy raw workflow payloads when accepted-artifact load is requested.
39. Runtime admission returns `RunAdmissionV1` when artifact, input, capabilities, secrets, clock, capacity, frame, and journal preconditions pass.
40. Runtime admission accepts exactly max input when schema validates.
41. Runtime admission accepts the last available run slot and frame slot when capacity is not exceeded.
42. Runtime admission rejects empty input when the artifact schema rejects empty input.
43. Runtime admission accepts empty input when the artifact schema explicitly allows empty input.
44. Runtime admission rejects raw submit when accepted artifacts are required.
45. Runtime admission rejects missing artifacts with `ArtifactNotFound`.
46. Runtime admission maps invalid artifact validation to `ArtifactInvalid` with exact source.
47. Runtime admission rejects oversized input with `InputTooLarge`.
48. Runtime admission rejects schema mismatch with `InputSchemaMismatch`.
49. Runtime admission rejects missing capability with `CapabilityDenied`.
50. Runtime admission rejects missing secret with `SecretUnavailable` without exposing values.
51. Runtime admission rejects duplicate run IDs with `RunAlreadyExists`.
52. Runtime admission rejects full active-run capacity with `ActiveRunCapacityExceeded`.
53. Runtime admission rejects frame exhaustion with `FrameAllocationFailed` without frame leak.
54. Runtime admission rejects journal append failure with `AdmissionJournalFailed` without execution.
55. Runtime admission rejects strict sync failure with `StrictDurabilityFailed` without execution.
56. Runtime admission rejects clock failure with `ClockUnavailable` without journaling.
57. Runtime admission records `RunAccepted` before execution and before/atomically with `RunAdmission`.
58. Strict runtime admission completes `SyncAll` before success is returned.
59. Journaled runtime admission exposes queued-but-unsynced data-loss window before execution begins.

## 2. Trophy Allocation

| Behavior(s) | Layer | Tool | Rationale |
|---|---|---|---|
| 1-4 | Unit + proptest | `#[test]`, `rstest`, `proptest` | Encoder semantics are pure and need min/max/off-by-one coverage. |
| 5-17 | Unit + fuzz + Kani | `#[test]`, `rstest`, `cargo-fuzz`, `kani` | Decoder is a byte parser; exact error classification and bounded allocation are required. |
| 18-36 | Unit + proptest + Kani | `#[test]`, `rstest`, `proptest`, `kani` | Validator is pure combinatorial logic over digests, bounds, gates, and duplicate-free lists. |
| 37-38 | Integration only, plus corruption fuzz | `/tests/` with real temp storage/keyspace | Store proof must use the real compiled-IR keyspace and envelope path. No mocks/fakes/in-memory substitute can satisfy this proof. |
| 39-59 | Integration + selected unit for pure admission checks | `/tests/`, public runtime APIs, real temp journal/frame pool | Admission is boundary behavior; tests observe state/events, not private interactions. |
| Public accepted-artifact journey | E2E/acceptance | CLI or public crate black-box test | Proves user-visible digest submission works and raw submit is rejected. |
| Governance/resource guarantees | Static gates | `moon ci`, clippy, deny, repo lint scripts, source grep scripts | Panic/resource/no-JSON guarantees are structural and must be enforced before runtime. |

Unit-density requirement: test writer must implement at least 42 named unit scenarios listed here. This exceeds the review-mandated floor of 25 for 5 public boundaries and prevents hiding integration/e2e counts inside the unit quota.

## 3. BDD Scenarios

### Behavior: accepted artifact encoder returns compiled-IR envelope when artifact is valid

- Test function name: `accepted_artifact_encoder_returns_compiled_ir_envelope_when_artifact_is_valid`
- Given: `AcceptedArtifactV1` with `artifact_version = "velvet.artifact/v1"`, `workflow_version = "velvet-ballastics/v1"`, workflow name `scope.valid_workflow`, non-zero `workflow_digest`, non-empty `ir_bytes = [0x01, 0x02]`, `ir_digest = blake3([0x01, 0x02])`, non-zero `action_contract_digest`, non-zero `input_schema_digest`, timestamp `1`, all 15 gate statuses accepted, proof flags all true, duplicate-free capabilities/secrets/action IDs, and warning gates in `1..=15`.
- When: `encode_accepted_artifact_v1(&artifact)` is called.
- Then: returned bytes length equals `RECORD_HEADER_LEN + payload_len`, header magic equals `MAGIC_COMPILED_ARTIFACT`, record kind equals `RecordKind::CompiledIr`, header length equals `RECORD_HEADER_LEN`, payload digest equals `blake3(payload)`, and CRC32C header checksum equals the recomputed CRC32C.

### Behavior: accepted artifact encoder accepts minimum valid artifact

- Test function name: `accepted_artifact_encoder_accepts_minimal_non_empty_artifact`
- Given: artifact has one-character valid workflow name if allowed by the name contract, one-byte `ir_bytes = [0x7f]`, `ir_digest = blake3([0x7f])`, zero warnings, zero capabilities, zero required secrets, zero idempotency-keyed actions, zero idempotency-attested actions, timestamp `1`, exactly 15 accepted gates, and all required non-zero digests.
- When: the artifact is encoded and decoded.
- Then: encoded header fields are exact compiled-IR envelope values and decoded artifact has one-byte IR, empty optional arrays, and gate count 15.

### Behavior: accepted artifact encoder accepts maximum valid artifact

- Test function name: `accepted_artifact_encoder_accepts_exactly_max_bounded_artifact`
- Given: artifact uses exactly `MAX_COMPILED_IR_BYTES` IR bytes, exactly maximum warning count, exactly maximum capabilities, exactly maximum required secrets, exactly maximum `idempotency_keyed`, exactly maximum `idempotency_attested`, exactly maximum warning message length, and encoded payload length exactly equal to the accepted-artifact payload maximum; all lists are duplicate-free and all digests match.
- When: `encode_accepted_artifact_v1(&artifact)` is called.
- Then: encoding succeeds with payload length equal to `MAX_ACCEPTED_ARTIFACT_PAYLOAD_BYTES`, decoding returns the same max-bound field lengths, and no `BoundExceeded` or `PayloadTooLarge` is returned.

### Behavior: accepted artifact decoder returns same artifact when envelope is valid

- Test function name: `accepted_artifact_decoder_returns_same_artifact_when_envelope_is_valid`
- Given: the explicit valid artifact from `accepted_artifact_encoder_returns_compiled_ir_envelope_when_artifact_is_valid` encoded by the public encoder.
- When: `decode_accepted_artifact_v1(&encoded)` is called.
- Then: returned artifact fields exactly equal the original artifact fields: version, name, workflow version, workflow digest, IR digest, IR bytes, action contract digest, timestamp, budget, capabilities, required secret identifiers, input schema digest, warnings, and verification proof.

### Behavior: accepted artifact decoder accepts maximum payload length

- Test function name: `accepted_artifact_decoder_accepts_exactly_max_payload_length`
- Given: valid encoded bytes whose envelope payload length equals `MAX_ACCEPTED_ARTIFACT_PAYLOAD_BYTES`, payload digest matches, and CRC32C matches.
- When: `decode_accepted_artifact_v1(&encoded)` runs.
- Then: returned artifact has exact max-bound field lengths and `ir_digest == blake3(ir_bytes)`.

### Behavior: accepted artifact decoder rejects forged overflowing payload length before allocation

- Test function name: `accepted_artifact_decoder_rejects_forged_overflowing_payload_length_without_allocation`
- Given: a 60-byte header whose payload length field is `usize::MAX`, `u64::MAX`, or the implementation's max representable encoded length, with compiled-artifact magic and otherwise deterministic fields.
- When: `decode_accepted_artifact_v1(&encoded_header_only)` runs under allocation instrumentation or a bounded allocator.
- Then: `Err(ArtifactEnvelopeError::PayloadTooLarge { len, max })` is returned before payload allocation/postcard decode, allocation count for advertised payload bytes remains zero, and no slice beyond `encoded.len()` is attempted.

### Behavior: accepted artifact decoder rejects storage-envelope corruptions with exact variants

- `accepted_artifact_decoder_returns_bad_magic_when_magic_is_wrong`: Given valid bytes with magic mutated to `found = 0xBAD0_BAD0`, When decoding, Then `Err(ArtifactEnvelopeError::BadMagic { found: 0xBAD0_BAD0 })`.
- `accepted_artifact_decoder_returns_unsupported_schema_when_schema_is_newer`: Given schema `CURRENT_SCHEMA_VERSION + 1`, When decoding, Then `Err(ArtifactEnvelopeError::UnsupportedSchemaVersion { version: CURRENT_SCHEMA_VERSION + 1 })`.
- `accepted_artifact_decoder_returns_migration_required_when_schema_is_older`: Given schema `CURRENT_SCHEMA_VERSION - 1`, When decoding, Then `Err(ArtifactEnvelopeError::MigrationRequired { from: CURRENT_SCHEMA_VERSION - 1, to: CURRENT_SCHEMA_VERSION })`.
- `accepted_artifact_decoder_returns_bad_record_kind_when_kind_is_not_compiled_ir`: Given `RecordKind::Workflow` or another non-compiled-IR kind, When decoding, Then `Err(ArtifactEnvelopeError::BadRecordKind { found })` with the mutated kind.
- `accepted_artifact_decoder_returns_header_length_mismatch_when_header_len_differs`: Given header length `RECORD_HEADER_LEN - 1`, When decoding, Then `Err(ArtifactEnvelopeError::HeaderLengthMismatch { found: RECORD_HEADER_LEN - 1 })`.
- `accepted_artifact_decoder_returns_header_checksum_mismatch_when_crc_is_wrong`: Given one header byte is changed without recomputing CRC32C, When decoding, Then `Err(ArtifactEnvelopeError::HeaderChecksumMismatch)`.
- `accepted_artifact_decoder_returns_payload_digest_mismatch_when_payload_byte_is_corrupted`: Given one payload byte is flipped after encoding, When decoding, Then `Err(ArtifactEnvelopeError::PayloadDigestMismatch)`.
- `accepted_artifact_decoder_returns_payload_too_large_when_payload_len_exceeds_bound`: Given payload length `MAX_ACCEPTED_ARTIFACT_PAYLOAD_BYTES + 1`, When decoding, Then `Err(ArtifactEnvelopeError::PayloadTooLarge { len: MAX_ACCEPTED_ARTIFACT_PAYLOAD_BYTES + 1, max: MAX_ACCEPTED_ARTIFACT_PAYLOAD_BYTES })`.
- `accepted_artifact_decoder_returns_unexpected_eof_when_header_or_payload_is_truncated`: Given `rstest` cases for lengths `0`, `1`, `RECORD_HEADER_LEN - 1`, `RECORD_HEADER_LEN`, and `RECORD_HEADER_LEN + payload_len - 1`, When decoding, Then each case returns `Err(ArtifactEnvelopeError::UnexpectedEof)`. No handwritten loop is allowed.
- `accepted_artifact_decoder_returns_postcard_decode_failed_when_payload_is_not_artifact`: Given a valid envelope around payload bytes `[0xde, 0xad, 0xbe, 0xef]` with matching digest/checksum, When decoding, Then `Err(ArtifactEnvelopeError::PostcardDecodeFailed)`.

### Behavior: accepted artifact validator accepts valid semantic boundaries

- `accepted_artifact_validator_returns_validated_artifact_when_all_semantics_hold`: Given a decoded valid artifact and storage key `D = artifact.ir_digest`, When validation runs, Then returned `ValidatedAcceptedArtifact` exposes `ir_digest = D`, `workflow_digest = artifact.workflow_digest`, and gate count 15.
- `accepted_artifact_validator_accepts_warning_gates_one_and_fifteen`: Given warnings exactly at gates 1 and 15 with non-empty bounded messages and corresponding warning-only pass statuses, When validation runs, Then returned validated artifact contains warning gate numbers `[1, 15]`.
- `accepted_artifact_validator_accepts_empty_optional_lists_and_exactly_max_lists`: Given two `rstest` cases: (a) zero capabilities/secrets/idempotency lists and (b) exactly maximum duplicate-free capabilities/secrets/idempotency lists/messages, When validation runs, Then case (a) returns arrays of length 0 and case (b) returns arrays of exact max lengths.

### Behavior: accepted artifact validator rejects every `ArtifactEnvelopeError` semantic variant

- `accepted_artifact_validator_returns_unsupported_artifact_version_when_version_differs`: Given version `velvet.artifact/v2`, When validation runs, Then `Err(ArtifactEnvelopeError::UnsupportedArtifactVersion { version: "velvet.artifact/v2" })`.
- `accepted_artifact_validator_returns_unsupported_workflow_version_when_language_differs`: Given workflow version `velvet-ballastics/v2`, When validation runs, Then `Err(ArtifactEnvelopeError::UnsupportedWorkflowVersion { version: "velvet-ballastics/v2" })`.
- `accepted_artifact_validator_returns_empty_workflow_name_when_name_is_empty`: Given name `""`, When validation runs, Then `Err(ArtifactEnvelopeError::EmptyWorkflowName)`.
- `accepted_artifact_validator_returns_invalid_workflow_name_when_name_scope_is_invalid`: Given name `"../bad name"`, When validation runs, Then `Err(ArtifactEnvelopeError::InvalidWorkflowName)`.
- `accepted_artifact_validator_returns_empty_ir_when_ir_bytes_are_empty`: Given `ir_bytes = []`, When validation runs, Then `Err(ArtifactEnvelopeError::EmptyIr)`.
- `accepted_artifact_validator_returns_ir_digest_mismatch_when_ir_hash_differs`: Given `ir_bytes = [1,2,3]`, `computed = blake3([1,2,3])`, and `expected = blake3([9,9,9])`, When validation runs, Then `Err(ArtifactEnvelopeError::IrDigestMismatch { expected, computed })`.
- `accepted_artifact_validator_returns_storage_key_digest_mismatch_when_key_differs_from_ir_digest`: Given `artifact.ir_digest = D` and storage key `K = blake3([0xaa])`, `K != D`, When validation runs, Then `Err(ArtifactEnvelopeError::StorageKeyDigestMismatch { key: K, artifact: D })`.
- `accepted_artifact_validator_returns_zero_digest_when_required_digest_is_zero`: Given `rstest` cases for `workflow_digest`, `ir_digest`, `action_contract_digest`, and `input_schema_digest` set to all-zero digest, When validation runs, Then `Err(ArtifactEnvelopeError::ZeroDigest { field })` with the exact field enum/name. No handwritten loop is allowed.
- `accepted_artifact_validator_returns_invalid_gate_count_when_count_is_not_fifteen`: Given counts `0`, `2`, `14`, and `16` as `rstest` cases, When validation runs, Then `Err(ArtifactEnvelopeError::InvalidGateCount { found })`.
- `accepted_artifact_validator_returns_verification_gate_failed_when_any_gate_failed`: Given `rstest` or proptest cases for failed gate 1 through 15, When validation runs, Then `Err(ArtifactEnvelopeError::VerificationGateFailed { gate })` with the exact gate.
- `accepted_artifact_validator_returns_missing_required_proof_flag_when_any_flag_is_false`: Given cases for `bounded`, `taint_safe`, `retry_safe`, `durable`, and `replayable` individually false, When validation runs, Then `Err(ArtifactEnvelopeError::MissingRequiredProofFlag { flag })`.
- `accepted_artifact_validator_returns_invalid_warning_gate_when_warning_gate_is_out_of_range`: Given warning gate `0` and `16` cases, When validation runs, Then `Err(ArtifactEnvelopeError::InvalidWarningGate { gate })`.
- `accepted_artifact_validator_returns_duplicate_capability_when_capability_repeats`: Given capability `network.github` appears twice, When validation runs, Then `Err(ArtifactEnvelopeError::DuplicateCapability { capability: network.github })`.
- `accepted_artifact_validator_returns_duplicate_secret_when_secret_repeats`: Given secret identifier `github_token` appears twice, When validation runs, Then `Err(ArtifactEnvelopeError::DuplicateSecret { secret: github_token })`.
- `accepted_artifact_validator_returns_duplicate_action_id_when_idempotency_action_repeats`: Given `idempotency_keyed` repeats action `A7` and separately `idempotency_attested` repeats `A7`, When validation runs, Then `Err(ArtifactEnvelopeError::DuplicateActionId { list: IdempotencyKeyed, action: A7 })` or `Err(ArtifactEnvelopeError::DuplicateActionId { list: IdempotencyAttested, action: A7 })`.
- `accepted_artifact_validator_returns_bound_exceeded_when_bounded_field_is_too_large`: Given each bounded field at `max + 1` as separate `rstest` cases, When validation runs, Then `Err(ArtifactEnvelopeError::BoundExceeded { field, len: max + 1, max })`.

### Behavior: real compiled-IR store loads only accepted-artifact payloads

- `artifact_store_returns_validated_artifact_when_compiled_ir_record_contains_accepted_artifact_payload`: Given real temp storage/keyspace created under a temporary directory, a `CompiledIrRecord { digest: D, ir: postcard(AcceptedArtifactV1 { ir_digest: D, ... }) }` persisted in the actual compiled-IR keyspace, and cleanup registered for the tempdir/journal, When `AcceptedArtifactStore::load_accepted_artifact(D)` is called, Then returned artifact has `ir_digest = D`, `workflow_digest` equal to stored workflow digest, and gate count 15.
- `artifact_store_returns_artifact_invalid_when_legacy_raw_workflow_parts_are_loaded_as_accepted_artifact`: Given the real compiled-IR keyspace stores legacy raw workflow bytes under digest `D`, When accepted-artifact load runs, Then the store returns `Err(ArtifactEnvelopeError::PostcardDecodeFailed)` or the runtime wrapper returns `Err(AdmissionError::ArtifactInvalid { digest: D, source: ArtifactEnvelopeError::PostcardDecodeFailed })`.

### Behavior: runtime admission succeeds at valid boundaries

- `runtime_admission_returns_run_admission_when_artifact_input_capabilities_secrets_and_journal_are_valid`: Given required policy, validated artifact `D`, schema-valid input `I`, grants covering all capabilities, secret identifiers present, new run `R`, non-full capacity, frame available, clock timestamp `T = 123`, and journal append/sync success, When admission runs, Then `RunAdmissionV1 { run: R, artifact_digest: D, workflow_digest: artifact.workflow_digest, input_digest: blake3(I), capabilities_granted: exact grants, secrets_available: exact identifiers, admitted_at_unix_ms: 123 }` is returned and event evidence shows `RunAccepted` before execution.
- `runtime_admission_accepts_exactly_max_input_when_schema_valid`: Given input length exactly `MAX_RUNTIME_INPUT_BYTES` and schema validator accepts it for schema digest `S`, When admission runs, Then returned `input_digest = blake3(input)` and no `InputTooLarge` is returned.
- `runtime_admission_accepts_last_available_run_and_frame_slot`: Given active-run capacity `N`, `N - 1` runs already active, and frame pool has exactly one free frame, When valid admission runs, Then success returns run `R`, active run count becomes `N`, and frame allocation evidence identifies the last slot assigned to `R` without leak.
- `runtime_admission_accepts_empty_input_when_schema_allows_empty`: Given input `[]` and artifact schema digest `S_empty_ok` whose validator accepts empty bytes, When admission runs, Then returned `input_digest = blake3([])` and run `R` is admitted.

### Behavior: runtime admission rejects every `AdmissionError` variant without execution-visible mutation

- `runtime_admission_returns_admission_required_when_raw_submit_is_used_under_required_policy`: Given accepted-artifact policy required, When `submit_direct`, `submit_compiled_with_inputs`, or raw `ShardCommand::Submit` is used, Then `Err(AdmissionError::AdmissionRequired)`, no frame, no run state, and no `RunAccepted` event.
- `runtime_admission_returns_artifact_not_found_when_store_has_no_digest`: Given no store record for `D`, When admission runs, Then `Err(AdmissionError::ArtifactNotFound { digest: D })` and no mutation.
- `runtime_admission_returns_artifact_invalid_when_store_validation_fails`: Given store returns `ArtifactEnvelopeError::PayloadDigestMismatch`, When admission runs, Then `Err(AdmissionError::ArtifactInvalid { digest: D, source: ArtifactEnvelopeError::PayloadDigestMismatch })` and no mutation.
- `runtime_admission_returns_input_too_large_when_input_exceeds_bound`: Given input length `MAX_RUNTIME_INPUT_BYTES + 1`, When admission runs, Then `Err(AdmissionError::InputTooLarge { len: MAX_RUNTIME_INPUT_BYTES + 1, max: MAX_RUNTIME_INPUT_BYTES })` and no journal event.
- `runtime_admission_returns_input_schema_mismatch_when_input_fails_schema`: Given schema digest `S` and input rejected by schema `S`, When admission runs, Then `Err(AdmissionError::InputSchemaMismatch { schema_digest: S })`.
- `runtime_admission_returns_input_schema_mismatch_when_empty_input_disallowed`: Given input `[]` and schema digest `S_non_empty` that rejects empty bytes, When admission runs, Then `Err(AdmissionError::InputSchemaMismatch { schema_digest: S_non_empty })` and no `RunAccepted`.
- `runtime_admission_returns_capability_denied_when_required_capability_is_missing`: Given artifact declares action `A` requires capability `C` and grants are `G` without `C`, When admission runs, Then `Err(AdmissionError::CapabilityDenied { action: A, required: C, granted: G })`.
- `runtime_admission_returns_secret_unavailable_when_required_secret_is_absent`: Given required secret `github_token` absent from presence set, When admission runs, Then `Err(AdmissionError::SecretUnavailable { secret: github_token })` and error display/debug/event payload does not contain any secret value fixture string.
- `runtime_admission_returns_run_already_exists_when_run_is_active_or_accepted`: Given run `R` already active or durably accepted, When admission runs, Then `Err(AdmissionError::RunAlreadyExists { run: R })`.
- `runtime_admission_returns_active_run_capacity_exceeded_when_capacity_is_full`: Given capacity `N` and `N` active runs, When admission runs, Then `Err(AdmissionError::ActiveRunCapacityExceeded { capacity: N })` and no frame leak.
- `runtime_admission_returns_frame_allocation_failed_when_frame_pool_is_exhausted`: Given run capacity available but frame pool free slots `0`, When admission runs, Then `Err(AdmissionError::FrameAllocationFailed)`, no run state, and no `RunAccepted`.
- `runtime_admission_returns_admission_journal_failed_when_run_events_cannot_be_recorded`: Given journal append deterministically fails with source `E_append`, When admission runs, Then `Err(AdmissionError::AdmissionJournalFailed { source: E_append })`, any temporary frame is released, and execution start marker is absent.
- `runtime_admission_returns_strict_durability_failed_when_sync_all_fails`: Given strict policy, append succeeds, and `SyncAll` fails with `E_sync`, When admission runs, Then `Err(AdmissionError::StrictDurabilityFailed { source: E_sync })` and execution marker is absent.
- `runtime_admission_returns_clock_unavailable_when_clock_cannot_supply_timestamp`: Given the clock returns unavailable before journal append, When admission runs, Then `Err(AdmissionError::ClockUnavailable)` and no `RunAccepted` event exists.

### Behavior: runtime durability and public acceptance are externally observable

- `runtime_admission_syncs_before_returning_when_policy_is_strict`: Given strict policy and journal instrumentation, When admission succeeds, Then `sync_all_completed = true` before success is returned and, if events are separate, `RunAccepted.seq < RunAdmission.seq`.
- `runtime_admission_queues_before_execution_when_policy_is_journaled`: Given journaled policy, When admission succeeds, Then `RunAccepted` is accepted into the queue before execution begins and returned metadata marks the unsynced data-loss window.
- `accepted_artifact_cli_submits_by_digest_and_observes_run_accepted_before_execution`: Given CLI/public API configured to require accepted artifacts and real storage contains artifact digest `D`, When user submits `D` with schema-valid input, Then output includes run id `R`, artifact digest `D`, input digest `blake3(input)`, and event query shows `RunAccepted` before first execution event.
- `accepted_artifact_cli_rejects_raw_submit_when_required`: Given same required mode, When user submits raw workflow/compiled workflow, Then CLI exits non-zero with public error code/message mapped to `AdmissionRequired`, prints no run id, and event query shows no `RunAccepted` for the attempted run.

## 4. Proptest Invariants

1. `encode_accepted_artifact_v1`/`decode_accepted_artifact_v1` roundtrip: any valid bounded artifact decodes to semantically equal artifact and exact envelope digest/checksum. Strategy: bounded artifact generator including min, max, duplicate-free lists, warning gates 1/15. Anti-invariant: invalid semantic fields return exact `ArtifactEnvelopeError`.
2. Max/min bound preservation: generated fields at `0 where optional`, `1 where required`, `max`, and `max + 1` produce exact success at valid boundaries and exact `BoundExceeded`/`PayloadTooLarge` at invalid boundaries.
3. Storage envelope corruption classifier: any generated single-byte mutation returns exact header/payload error or valid artifact only if all invariants still hold; never panics.
4. Forged length classifier: arbitrary advertised lengths above max return `PayloadTooLarge { len, max }` before allocation; truncations return `UnexpectedEof`.
5. Digest semantics: validation succeeds only when storage key equals `artifact.ir_digest`, IR digest equals `blake3(ir_bytes)`, and all required digests are non-zero.
6. Verification proof completeness: gate count must be 15, every status accepted/warning-only pass, all proof flags true.
7. Duplicate-free bounded lists: capabilities/secrets/idempotency lists validate iff duplicate-free and length <= bound.
8. Warning gate bounds: gates 1 and 15 can succeed; gates 0 and 16 always return `InvalidWarningGate { gate }`.
9. Runtime admission state preservation: for any single generated precondition failure, snapshots show no new run, no frame leak, no execution marker, no `RunAccepted`.
10. Secret non-leakage: generated secret values are never present in errors, debug strings, display strings, event records, or admission records; only identifiers appear.

## 5. Fuzz Targets

1. Accepted artifact envelope decoder. Input: arbitrary bytes. Risk: panic, unchecked slicing, OOM from forged length, corrupt acceptance. Seeds: empty, 59 bytes, valid artifact, bad magic, bad kind, max payload, max+1 payload, `usize::MAX`/`u64::MAX` payload length, checksum mismatch, payload digest mismatch, legacy raw workflow payload.
2. Postcard `AcceptedArtifactV1` payload decoder. Input: arbitrary payload wrapped in valid envelope. Risk: postcard panic, malformed bounds accepted. Seeds: minimal valid, exactly max valid, wrong versions, empty name, duplicate capability, gate count 2, warning gates 0/1/15/16.
3. Runtime admission input validator. Input: arbitrary input bytes plus schema selector. Risk: schema confusion, digest misbinding, max/off-by-one bypass. Seeds: empty allowed, empty rejected, max valid, max+1 invalid, random bytes, high-bit bytes.
4. Real compiled-IR store load path. Input: arbitrary record bytes persisted through real temp compiled-IR keyspace. Risk: fake path masking keyspace bug, legacy payload accepted, digest conflation. Seeds: valid accepted artifact, legacy raw parts, key mismatch, IR mismatch, zero digest.
5. CLI/public artifact digest parser if surfaced. Input: arbitrary digest string/bytes. Risk: invalid digest accepted, raw fallback, panic on length/hex. Seeds: valid digest, empty, too short, too long, non-hex, all-zero, missing digest, corrupt artifact digest.

## 6. Kani Harnesses

1. Record header bounds never permit unchecked payload access. Property: all bounded byte slices decode to classified error or validated artifact without out-of-bounds access. Bound: `0..=RECORD_HEADER_LEN + 64`, symbolic payload length including `max`, `max+1`, and representable overflow sentinels.
2. Forged payload length cannot allocate advertised bytes. Property: advertised length > max implies `PayloadTooLarge` before allocation/slice. Bound: symbolic length field over implementation width.
3. Verification gate completeness. Property: only 1..=15 gate numbers validate, any failed gate rejects, legacy 2/13-gate proofs reject. Bound: 15 symbolic gate statuses and warning gate `0..=16`.
4. Bounded collection duplicate detection. Property: validation succeeds iff symbolic list is duplicate-free and within bound. Bound: representative `0..=MAX_LIST_BOUND+1`.
5. Admission error paths do not cross durability boundary. Property: any failure before journal success leaves `run_inserted=false`, `execution_started=false`, `run_accepted_recorded=false`, `frame_leaked=false`. Bound: one symbolic failure phase.
6. Strict admission success requires sync completion. Property: success under `Strict` implies `RunAccepted`, `RunAdmission`, `sync_all_completed`, and execution after `RunAccepted`. Bound: one run, symbolic append/sync outcomes.

## 7. Mutation Testing Checkpoints

Minimum: `cargo-mutants` >=90% killed for accepted artifact envelope, real store-load, and admission modules; equivalent survivors require written justification.

Critical mutants that must be killed:

- Change `velvet.artifact/v1` or `velvet-ballastics/v1`: killed by unsupported version scenarios.
- Accept empty or max+1 required fields: killed by minimal/max/bound-exceeded scenarios.
- Change `>` to `>=` for max IR/warnings/capabilities/secrets/idempotency/message/payload/input: killed by exact-max encoder/decoder/validator/admission scenarios.
- Remove forged-length preallocation guard: killed by `accepted_artifact_decoder_rejects_forged_overflowing_payload_length_without_allocation` and Kani forged-length harness.
- Accept non-compiled magic/kind/header length/schema: killed by exact decoder corruption scenarios.
- Remove CRC or payload digest checks: killed by checksum/digest mismatch scenarios.
- Map postcard failure to a vague error: killed by exact `PostcardDecodeFailed` scenarios.
- Compare storage key to workflow digest or action digest: killed by storage key mismatch and digest semantics property.
- Skip `blake3(ir_bytes)` verification: killed by IR digest mismatch scenario.
- Accept zero required digest: killed by `ZeroDigest { field }` cases.
- Accept legacy 2-gate or 13-gate proof: killed by invalid gate count cases.
- Ignore failed gate or proof flag: killed by exact failed-gate/proof-flag cases.
- Change warning upper bound to 13 or 16: killed by warning gates 1/15 success and gate 16 failure.
- Remove duplicate detection for capabilities/secrets/actions: killed by duplicate scenarios and Kani duplicate harness.
- Use fake/in-memory path instead of real compiled-IR keyspace: killed by real-storage integration proof.
- Treat legacy raw workflow bytes as accepted artifact: killed by real store legacy rejection.
- Runtime checks artifact existence only: killed by `ArtifactInvalid { source }` integration.
- Permit raw submit in required mode: killed by integration and CLI raw-submit rejection.
- Compute input digest from artifact bytes: killed by success and max-input scenarios asserting `blake3(input)`.
- Reject exact max input or last slot due to off-by-one: killed by max-input and last-slot admission scenarios.
- Accept max+1 input: killed by `InputTooLarge { len: max + 1, max }`.
- Conflate empty-input allowed/rejected schemas: killed by empty accepted and empty rejected scenarios.
- Allow missing capability or secret, or leak secret value: killed by exact denial and leakage properties.
- Record `RunAccepted` before failed validation or fail to rollback frame: killed by all no-mutation error scenarios and Kani no-boundary harness.
- Start execution before `RunAccepted`: killed by integration/e2e event-order checks.
- Skip strict `SyncAll` or return success after sync failure: killed by strict success/failure scenarios.
- Treat zero timestamp as normal production success: killed by clock-unavailable/zero-timestamp policy scenario; if test-only clock permits zero, that must be scoped to deterministic tests only and never production admission.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| encode normal valid | valid v1 artifact | exact compiled-IR envelope header/digest/checksum | unit |
| encode minimal | one-byte IR, empty optional lists | decoded artifact with one-byte IR and empty arrays | unit |
| encode max | exact max fields/payload | decoded artifact with exact max lengths | unit/proptest |
| decode valid | valid encoded bytes | artifact semantically equal to original | unit |
| decode max payload | payload len = max | artifact with exact max fields | unit |
| forged overflow length | length = usize/u64 max | `Err(ArtifactEnvelopeError::PayloadTooLarge { len, max })`, zero advertised allocation | unit/Kani |
| bad magic | wrong magic | `Err(ArtifactEnvelopeError::BadMagic { found })` | unit/fuzz |
| newer schema | schema current+1 | `Err(ArtifactEnvelopeError::UnsupportedSchemaVersion { version })` | unit |
| older schema | schema current-1 | `Err(ArtifactEnvelopeError::MigrationRequired { from, to })` | unit |
| bad kind | non-CompiledIr | `Err(ArtifactEnvelopeError::BadRecordKind { found })` | unit |
| bad header len | len != 60 | `Err(ArtifactEnvelopeError::HeaderLengthMismatch { found })` | unit |
| bad CRC | mutated header | `Err(ArtifactEnvelopeError::HeaderChecksumMismatch)` | unit/fuzz |
| corrupt payload | flipped payload | `Err(ArtifactEnvelopeError::PayloadDigestMismatch)` | unit/fuzz |
| oversized payload | max+1 | `Err(ArtifactEnvelopeError::PayloadTooLarge { len, max })` | unit/fuzz |
| EOF | rstest truncation cases | `Err(ArtifactEnvelopeError::UnexpectedEof)` | unit/proptest |
| invalid postcard | non-artifact payload | `Err(ArtifactEnvelopeError::PostcardDecodeFailed)` | unit/fuzz |
| valid semantics | all constraints hold | validated artifact exact digests/proof | unit |
| warning gates 1/15 | boundary warnings | validated artifact warning gates `[1, 15]` | unit |
| empty/max lists | optional empty or exact max | validated exact lengths | unit |
| bad artifact version | v2 | `Err(ArtifactEnvelopeError::UnsupportedArtifactVersion { version })` | unit |
| bad workflow version | v2 | `Err(ArtifactEnvelopeError::UnsupportedWorkflowVersion { version })` | unit |
| empty name | `""` | `Err(ArtifactEnvelopeError::EmptyWorkflowName)` | unit |
| invalid name | invalid scope/name | `Err(ArtifactEnvelopeError::InvalidWorkflowName)` | unit |
| empty IR | `[]` | `Err(ArtifactEnvelopeError::EmptyIr)` | unit |
| IR mismatch | digest mismatch | `Err(ArtifactEnvelopeError::IrDigestMismatch { expected, computed })` | unit/proptest |
| storage key mismatch | key != IR digest | `Err(ArtifactEnvelopeError::StorageKeyDigestMismatch { key, artifact })` | unit/proptest |
| zero digest | any required zero | `Err(ArtifactEnvelopeError::ZeroDigest { field })` | unit/proptest |
| invalid gate count | 0/2/14/16 | `Err(ArtifactEnvelopeError::InvalidGateCount { found })` | unit |
| failed gate | gate 1..=15 | `Err(ArtifactEnvelopeError::VerificationGateFailed { gate })` | unit/proptest |
| missing proof flag | one false flag | `Err(ArtifactEnvelopeError::MissingRequiredProofFlag { flag })` | unit |
| invalid warning gate | 0/16 | `Err(ArtifactEnvelopeError::InvalidWarningGate { gate })` | unit |
| duplicate capability | repeated C | `Err(ArtifactEnvelopeError::DuplicateCapability { capability: C })` | unit |
| duplicate secret | repeated S | `Err(ArtifactEnvelopeError::DuplicateSecret { secret: S })` | unit |
| duplicate action | repeated A | `Err(ArtifactEnvelopeError::DuplicateActionId { list, action: A })` | unit |
| bound exceeded | max+1 field | `Err(ArtifactEnvelopeError::BoundExceeded { field, len, max })` | unit |
| real store accepted payload | real compiled-IR keyspace | validated artifact exact `ir_digest`/gate count | integration |
| real store legacy payload | raw workflow bytes | `Err(PostcardDecodeFailed)` or exact `ArtifactInvalid` source | integration/fuzz |
| valid admission | all preconditions | exact `RunAdmissionV1` and ordered event evidence | integration |
| exact max input | len = max and schema valid | success with `input_digest = blake3(input)` | integration |
| last run/frame slot | capacity-1 active, one frame | success and no leak | integration |
| empty input allowed | `[]`, allowing schema | success with `blake3([])` | integration |
| empty input rejected | `[]`, rejecting schema | `Err(AdmissionError::InputSchemaMismatch { schema_digest })` | integration |
| raw submit required | raw workflow | `Err(AdmissionError::AdmissionRequired)` and no mutation | integration/e2e |
| missing artifact | store miss | `Err(AdmissionError::ArtifactNotFound { digest })` | integration |
| invalid artifact | corrupt source | `Err(AdmissionError::ArtifactInvalid { digest, source })` | integration |
| input too large | max+1 | `Err(AdmissionError::InputTooLarge { len, max })` | integration |
| schema mismatch | invalid input | `Err(AdmissionError::InputSchemaMismatch { schema_digest })` | integration |
| capability missing | grants omit C | `Err(AdmissionError::CapabilityDenied { action, required, granted })` | integration |
| secret absent | missing ID | `Err(AdmissionError::SecretUnavailable { secret })`, no value leak | integration/proptest |
| duplicate run | R exists | `Err(AdmissionError::RunAlreadyExists { run })` | integration |
| capacity full | N active of N | `Err(AdmissionError::ActiveRunCapacityExceeded { capacity: N })` | integration |
| frame exhausted | no frame | `Err(AdmissionError::FrameAllocationFailed)`, no leak | integration/Kani |
| journal failure | append fails | `Err(AdmissionError::AdmissionJournalFailed { source })` | integration/Kani |
| strict sync failure | SyncAll fails | `Err(AdmissionError::StrictDurabilityFailed { source })` | integration/Kani |
| clock unavailable | no timestamp | `Err(AdmissionError::ClockUnavailable)` | integration |
| strict success | strict policy | sync completed before return, ordered events | integration |
| journaled success | journaled policy | queued boundary before execution, data-loss window visible | integration |
| CLI digest submit | public digest path | run id + artifact/input digests + RunAccepted before execution | e2e |
| CLI raw reject | public raw path | non-zero/typed `AdmissionRequired`, no run id/event | e2e |
| static governance | source tree | `moon ci`; no unsafe/unwrap/expect/panic/todo/unimplemented/dbg; no runtime JSON/YAML/HTTP | static |

## Static Resource, Panic, and Command Gates

Implementation acceptance evidence must include exact command names, exit status, and report/log paths for:

1. `moon ci` from repository root.
2. Targeted unit/integration commands for accepted artifact envelope, real compiled-IR store, and runtime admission tests.
3. Source lint proving no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` were introduced.
4. Runtime-core scan proving no JSON, YAML, or HTTP parsing is introduced for the accepted-artifact envelope or admission path.
5. Bounded allocation/resource check for forged payload length and max input/artifact cases.
6. `cargo fuzz` smoke runs for all five fuzz targets.
7. `kani` proof runs for all six harnesses.
8. `cargo mutants` report with >=90% killed mutants.

## Open Questions

1. If implementation names modules/newtypes differently, tests must keep behavior names and assert the public equivalent exact variants/fields.
2. The contract does not define the concrete input schema validator. Test writer must use the first public schema boundary and create two deterministic schemas: `S_empty_ok` and `S_non_empty`.
3. If zero timestamps are permitted only by a test-only deterministic clock, tests must prove production admission maps unavailable/invalid clock to `AdmissionError::ClockUnavailable` while test-only zero is scoped and documented.
