# vb-hs9m Codebase Map

## Bead: Observability and evidence packaging
**Bead ID:** vb-hs9m
**Focus:** Observability, tracing, evidence collection, artifact packaging, audit trails for BDD execution
**Source Checkout:** `/home/lewis/src/velvet-ballistics`
**Isolated Workspace:** `/home/lewis/src/vb-hs9m-workspace`

---

## 1. Observability Infrastructure

### 1.1 Trace Ring (`vb_runtime/src/trace.rs`)
- **Type:** Bounded SPSC ring buffer using `rtrb` crate
- **Core Struct:** `TraceRing` with capacity, dropped counter, history VecDeque
- **Core Enum:** `TraceEvent` variants:
  - `StepStarted`, `StepEnded`, `SlotWritten`, `ActionScheduled`, `ActionCompleted`, `ActionFailed`, `AskAnswered`, `RunSubmitted`, `RunFinished`, `RunFailed`, `RunCancelled`
- **Key Methods:**
  - `push(TraceEvent) -> bool` — returns false when ring full
  - `drain()`, `drain_into(limit, &mut Vec)`, `drain_for_run(RunId, limit)`
  - `snapshot_for_run(RunId, limit)` — non-destructive
  - `has_terminal_event_for_run(RunId) -> bool`
- **Tests:** 1077 lines of BDD-style tests including adversarial overflow scenarios
- **Risk Tags:** `persistence`, `concurrency` (SPSC ring buffer)

### 1.2 Trace IPC Layer (`vb_ipc::IpcTraceEvent`)
- **Location:** `crates/vb_ui/src/workflow/execution_details.rs`
- **Event Kinds:** `StepStarted`, `StepEnded`, `SlotWritten`, `ActionScheduled`, `ActionCompleted`, `ActionFailed`, `AskAnswered`, `RunSubmitted`, `RunFinished`, `RunFailed`, `RunCancelled`
- **Risk Tags:** `persistence`, `user-visible-behavior`

### 1.3 Journal Events (`vb_runtime/src/journal.rs`, `vb_storage/src/events.rs`)
- **Types:** `RuntimeJournalEvent`, `JournalEvent`
- **Pattern:** `postcard` serialization for durability
- **Risk Tags:** `persistence`, `concurrency`

---

## 2. Evidence Collection System

### 2.1 Evidence Bundle Module (`xtask/src/evidence/bundle.rs`)
- **Core Types:**
  - `EvidenceBundle` — top-level container with schema_version, executor_context, linked_bead_id, gates, source_test_mappings, release_artifacts
  - `ExecutorContext` — agent, timestamp, machine
  - `GateEvidence` — kind, gate_name, command, exit_code, log, status, why_failed
  - `SourceTestMapping` — source_path -> Vec<test_names>
  - `ReleaseGateArtifact` — name, path, digest, artifact_type
  - `ArtifactType` enum — Benchmark, Coverage, Mutation, SupplyChain, Miri, Clippy, Fmt
  - `EvidenceBundleFormat` — Yaml, Json, Postcard
- **Key Functions:**
  - `parse_bundle_schema_version(&str) -> Result<String, Error>` — validates major.minor format
  - `validate_bundle(&EvidenceBundle) -> Vec<Error>` — checks required fields
  - `write_bundle(&EvidenceBundle, &Path, EvidenceBundleFormat) -> Result<()>`
  - `read_bundle(&Path, EvidenceBundleFormat) -> Result<EvidenceBundle>`
  - `bundle_path(bead_id, format) -> PathBuf` — path is `.evidence/<bead-id>/bundle.<ext>`
- **Postcard Wire Format:** `EvidenceBundlePostcard`, `GateEvidencePostcard`, `GateStatusPostcard`
- **Risk Tags:** `persistence`, `dependency` (serde, postcard)

### 2.2 Gate Evidence Types (`xtask/src/evidence/tooling_and_gate_types.rs`)
- **Types:**
  - `GateStatus` — `Pass`, `Fail`, `Skipped { reason: String }`
  - `WhyFailed` — gate_name, hint, repair_command, variant, fixture_id, expected_gate
  - `FalsePassDiagnosticVariant` — Overlap, Secret
  - `XtaskCommandDiagnostic` — error_code, fixture_id, expected_gate, actual_status, variant
