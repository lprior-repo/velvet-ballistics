# Verification Layers: vb-qi37.4.1 - Accepted Artifact Envelope

## Boundary

- **Verified kernel**: Artifact envelope encoding/decoding, digest computation, structure validation, gate/proof validation, idempotency/capability/secret list deduplication, storage key binding.
- **Lean contract projection**: `lean-contract.md` for all pure deterministic clauses.
- **Runtime shell**: Journal persistence, `RunAccepted`/`RunAdmission` durability, clock access, frame pool allocation, capability/secret runtime checking, policy enforcement.
- **External systems excluded from formal proof**: Fjall storage backend, network, external processes, clock hardware.

## Layer Assignment

### Encoding/Decoding (Envelope Integrity)

| Clause | Layer | Checker | Evidence |
|--------|-------|---------|----------|
| PRE-001: magic = MAGIC_COMPILED_ARTIFACT | kani | `kani::verify_bad_magic` | formal-verification-report.md |
| PRE-002: schema version validated | kani | `kani::verify_schema_version` | formal-verification-report.md |
| PRE-003: record kind = CompiledIr | kani | `kani::verify_record_kind` | formal-verification-report.md |
| PRE-004: header checksum valid | kani | `kani::verify_header_checksum` | formal-verification-report.md |
| PRE-005: payload digest valid | kani | `kani::verify_payload_digest` | formal-verification-report.md |
| PRE-006: postcard decode succeeds | kani | `kani::verify_postcard_decode` | formal-verification-report.md |
| POST-001: roundtrip preserves artifact | lean | `lake build VelvetBallistics.ArtifactEnvelope` | lean-report.md |
| POST-002: envelope header valid | lean + kani | `lake build` + `kani::verify_header` | lean-report.md + formal-verification-report.md |

### Digest Computation

| Clause | Layer | Checker | Evidence |
|--------|-------|---------|----------|
| PRE-010: ir_digest == blake3(ir_bytes) | lean | `lake build VelvetBallistics.Digest::ir_digest_correct` | lean-report.md |
| PRE-011: workflow_digest non-zero | kani | `kani::verify_nonzero_digest` | formal-verification-report.md |
| PRE-012: ir_digest non-zero | kani | `kani::verify_nonzero_digest` | formal-verification-report.md |
| PRE-013: action_contract_digest non-zero | kani | `kani::verify_nonzero_digest` | formal-verification-report.md |
| INV-002: digest separation invariant | lean | `lake build VelvetBallistics.Digest::digestes_distinct` | lean-report.md |

### Artifact Validation

| Clause | Layer | Checker | Evidence |
|--------|-------|---------|----------|
| PRE-014: artifact_version = velvet.artifact/v1 | kani | `kani::verify_artifact_version` | formal-verification-report.md |
| PRE-015: workflow_version = velvet-ballistics/v1 | kani | `kani::verify_workflow_version` | formal-verification-report.md |
| PRE-016: workflow_name non-empty | kani | `kani::verify_workflow_name` | formal-verification-report.md |
| PRE-017: ir_bytes non-empty | kani | `kani::verify_ir_bytes` | formal-verification-report.md |
| PRE-018: ir_bytes within MAX_COMPILED_IR_BYTES | kani | `kani::verify_ir_bytes_bound` | formal-verification-report.md |
| PRE-019: verification.gate_count == 15 | lean | `lake build VelvetBallistics.Verification::gate_count_positive` | lean-report.md |
| PRE-020: all proof flags true | lean | `lake build VelvetBallistics.Verification::proof_flags_imply_accepted` | lean-report.md |
| PRE-021: bounded warnings in 1..=15 | lean | `lake build VelvetBallistics.Verification::warning_gate_in_range` | lean-report.md |
| POST-003: storage_key_digest == artifact.ir_digest | lean | `lake build VelvetBallistics.Storage::storage_key_matches_digest` | lean-report.md |

### List Deduplication

