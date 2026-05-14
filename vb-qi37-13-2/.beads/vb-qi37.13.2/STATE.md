# vb-qi37.13.2 STATE

## Current State
- **phase**: 1
- **bead**: vb-qi37.13.2
- **title**: cli: Implement diagnostic envelopes and exit codes
- **workspace**: /home/lewis/src/vb-qi37-13-2
- **current_state**: 15
- **go_skill_state**: 15 (landing)

## Exploration Findings

### 1. Envelope Types

**Kind enum variants (16 total)** in `vb-qi37-13-2-ws/crates/velvet_ballastics/src/cli_envelope.rs`:
| Kind | String Constant |
|------|----------------|
| `VerificationReport` | `"VerificationReport"` |
| `DiagnosticReport` | `"DiagnosticReport"` |
| `WorkflowExplanation` | `"WorkflowExplanation"` |
| `WorkflowGraph` | `"WorkflowGraph"` |
| `SimulationReport` | `"SimulationReport"` |
| `SubmitRunResult` | `"SubmitRunResult"` |
| `RunInspection` | `"RunInspection"` |
| `RunEvents` | `"RunEvents"` |
| `ReplayReport` | `"ReplayReport"` |
| `IncidentReport` | `"IncidentReport"` |
| `ActionList` | `"ActionList"` |
| `ActionDescription` | `"ActionDescription"` |
| `DoctorReport` | `"DoctorReport"` |
| `AiContextPacket` | `"AiContextPacket"` |
| `CliStatus` | `"CliStatus"` |
| `AgentContext` | `"AgentContext"` |

**Schema version**: `"velvet-ballistics/cli-output/v1"` (constant `SCHEMA_VERSION`)

**Envelope structure**: JSON object with `schema_version`, `kind`, `data` fields via `build_envelope()`.

### 2. Exit Codes

**CliExitCode enum** in `vb-qi37-13-2-ws/crates/velvet_ballastics/src/exit_code.rs`:
| Variant | Value | Description |
|---------|-------|-------------|
| `Success` | 0 | Operation completed successfully |
| `ValidationFailed` | 1 | Input validation or argument parsing failed |
| `VerificationFailed` | 2 | Workflow verification failed |
| `CompileFailed` | 3 | Workflow compilation or code generation failed |
| `RuntimeFailed` | 4 | Runtime execution or step evaluation failed |
| `StorageError` | 5 | Storage, journal, or persistence operation failed |
| `IpcError` | 6 | IPC server operation failed |
| `ActionPolicyError` | 7 | Action policy violation |
| `ReplayDivergence` | 8 | Replay divergence detected |
| `DomainError` | 9 | Domain-specific business logic rule violation |

Implements `From<CliExitCode> for ExitCode` via `#[repr(u8)]` discriminant.

### 3. Test Coverage

**cli_envelope.rs tests (8 tests)**:
- `test_schema_version_not_empty`
- `test_kind_as_str`
- `test_kind_from_str`
- `test_build_envelope_has_schema_version`
- `test_build_envelope_has_kind`
- `test_build_envelope_has_data`
- `test_serialize_with_version`
- `test_all_kind_variants`

**exit_code.rs tests (4 tests)**:
- `discriminant_values_match_spec` — verifies all 10 discriminants
- `from_cli_exit_code_to_exit_code` — conversion roundtrip
- `from_core_error_maps_to_runtime_failed`
- `from_journal_error_maps_to_storage_error`
- `all_variants_are_distinct` — no duplicate discriminants

### 4. Risks

| Risk | Location | Description |
|------|----------|-------------|
| `unwrap_or_default` in serialization | main.rs (lines 2190, 2515, 2587, etc.) | Silent error swallowing on JSON serialization |
| `expect()` on agent context serialize | main.rs:104 | Could produce raw panic |
| Dual crate ambiguity | `crates/velvet_ballastics` vs `vb-oaom/crates/velvet_ballastics` | Which is canonical CLI? |
| Exit code 9 not in original spec | `exit_code.rs` | Spec said 0-8, code has 0-9 |

### 5. File Locations

| File | Purpose |
|------|---------|
| `vb-qi37-13-2-ws/crates/velvet_ballastics/src/cli_envelope.rs` | Kind enum, envelope builder, SCHEMA_VERSION |
| `vb-qi37-13-2-ws/crates/velvet_ballastics/src/exit_code.rs` | CliExitCode enum 0-9 |
| `vb-qi37-13-2-ws/crates/vb_core/src/diagnostic.rs` | DiagnosticCode, Severity, Diagnostic |
| `vb-qi37-13-2-ws/crates/vb_ui_model/src/envelope.rs` | DiagnosticEntry, OutputEnvelope, EnvelopeKind |

## State History
- **State 13 (evidence-packaging + truth-serum)**: APPROVED — assurance-bundle.md, truth-serum-report.md, final-evidence-decision.md written. All 18 contract clauses covered. Zero-panic clippy gate PASS. black-hat APPROVED. contract-verification-review APPROVED.
- **State 14 (landing)**: Code pushed to origin/main at commit 272025ae50be
- **State 15 (landing complete)**: landing-report.md written

## Landing Evidence

- **Git Commit**: 272025ae50be7d3c9e1c38a0e0c719eaceec8a8
- **Git Push**: SUCCESS to origin/main
- **landing-report.md**: Written at `.beads/vb-qi37.13.2/landing-report.md`
- **Dolt Push**: Remote not configured for isolated workspace

## Next State
- **State 15 (landing)**: COMPLETE

(End of file - total 128 lines)