- **Error Variants:** GateTimeout, GateFailed, MissingEvidence, EvidenceWriteFailed, SubcommandNotFound, BeadDirectoryCreationFailed, YamlSerializationFailed, UpstreamMoonFailed, UpstreamJustFailed, SchemaVersionParseFailed, MissingRequiredField, BundleSerializationFailed
- **Risk Tags:** `dependency`

### 2.3 Evidence Persistence (`xtask/src/evidence/persistence.rs`)
- **Functions:**
  - `explain_failure(&GateEvidence) -> Option<WhyFailed>`
  - `failure_hint(gate_name) -> &'static str`
  - `failure_repair_command(gate_name) -> &'static str`
  - `validate_evidence_dir(dir, required_gates) -> Result<Vec<Error>>` — fail-closed on missing
  - `evidence_path(bead_id, gate_name) -> PathBuf` — `.evidence/<bead-id>/<gate-name>.yaml`
  - `write_evidence(&GateEvidence, &Path) -> Result<()>`
- **Risk Tags:** `persistence`

### 2.4 Profile Runner (`xtask/src/evidence/profile_runner.rs`)
- **Functions:**
  - `run_gate(gate, cmd, evidence_path) -> Result<GateEvidence>`
  - `run_profile(profile, bead_id, output_dir) -> Result<ProfileEvidence>`
  - `run_ai_release_profile(bead_id, output_dir) -> Result<ProfileEvidence>`
- **Risk Tags:** `dependency`

---

## 3. Artifact Packaging

### 3.1 Evidence Root Structure
- **Location:** `.evidence/` directory in workspace root
- **Pattern:** `.evidence/<bead-id>/<gate-name>.yaml`
- **Bundle Pattern:** `.evidence/<bead-id>/bundle.yaml|json|postcard`

### 3.2 Release Validation (`xtask/src/evidence/release_validation.rs`, `release_validators.rs`)
- **Types:** `UiReleaseEvidence`, `ReleaseProfileEvidence`, `ReleaseParityClaim`, `ReleaseBeadId`
- **Functions:**
  - `include_ui_gates_in_ai_release(bead_id) -> Result<ReleaseProfileEvidence>`
  - `check_redaction_artifacts(evidence, denylist) -> Result<RedactionEvidence>`
- **Risk Tags:** `auth-security` (secret denylist for redaction)

### 3.3 Release Rendering (`xtask/src/evidence/release_rendering.rs`)
- **Purpose:** Render release evidence to various formats
- **Risk Tags:** `persistence`, `dependency`

---

## 4. BDD/Acceptance Catalog

### 4.1 Scenario Structure (`crates/workspace_tests/src/acceptance_catalog.rs`)
- **Core Struct:** `Scenario` with fields:
  - `id`, `master_behavior`, `given`, `when`, `then`
  - `public_surface`, `fixture`, `expected_outcome`, `expected_error`
  - `durability_profile`, `related_bead`, `executable_evidence_target`
  - `deferred_follow_up_bead`
- **Validation Errors:** `EmptyCatalog`, `MissingGivenWhenThen`, `MissingExactAssertion`, `MissingEvidenceDisposition`, `ConflictingEvidenceDisposition`, `InvalidExecutableEvidenceTarget`, `InvalidDeferredFollowUpBead`, `PrivateSurface`, `SharedFixture`, `DuplicateScenarioId`
- **Functions:**
  - `catalog() -> &'static [Scenario]`
  - `validate_catalog(scenarios) -> Result<(), CatalogValidationError>`

### 4.2 BDD Test Files
- `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs` — 900+ lines with Given/When/Then comments
- `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs` — 532 lines, scenario catalog validation
- `crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs` — Recovery scenarios
- `crates/workspace_tests/tests/vb_qi37_12_state8_silent_discard_contract.rs` — Silent discard scenarios

### 4.3 Quality Test Loop Inventory (`crates/workspace_tests/src/quality/test_loop_inventory/`)
- `mod.rs`, `loop_pattern.rs`, `discover.rs`, `classify.rs`, `assignment.rs`
- `report_types.rs`, `report_render.rs`, `workspace.rs`, `scan.rs`
- `errors.rs`, `newtypes.rs`, `disposition_validate.rs`, `validated.rs`
- **Risk Tags:** `performance` (loop pattern analysis)

---

## 5. Audit Trail / Traceability

### 5.1 Traceability Matrix Files
- `.beads/vb-hs9m/delivery-scope.jsonl` — scope per file/API/dependency/contract/risk tag/verifier mode
- `.beads/vb-kyyf/proof-obligations.planned.jsonl` — traceability from vb-kyyf
- `velvet-ballistics-MASTER.md` — master behavior catalog