| Clause | Layer | Checker | Evidence |
|--------|-------|---------|----------|
| PRE-022: idempotency_keyed duplicate-free | lean | `lake build VelvetBallistics.Idempotency::no_duplicates_keyed` | lean-report.md |
| PRE-023: idempotency_attested duplicate-free | lean | `lake build VelvetBallistics.Idempotency::no_duplicates_attested` | lean-report.md |
| PRE-024: capabilities duplicate-free | lean | `lake build VelvetBallistics.Capability::no_duplicate_capabilities` | lean-report.md |
| PRE-025: required_secrets duplicate-free | lean | `lake build VelvetBallistics.Secret::no_duplicate_secrets` | lean-report.md |

### Error Taxonomy

| Clause | Layer | Checker | Evidence |
|--------|-------|---------|----------|
| ERR-001: BadMagic | kani | `kani::verify_bad_magic_error` | formal-verification-report.md |
| ERR-002: UnsupportedSchemaVersion | kani | `kani::verify_schema_version_error` | formal-verification-report.md |
| ERR-003: MigrationRequired | kani | `kani::verify_migration_error` | formal-verification-report.md |
| ERR-004: BadRecordKind | kani | `kani::verify_record_kind_error` | formal-verification-report.md |
| ERR-005: HeaderLengthMismatch | kani | `kani::verify_header_length_error` | formal-verification-report.md |
| ERR-006: HeaderChecksumMismatch | kani | `kani::verify_header_checksum_error` | formal-verification-report.md |
| ERR-007: PayloadDigestMismatch | kani | `kani::verify_payload_digest_error` | formal-verification-report.md |
| ERR-008: PayloadTooLarge | kani | `kani::verify_payload_size_error` | formal-verification-report.md |
| ERR-009: UnexpectedEof | kani + miri | `kani::verify_eof_error` + `cargo-careful` | formal-verification-report.md |
| ERR-010: PostcardDecodeFailed | kani | `kani::verify_postcard_error` | formal-verification-report.md |
| ERR-011: UnsupportedArtifactVersion | kani | `kani::verify_artifact_version_error` | formal-verification-report.md |
| ERR-012: UnsupportedWorkflowVersion | kani | `kani::verify_workflow_version_error` | formal-verification-report.md |
| ERR-013: EmptyWorkflowName | kani | `kani::verify_workflow_name_error` | formal-verification-report.md |
| ERR-014: InvalidWorkflowName | kani | `kani::verify_workflow_name_error` | formal-verification-report.md |
| ERR-015: EmptyIr | kani | `kani::verify_ir_bytes_error` | formal-verification-report.md |
| ERR-016: IrDigestMismatch | kani | `kani::verify_ir_digest_error` | formal-verification-report.md |
| ERR-017: StorageKeyDigestMismatch | kani | `kani::verify_storage_key_error` | formal-verification-report.md |
| ERR-018: ZeroDigest | kani | `kani::verify_nonzero_digest_error` | formal-verification-report.md |
| ERR-019: InvalidGateCount | kani | `kani::verify_gate_count_error` | formal-verification-report.md |
| ERR-020: VerificationGateFailed | kani | `kani::verify_gate_failed_error` | formal-verification-report.md |
| ERR-021: MissingRequiredProofFlag | kani | `kani::verify_proof_flag_error` | formal-verification-report.md |
| ERR-022: InvalidWarningGate | kani | `kani::verify_warning_gate_error` | formal-verification-report.md |
| ERR-023: DuplicateCapability | kani | `kani::verify_duplicate_cap_error` | formal-verification-report.md |
| ERR-024: DuplicateSecret | kani | `kani::verify_duplicate_secret_error` | formal-verification-report.md |
| ERR-025: DuplicateActionId | kani | `kani::verify_duplicate_action_error` | formal-verification-report.md |
| ERR-026: BoundExceeded | kani | `kani::verify_bound_error` | formal-verification-report.md |

### Runtime Admission

| Clause | Layer | Checker | Evidence |
|--------|-------|---------|----------|
| PRE-026: admit_run checks artifact existence | kani | `kani::verify_admit_run` | formal-verification-report.md |
| PRE-027: capability coverage check | kani | `kani::verify_capability_check` | formal-verification-report.md |
| POST-004: RunAdmission returned on success | kani | `kani::verify_run_admission` | formal-verification-report.md |
| ERR-ADM-001: ArtifactNotFound | kani | `kani::verify_artifact_not_found` | formal-verification-report.md |
| ERR-ADM-002: CapabilityDenied | kani | `kani::verify_capability_denied` | formal-verification-report.md |
| ERR-ADM-003: ResourceCapacityExceeded | kani | `kani::verify_capacity_exceeded` | formal-verification-report.md |
| INV-001: no execution without accepted artifact | kani | `kani::verify_no_exec_without_artifact` | formal-verification-report.md |

