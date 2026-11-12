# Proof Repair Guide: vb-ahfl State 6 After State 5 Attempt 6

## Rejection Reason

State 6 proof-review is rejected because `VERUS-META-001`, `VERUS-BOUNDS-001`, `VERUS-REDACT-001`, and `VERUS-GRAPH-001` remain `PASS_LOCAL_MODEL` only. The required production-bound Verus harness files do not exist. `KANI-CANON-001` is resolved with raw SUCCESS evidence.

## Nearest Route

**State 5 rerun** to write production-bound Verus harness files. Do not route to State 10; State 10 completed its work (exposed production APIs and fixed missing include). State 5 owns proof writing, not proof planning or production implementation.

## Required Actions for State 5 Rerun

### 1. Write Production-Bound Verus Harness Files

Create the following files in `verification/verus/`:

#### `verification/verus/vb_ahfl_metadata_envelope_production.rs`
- Import `MetadataEnvelope`, `EnvelopeKind` from `crates/vb_ui_model/src/envelope/types.rs`
- Import `canonicalize_ui_artifact` from `crates/vb_ui_model/src/canonical.rs`
- Write Verus harness that proves `VERUS-META-001`: metadata completeness and schema/kind agreement
- Run: `verus verification/verus/vb_ahfl_metadata_envelope_production.rs`

#### `verification/verus/vb_ahfl_bounds_production.rs`
- Import `WorkflowGraphView`, `RunEventsView`, `VerificationReportView`, `IncidentReportView` from `crates/vb_ui_model/src/workflow.rs` and related modules
- Write Verus harness that proves `VERUS-BOUNDS-001`: bounded collections and truncation metadata
- Run: `verus verification/verus/vb_ahfl_bounds_production.rs`

#### `verification/verus/vb_ahfl_redaction_production.rs`
- Import `redact_secret_value`, `RedactedValueView`, `classify_secret_sensitivity` from `crates/vb_ui_model/src/redact.rs`
- Write Verus harness that proves `VERUS-REDACT-001`: fail-closed redaction projection
- Run: `verus verification/verus/vb_ahfl_redaction_production.rs`

#### `verification/verus/vb_ahfl_graph_events_production.rs`
- Import `WorkflowGraphView`, `WorkflowNodeView`, `WorkflowEdgeView`, `RunEventsView`, `RunEventView` from `crates/vb_ui_model/src/workflow.rs` and `crates/vb_ui_model/src/run.rs`
- Write Verus harness that proves `VERUS-GRAPH-001`: graph/event references and ordering
- Run: `verus verification/verus/vb_ahfl_graph_events_production.rs`

### 2. Update proof-writer-report.md and proof-evidence.md

- Document the new Verus harness files written
- Record the verus output (0 errors expected) for each file
- Classify each obligation as `PASS_PRODUCTION_BOUND` when verus passes

### 3. Do NOT Edit

- Production source code in `crates/vb_ui_model/src/`
- Kani harness `crates/vb_ui_model/tests/vb_ahfl_canonicalization_no_false_parity.rs`
- Abstract Verus model `verification/verus/vb_ahfl_ui_artifact_contract.rs`
- JSONL obligation files (unless adding new artifact entries)

## Expected Outcome

After State 5 rerun:
- `verification/verus/vb_ahfl_metadata_envelope_production.rs`: 0 errors
- `verification/verus/vb_ahfl_bounds_production.rs`: 0 errors
- `verification/verus/vb_ahfl_redaction_production.rs`: 0 errors
- `verification/verus/vb_ahfl_graph_events_production.rs`: 0 errors
- `KANI-CANON-001`: already resolved

Then State 6 re-review may approve if all critical/high obligations have raw evidence or approved waivers.