### 5.2 Evidence Index Pattern
- Evidence paths embedded in scenario structs: `.evidence/vb-kyyf/bdd-cross-run-determinism.md`
- Source-test mappings in `EvidenceBundle.source_test_mappings`

---

## 6. Verification Infrastructure

### 6.1 Kani Harnesses (Evidence)
- `xtask/tests/bundle_tests.rs` — bundle.rs proof harnesses (OBL-001 through OBL-008)
  - `parse_bundle_schema_version_non_panic`
  - `validate_bundle_correctness`
  - `write_bundle_non_panic`, `read_bundle_non_panic`
  - Round-trip identity properties (OBL-005, OBL-006, OBL-007)
- **Risk Tags:** `verification`, `unsafe-ub` (kani)

### 6.2 Formal Verification
- `verification/verus/run_frame_invariant.rs`
- `verification/verus/signals_invariant.rs`
- `kani/verify_idempotency_time_in_key.rs`
- `kani/verify_idempotency_random_in_key.rs`

---

## 7. Workspace Structure

### 7.1 Crates
- `crates/vb_core/` — workflow engine, validation, replay
- `crates/vb_runtime/` — runtime, trace ring, journal, action queue, admission
- `crates/vb_storage/` — events, recovery, persistence
- `crates/vb_validate/` — validation gates
- `crates/vb_yaml/` — YAML parsing, source maps
- `crates/vb_ui/` — UI workflow execution details
- `crates/vb_ui_snapshot/` — snapshot testing
- `crates/workspace_tests/` — integration tests, acceptance catalog, quality test loop inventory

### 7.2 xtask Evidence Module
- `xtask/src/evidence/bundle.rs` — evidence bundle container and serialization
- `xtask/src/evidence/tooling_and_gate_types.rs` — gate evidence types
- `xtask/src/evidence/persistence.rs` — file I/O for evidence
- `xtask/src/evidence/profile_runner.rs` — gate execution
- `xtask/src/evidence/release_model.rs`, `release_contract.rs`, `release_rendering.rs`, `release_validation.rs`, `release_validators.rs` — release evidence
- `xtask/src/evidence/raw_documents.rs`, `parsed_documents.rs`, `fixture_parsers.rs`, `negative_fixtures.rs` — fixture parsing
- `xtask/src/evidence/artifact_facts.rs`, `error_profile_domain.rs` — domain types

---

## 8. Risk Assessment

| Risk Tag | Description | Files |
|----------|-------------|-------|
| `persistence` | Evidence durability, journal storage, recovery | `trace.rs`, `events.rs`, `persistence.rs`, `bundle.rs` |
| `concurrency` | SPSC ring buffer, multi-threaded execution | `trace.rs`, `journal.rs`, `action_queue.rs` |
| `verification` | Kani harnesses, formal proofs | `bundle_tests.rs`, `*.rs` in `verification/` |
| `unsafe-ub` | Kani unbounded model checking | `kani_*.rs` |
| `dependency` | External crate dependencies (serde, postcard) | `bundle.rs`, `tooling_and_gate_types.rs` |
| `auth-security` | Secret redaction, negative fixtures | `tooling_and_gate_types.rs`, `release_validators.rs` |
| `user-visible-behavior` | Trace events, UI display | `execution_details.rs`, `vb_vt2f_*` tests |
| `performance` | Test loop inventory, BDD execution | `test_loop_inventory/*.rs` |

---

## 9. Unknowns / Blockers

- **UNKNOWN:** Whether `vb_runtime/src/trace.rs` TraceRing is used in production hot path or only test
- **UNKNOWN:** Whether there is a structured logging framework (tracing crate) beyond the manual TraceEvent enum
- **UNKNOWN:** Whether BDD runner has a dedicated executable command or is test-suite driven
- **DISCOVERY_BLOCKED:** Cannot determine full observability API surface without reading every runtime/engine file

---

## 10. Recommended Downstream Owners

- **Contract/Requirements:** `rust-contract` skill — for requirements extraction
- **Proof Planning:** `proof-planner` skill — for verifier lane selection
- **Test Planning:** `test-planner` skill — for BDD scenario test planning
- **Implementation:** `holzman-rust` skill — for safe Rust implementation
- **Formal Verification:** `formal-verifier` skill — for evidence execution