### Property/Fuzz Testing

| Clause | Layer | Checker | Evidence |
|--------|-------|---------|----------|
| POST-001: roundtrip preservation | proptest | `cargo test -p vb_storage proptest_roundtrip` | proptest-report.md |
| PRE-001 to PRE-025: all preconditions | proptest | `cargo test -p vb_storage proptest_artifact_validation` | proptest-report.md |
| ERR-001 to ERR-026: all error variants | cargo-fuzz | `cargo fuzz run artifact_decode` | fuzz-report.md |
| envelope corruption | cargo-fuzz | `cargo fuzz run envelope_corrupt` | fuzz-report.md |

### Concurrency Testing

| Clause | Layer | Checker | Evidence |
|--------|-------|---------|----------|
| Journal append ordering | loom | `cargo loom --test journal_ordering` | loom-report.md |
| RunAccepted/RunAdmission atomicity | loom | `cargo loom --test admission_atomicity` | loom-report.md |

### Coverage/Mutation

| Clause | Layer | Checker | Evidence |
|--------|-------|---------|----------|
| All clauses | cargo-mutants | `cargo mutants --test vb_storage` | mutants-report.md |
| All clauses | cargo-llvm-cov | `cargo llvm-cov --test vb_storage` | coverage-report.md |

### Static Analysis

| Clause | Layer | Checker | Evidence |
|--------|-------|---------|----------|
| No unsafe in vb_storage | cargo-geiger | `cargo geiger -p vb_storage` | geiger-report.md |
| No unsafe in vb_runtime | cargo-geiger | `cargo geiger -p vb_runtime` | geiger-report.md |
| No clippy violations | clippy | `cargo clippy -p vb_storage -p vb_runtime` | clippy-report.md |

### Integration Tests

| Clause | Layer | Checker | Evidence |
|--------|-------|---------|----------|
| submit_artifact followed by admit_run | cargo-nextest | `cargo nextest run -p velvet_ballistics admission_evidence_integration` | integration-report.md |
| submit_direct rejected when artifact required | cargo-nextest | `cargo nextest run -p velvet_ballistics reject_raw_submit` | integration-report.md |

## Lean Scope

- **Theorem module**: `VelvetBallistics.ArtifactEnvelope`, `VelvetBallistics.Digest`, `VelvetBallistics.Verification`, `VelvetBallistics.Idempotency`, `VelvetBallistics.Capability`, `VelvetBallistics.Secret`, `VelvetBallistics.Storage`
- **Rust target**: `crates/vb_storage/src/admission.rs`, `crates/vb_runtime/src/admission.rs`
- **Abstraction relation**: Rust `AcceptedArtifactV1` ↔ Lean `Artifact` record; `blake3::hash` ↔ Lean `blake3` function; `VerificationProof` ↔ Lean `VerificationProof` record
- **Shell exclusions**: All I/O, storage persistence, clock access, frame allocation, capability/secret runtime checking, policy enforcement
- **Non-goals**: Fjall backend, external processes, async scheduling, generated workflow code

## Waivers

- **WAIVER-ADMISSION-001**: Runtime admission preconditions (capability coverage, secret presence, frame allocation, run capacity) require runtime shell access. Compensating: Kani bounded model check + integration tests.
- **WAIVER-ADMISSION-002**: Durability ordering and strict/journaled policy behavior require storage backend. Compensating: storage integration tests + cargo-fuzz for adversarial journal interleavings.
- **WAIVER-ADMISSION-003**: Input schema validation requires workflow IR input schema. Compensating: proptest + integration tests.
- **WAIVER-ADMISSION-004**: Third-party library functions (byteorder CRC32C, blake3). Compensating: library unit tests + integration tests + fuzzing.
